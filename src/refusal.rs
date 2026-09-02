//! The one place refusal copy is written (`docs/design/refusal-copy.md`).
//!
//! Three producers used to write their own: the gated reason in `main.rs`, the `--explain` header,
//! and the nudge. That is how "not on the allowlist" survived in some outputs after being removed
//! from others. Everything routes through [`Refusal::render`] instead, so a wording change lands
//! everywhere or nowhere.
//!
//! The message is chosen by what safe-chains EMITS for this command on this harness, never by the
//! harness's name. A deny-harness we abstain on produces an ordinary prompt, so "blocked" would be
//! a lie there. Deriving copy from the emission is what stops it drifting when a harness's
//! behaviour changes, as Cursor's did when `allow` turned out to be ignored.

/// What happens to the command after safe-chains answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// We emit `deny` and the harness honours it. The command does not run.
    DidNotRun,
    /// We abstain, and the harness runs its own approval flow.
    GoesToHuman,
    /// No harness, or one whose behaviour we cannot name: the direct CLI and `--explain`.
    ///
    /// Vague about CONSEQUENCE, exact about CAUSE. A confident "this was blocked" that turns out
    /// false costs the reader's trust in the cause as well, which is the part they can act on.
    Unknown,
}

/// Why safe-chains did not approve the command.
#[derive(Debug, Clone)]
pub enum Cause {
    /// No researched entry for the resolved command name.
    NoEntry {
        /// The command name as RESOLVED, which is the single most useful fact and was absent
        /// entirely. When the refusal is a parse surprise, this word IS the explanation.
        command: String,
        /// The assignment that swallowed the command name, when that is what happened.
        swallowed_by: Option<String>,
    },
    /// A researched command reaching somewhere it may not, already phrased by `ReachReason`.
    Reach(String),
}

impl Cause {
    /// The `NoEntry` cause for a command line: the name the shell would RUN, and the assignment
    /// that swallowed it when one did.
    ///
    /// A leading run of `NAME=VALUE` words is an environment prefix; the first word after it is the
    /// program. That is the whole parse surprise: `RUSTDOCFLAGS=-D warnings cargo doc` runs
    /// `warnings`, and the message never said so, which made the refusal look arbitrary. The bug
    /// was otherwise silent — `bash: warnings: command not found` matches neither `^error` nor
    /// `^warning`, so the user's own grep swallowed it too.
    pub fn no_entry(command: &str) -> Self {
        let words = shell_words::split(command).unwrap_or_default();
        let mut last_assignment = None;
        for word in &words {
            if is_assignment(word) {
                last_assignment = Some(word.clone());
                continue;
            }
            return Cause::NoEntry {
                command: crate::parse::Token::from_raw(word.clone()).command_name().to_string(),
                swallowed_by: last_assignment,
            };
        }
        // Only assignments, or nothing parseable. There is no program name to name.
        Cause::NoEntry { command: command.trim().to_string(), swallowed_by: None }
    }
}

/// `NAME=VALUE` with a shell-legal name. Deliberately strict about the NAME: `-D=x` is not an
/// assignment, and treating it as one would hint at a parse surprise that is not there.
fn is_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else { return false };
    !name.is_empty()
        && name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// A refusal to render. See the module docs.
#[derive(Debug, Clone)]
pub struct Refusal {
    pub outcome: Outcome,
    pub cause: Cause,
}

const ISSUES: &str = "https://github.com/michaeldhopkins/safe-chains/issues";

/// The `--explain` header for a single command that did not auto-approve.
///
/// `--explain` runs against no harness, so it says nothing about what follows — the same
/// `Outcome::Unknown` discipline the builder applies. It also prints the resolved profile and the
/// refusing clause underneath, which is the detail a deliberate query can afford and an
/// interruption cannot.
pub const EXPLAIN_SINGLE: &str = "did not auto-approve this command. safe-chains approves \
                                  commands it has researched and has no opinion about the rest.";

/// The same, for a chain where only some segments were refused.
pub const EXPLAIN_MANY: &str = "safe-chains approves commands it has researched and has no \
                                opinion about the rest.";

/// Words that characterise the COMMAND rather than describing what happened.
///
/// "this command is not on the allowlist" reads as a verdict, and an agent's natural response to a
/// verdict is to hunt for a spelling that passes. The true statement is nearly the opposite:
/// safe-chains approves what it has researched and has no opinion about the rest.
///
/// `denied` is absent deliberately: it is the name of a `Verdict` variant and appears throughout the
/// code and the docs. This list governs AGENT-FACING copy, which is what `no_refusal_copy_
/// characterises_the_command` checks.
pub const AVOID: &[&str] = &[
    "not allowed",
    "rejected",
    "forbidden",
    "dangerous",
    "unsafe",
    "suspicious",
    "violation",
    "denied by policy",
    "allowlist",
];

impl Refusal {
    /// The agent-facing message.
    ///
    /// Leads with the resolved name and the outcome. If a harness truncates `additionalContext`,
    /// the sentence that survives has to be the one carrying the fact and the consequence, not the
    /// explanation of what safe-chains is.
    pub fn render(&self) -> String {
        let mut out = String::new();
        match &self.cause {
            Cause::NoEntry { command, .. } => {
                out.push_str(&match self.outcome {
                    Outcome::DidNotRun => format!(
                        "safe-chains did not approve this, and the command did not run. \
                         safe-chains has no entry for the command `{command}`."
                    ),
                    Outcome::GoesToHuman => format!(
                        "safe-chains has no entry for the command `{command}`, so it did not \
                         auto-approve this."
                    ),
                    Outcome::Unknown => format!(
                        "safe-chains has no entry for the command `{command}`, so it did not \
                         approve it."
                    ),
                });
                out.push(' ');
                out.push_str(self.not_a_rating());
            }
            Cause::Reach(why) => {
                out.push_str(&match self.outcome {
                    Outcome::DidNotRun => {
                        format!("safe-chains did not approve this, and the command did not run. {why}.")
                    }
                    Outcome::GoesToHuman => {
                        format!("safe-chains did not auto-approve this, so please confirm. {why}.")
                    }
                    Outcome::Unknown => format!("safe-chains did not approve this. {why}."),
                });
            }
        }

        if let Some(hint) = self.parse_surprise() {
            out.push(' ');
            out.push_str(&hint);
        }

        if let Outcome::GoesToHuman = self.outcome {
            out.push_str(" The normal approval prompt follows.");
        }

        if let Cause::NoEntry { command, .. } = &self.cause {
            out.push_str(&format!(
                " If `{command}` is a real command that should be approved, please open an issue: \
                 {ISSUES}"
            ));
        }
        out
    }

    /// Said once, plainly. An agent that reads a refusal as a verdict goes looking for a spelling
    /// that passes, so the copy has to close that path rather than leave it open.
    fn not_a_rating(&self) -> &'static str {
        match self.outcome {
            Outcome::Unknown => {
                "That is not a rating of the command. safe-chains approves commands it has \
                 researched. For anything else it gives no answer, and the tool that ran \
                 safe-chains decides what to do by its own default. Rewriting the command to get \
                 it approved is not the fix."
            }
            _ => {
                "That is not a rating of the command. safe-chains approves commands it has \
                 researched and has no opinion about the rest. Rewriting the command to get it \
                 approved is not the fix."
            }
        }
    }

    /// The extra sentence for `RUSTDOCFLAGS=-D warnings cargo doc`, where the unquoted assignment
    /// makes `warnings` the command NAME.
    ///
    /// Emitted only when the resolved name is an unknown bare word AND an assignment prefix is
    /// present. A hint that is wrong half the time is worse than none, because it teaches the
    /// reader to skip the explanation.
    fn parse_surprise(&self) -> Option<String> {
        let Cause::NoEntry { command, swallowed_by: Some(assignment) } = &self.cause else {
            return None;
        };
        let value = assignment.split_once('=').map(|(_, v)| v).unwrap_or_default();
        Some(format!(
            "The command name here is `{command}`. It comes after the `{assignment}` assignment, \
             so the shell reads it as the program to run. If you meant `{value} {command}` as one \
             value, it needs quotes."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_entry(outcome: Outcome) -> Refusal {
        Refusal {
            outcome,
            cause: Cause::NoEntry { command: "warnings".into(), swallowed_by: None },
        }
    }

    /// The copy follows the EMISSION. This is the rule the whole module exists for: a deny-harness
    /// we ABSTAIN on prompts a human, so "did not run" would be false there.
    #[test]
    fn the_wording_follows_the_outcome_not_the_harness() {
        let blocked = no_entry(Outcome::DidNotRun).render();
        assert!(blocked.contains("did not run"), "{blocked}");
        assert!(!blocked.contains("approval prompt"), "a blocked command asks nobody: {blocked}");

        let asked = no_entry(Outcome::GoesToHuman).render();
        assert!(asked.contains("approval prompt follows"), "{asked}");
        assert!(!asked.contains("did not run"), "an abstain did not stop anything: {asked}");

        // Unknown harness: exact about cause, silent about consequence.
        let unknown = no_entry(Outcome::Unknown).render();
        assert!(unknown.contains("no entry for the command"), "{unknown}");
        assert!(!unknown.contains("did not run"), "we cannot know that: {unknown}");
        assert!(!unknown.contains("approval prompt"), "we cannot know that either: {unknown}");
    }

    /// The resolved name is the single most useful fact, and it was absent from every message.
    #[test]
    fn the_resolved_command_name_is_always_named() {
        for outcome in [Outcome::DidNotRun, Outcome::GoesToHuman, Outcome::Unknown] {
            let text = no_entry(outcome).render();
            assert!(text.contains("`warnings`"), "{outcome:?} did not name the command: {text}");
        }
    }

    #[test]
    fn no_message_characterises_the_command() {
        let mut texts = vec![
            no_entry(Outcome::DidNotRun).render(),
            no_entry(Outcome::GoesToHuman).render(),
            no_entry(Outcome::Unknown).render(),
        ];
        texts.push(
            Refusal { outcome: Outcome::GoesToHuman, cause: Cause::Reach("it reads `~/.ssh/id_rsa`".into()) }
                .render(),
        );
        for text in &texts {
            for word in AVOID {
                assert!(!text.to_lowercase().contains(word), "`{word}` appears in: {text}");
            }
            assert!(!text.contains('—'), "em dash in agent-facing copy: {text}");
            assert!(!text.contains(';'), "semicolon in agent-facing copy: {text}");
        }
    }

    /// EVERY producer of agent-facing refusal copy, not just this module's.
    ///
    /// The spec's stated partial-implementation risk: "a string check that only scans `main.rs`
    /// will pass while `ReachReason` still says blocked. Enumerate the producers, not the files you
    /// remember." Three of them exist — the gated reason, the `--explain` header, and the reach
    /// nudge — and fixing one at a time is how "not on the allowlist" survived in some outputs
    /// after being removed from others.
    ///
    /// This reaches them through their PUBLIC entry points rather than by grepping source, so a
    /// producer that changes shape is still covered and a new one is not silently missed.
    #[test]
    fn every_refusal_producer_obeys_the_vocabulary() {
        let mut texts: Vec<String> = Vec::new();

        // Producer 1: the builder, in all three outcomes and both causes.
        for outcome in [Outcome::DidNotRun, Outcome::GoesToHuman, Outcome::Unknown] {
            texts.push(no_entry(outcome).render());
            texts.push(
                Refusal { outcome, cause: Cause::Reach("it reads `~/.ssh/id_rsa`".into()) }.render(),
            );
        }

        // Producer 2: the `--explain` header, for one command and for a chain.
        texts.push(crate::cst::explain("frobnicate --wibble").render());
        texts.push(crate::cst::explain("ls && frobnicate --wibble").render());

        // Producer 3: the reach nudge, which is where the spec expected a stale "blocked" to hide.
        for (path, reason) in [("~/.ssh/id_rsa", "read"), ("/etc/sudoers", "write")] {
            let _ = reason;
            if let Some((p, why)) = crate::workspace_overreach(&format!("cat {path}")) {
                texts.push(why.message(&p));
            }
        }

        assert!(texts.len() >= 8, "only {} producers probed — the sweep shrank", texts.len());
        for text in &texts {
            for word in AVOID {
                assert!(
                    !text.to_lowercase().contains(word),
                    "`{word}` appears in agent-facing copy:\n{text}"
                );
            }
        }
    }

    /// The reported case, end to end: `RUSTDOCFLAGS=-D warnings cargo doc` runs `warnings`.
    #[test]
    fn no_entry_names_the_program_the_shell_would_run() {
        let c = Cause::no_entry("RUSTDOCFLAGS=-D warnings cargo doc --no-deps");
        match &c {
            Cause::NoEntry { command, swallowed_by } => {
                assert_eq!(command, "warnings", "the assignment swallowed the name");
                assert_eq!(swallowed_by.as_deref(), Some("RUSTDOCFLAGS=-D"));
            }
            other => panic!("expected NoEntry, got {other:?}"),
        }

        // No assignment: the first word is the program and there is no surprise to explain.
        match Cause::no_entry("frobnicate --wibble") {
            Cause::NoEntry { command, swallowed_by } => {
                assert_eq!(command, "frobnicate");
                assert_eq!(swallowed_by, None);
            }
            other => panic!("expected NoEntry, got {other:?}"),
        }

        // A path is reported by its command name, as everywhere else.
        match Cause::no_entry("/usr/local/bin/frobnicate") {
            Cause::NoEntry { command, .. } => assert_eq!(command, "frobnicate"),
            other => panic!("expected NoEntry, got {other:?}"),
        }

        // `-D=x` is not an assignment. Treating it as one would hint at a parse surprise that is
        // not there, and a hint that is wrong teaches the reader to skip the explanation.
        match Cause::no_entry("-D=x frobnicate") {
            Cause::NoEntry { command, swallowed_by } => {
                assert_eq!(command, "-D=x", "a flag is not an env prefix");
                assert_eq!(swallowed_by, None);
            }
            other => panic!("expected NoEntry, got {other:?}"),
        }
    }

    /// The hint fires on the shape that produced this spec, and on nothing else.
    #[test]
    fn the_parse_surprise_hint_is_conditional() {
        let plain = no_entry(Outcome::GoesToHuman).render();
        assert!(!plain.contains("assignment"), "hinted at a parse surprise with no assignment: {plain}");

        let surprised = Refusal {
            outcome: Outcome::GoesToHuman,
            cause: Cause::NoEntry {
                command: "warnings".into(),
                swallowed_by: Some("RUSTDOCFLAGS=-D".into()),
            },
        }
        .render();
        assert!(surprised.contains("RUSTDOCFLAGS=-D"), "{surprised}");
        assert!(surprised.contains("it needs quotes"), "{surprised}");
        // The suggestion has to name the value the user meant, or it explains nothing.
        assert!(surprised.contains("`-D warnings`"), "{surprised}");
    }
}
