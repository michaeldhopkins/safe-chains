//! Coexistence bridge (v1.4 §4.5; annex `…-engine` §4). Runs the profile resolver
//! alongside the legacy classifier behind the `SAFE_CHAINS_ENGINE` selector, and
//! projects a resolved profile back to a legacy [`Verdict`] so the existing ceiling
//! gate (`main::run_cli`) keeps working unchanged.
//!
//! Default is `legacy` — the engine does not run and behavior is byte-identical. Only
//! commands with a resolver (`resolve::resolve` → `Some`) are ever computed by the
//! engine; everything else stays on the legacy path.

use std::sync::LazyLock;

use super::authoring::default_levels;
use super::facet::Profile;
use super::resolve;
use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

/// The rollout selector (annex `…-engine` §5).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Current classifier is authoritative; the engine does not run (default).
    Legacy,
    /// Legacy is authoritative; the engine runs alongside and divergences are reported.
    Shadow,
    /// The engine is authoritative for commands it can resolve; legacy for the rest.
    New,
}

static MODE: LazyLock<Mode> = LazyLock::new(|| match std::env::var("SAFE_CHAINS_ENGINE") {
    Ok(v) if v == "shadow" => Mode::Shadow,
    Ok(v) if v == "new" => Mode::New,
    _ => Mode::Legacy,
});

/// The active engine mode (read once from `SAFE_CHAINS_ENGINE`).
pub fn mode() -> Mode {
    *MODE
}

/// The engine's verdict for a command whose resolver exists, or `None` if it has none
/// (the caller keeps the legacy verdict).
pub fn engine_verdict(tokens: &[Token]) -> Option<Verdict> {
    resolve::resolve(tokens).map(|p| project(&p))
}

/// Project a resolved profile to a legacy [`Verdict`]: the **lowest** authored level
/// that admits it, mapped back to its legacy [`SafetyLevel`]; `Denied` if no authored
/// level admits it (above the ladder → worst-case, §0). `default_levels()` is the
/// ascending local chain (inert ⊂ read-local ⊂ write-local), so the first match is the
/// minimum.
pub fn project(profile: &Profile) -> Verdict {
    for level in default_levels() {
        if level.admits(profile) {
            return Verdict::Allowed(to_legacy(&level.name));
        }
    }
    Verdict::Denied
}

fn to_legacy(level_name: &str) -> SafetyLevel {
    match level_name {
        "inert" => SafetyLevel::Inert,
        "read-local" => SafetyLevel::SafeRead,
        _ => SafetyLevel::SafeWrite, // write-local, developer, …
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::facet::*;

    fn toks(parts: &[&str]) -> Vec<Token> {
        parts.iter().map(|p| Token::from_test(p)).collect()
    }

    #[test]
    fn project_maps_profiles_to_the_lowest_admitting_level() {
        // echo — inert
        let echo = Profile::of(vec![{
            let mut c = Capability::new(Operation::Observe);
            c.disclosure.audience = DisclosureAudience::LocalProcess;
            c
        }]);
        assert_eq!(project(&echo), Verdict::Allowed(SafetyLevel::Inert));

        // cat ./notes — read-local
        let read = Profile::of(vec![{
            let mut c = Capability::new(Operation::Observe);
            c.locus.local = LocalLocus::Worktree;
            c.disclosure.audience = DisclosureAudience::LocalProcess;
            c
        }]);
        assert_eq!(project(&read), Verdict::Allowed(SafetyLevel::SafeRead));

        // cat ~/.ssh/id_rsa — above the authored ladder → Denied
        let home = Profile::of(vec![{
            let mut c = Capability::new(Operation::Observe);
            c.locus.local = LocalLocus::User;
            c.disclosure.audience = DisclosureAudience::LocalProcess;
            c
        }]);
        assert_eq!(project(&home), Verdict::Denied);
    }

    #[test]
    fn engine_verdict_is_none_for_unresearched_commands() {
        assert!(engine_verdict(&toks(&["rm", "-rf", "/"])).is_none());
        assert_eq!(engine_verdict(&toks(&["echo", "hi"])), Some(Verdict::Allowed(SafetyLevel::Inert)));
        assert_eq!(
            engine_verdict(&toks(&["cat", "./notes.md"])),
            Some(Verdict::Allowed(SafetyLevel::SafeRead)),
        );
        assert_eq!(engine_verdict(&toks(&["cat", "~/.ssh/id_rsa"])), Some(Verdict::Denied));
    }

    /// The engine may deny what legacy allowed (intended tightening) or classify higher,
    /// but must **never allow what legacy denied**, nor classify lower.
    fn not_looser(legacy: Verdict, engine: Verdict) -> bool {
        match (legacy, engine) {
            (_, Verdict::Denied) => true,
            (Verdict::Denied, Verdict::Allowed(_)) => false,
            (Verdict::Allowed(l), Verdict::Allowed(e)) => e >= l,
        }
    }

    /// The rollout safety gate on hand-picked forms — including the ones the wiring and
    /// the review flushed (unrecognized/dangerous flags, and pattern-less grep, which
    /// legacy denies as a usage error).
    #[test]
    fn the_engine_is_never_looser_than_legacy() {
        let cases = [
            "echo hi", "echo", "cat ./notes.md", "cat -n ./notes.md", "cat ~/.ssh/id_rsa",
            "cat /etc/hosts", "cat a.txt b.txt", "grep foo src/main.rs", "grep -r foo src/",
            "grep -r foo ~", "grep foo bar.txt",
            // unrecognized / dangerous flags must worst-case
            "cat --unknownflag ./x", "cat -Z ./x", "grep -P foo file",
            "grep --perl-regexp foo file", "grep --wat foo file",
            // pattern-less grep (C1): legacy denies as a usage error, engine must too
            "grep", "grep -r", "grep -i", "grep -e foo", "grep -f patterns.txt",
        ];
        for cmd in cases {
            let legacy = crate::command_verdict(cmd);
            let t = toks(&cmd.split_whitespace().collect::<Vec<_>>());
            let Some(engine) = engine_verdict(&t) else { continue };
            assert!(
                not_looser(legacy, engine),
                "engine LOOSER than legacy for `{cmd}`: legacy {legacy}, engine {engine}",
            );
        }
    }

    /// The specified §5 corpus-diff harness (the test C1 slipped past): run **every**
    /// command's real `examples_safe`/`examples_denied` through both paths and assert the
    /// engine is never looser — not a hand-picked list. Only simple (single-command)
    /// examples are comparable at the leaf; chains/redirects/substitutions are the CST's
    /// job, not this leaf's.
    #[test]
    fn the_engine_is_never_looser_over_the_example_corpus() {
        for (name, safe, denied) in crate::registry::corpus_examples() {
            for ex in safe.iter().chain(denied.iter()) {
                if ex.contains(['|', '>', '<', '&', ';', '$', '`', '(', '\n']) {
                    continue; // not a bare single command — skip the leaf comparison
                }
                let t = toks(&ex.split_whitespace().collect::<Vec<_>>());
                let Some(engine) = engine_verdict(&t) else { continue };
                let legacy = crate::command_verdict(ex);
                assert!(
                    not_looser(legacy, engine),
                    "engine LOOSER than legacy for `{ex}` ({name}): legacy {legacy}, engine {engine}",
                );
            }
        }
    }
}
