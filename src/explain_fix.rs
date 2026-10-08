//! The last thing `--explain` says about a refused command: what would change the answer.
//!
//! There are three answers, and saying which one applies is the point. An agent told only "no
//! opinion" read safe-chains' source code for most of an hour to find out what it needed.
//!
//! - **Add an entry**: safe-chains has no entry for the command. `safe-chains config add` writes one.
//! - **Grant a path**: a known command reached a path outside the places approved by default, and a
//!   grant would change that. It is offered only after asking the classifier with the grant in place,
//!   so it is never advice that changes nothing.
//! - **Nothing in the config**: a built-in rule refused an option, a subcommand or a protected path.
//!   No entry or grant changes it, and a person has to approve the command when it runs.
//!
//! One more case is not a rule at all: the level in force is stricter than the default, and the
//! default approves the command. Saying "a built-in rule refused it" there would be false.

use crate::userconfig::{Access, Step, step};
use crate::{ReachReason, sanitize_display, suggest};

/// What would change the verdict on one refused command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fix {
    AddEntry { names: Vec<String>, command: String },
    Grant { path: String, access: Access },
    RaiseLevel,
    Nothing(Because),
}

/// Why nothing in the config can change the verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Because {
    /// A known command, refused for an option, a subcommand or the way it is used.
    BuiltInRule {
        names: Vec<String>,
    },
    /// A path a built-in rule protects, with the reason `ReachReason` gives.
    ProtectedPath {
        path: String,
        reason: ReachReason,
    },
    /// A path built by an interpolation, which names nowhere in particular.
    Unconfined {
        path: String,
    },
    /// A path too wide to grant in one rule (`/`, the home folder), or one with a wildcard.
    TooWide {
        path: String,
    },
    /// A path that holds settings or programs that run later (agents' settings, startup files, PATH).
    RunsLater {
        path: String,
    },
    /// A command that runs an AI coding agent.
    RunsAnAgent {
        name: String,
    },
    Unparseable,
}

/// The fix for one refused command (a segment of a chain, or the whole command).
///
/// `approved` answers "would this command be approved?" the way the caller's verdict does, at the
/// caller's level. It is asked again with a hypothetical grant in place, and with the default level.
pub fn fix_for(command: &str, approved: &dyn Fn(&str) -> bool, approved_by_default: bool) -> Fix {
    if crate::cst::parse(command).is_none() {
        return Fix::Nothing(Because::Unparseable);
    }
    if approved_by_default {
        return Fix::RaiseLevel;
    }
    match suggest::analyze(command) {
        suggest::Outcome::Unparseable => return Fix::Nothing(Because::Unparseable),
        suggest::Outcome::Generated { entries, .. } => {
            let names: Vec<String> = entries.into_iter().map(|e| e.name).collect();
            if let Some(name) = names.iter().find(|n| crate::userconfig::runs_an_agent(n)) {
                return Fix::Nothing(Because::RunsAnAgent { name: name.clone() });
            }
            return Fix::AddEntry { names, command: command.trim().to_string() };
        }
        suggest::Outcome::AlreadyAllowed | suggest::Outcome::RecognizedButDenied { .. } => {}
    }
    let names = command_names(command);
    let Some((path, reason)) = crate::workspace_overreach(command) else {
        return Fix::Nothing(Because::BuiltInRule { names });
    };
    match reason {
        ReachReason::OutsideWorkspace | ReachReason::ForeignTemp => {
            let resolved = crate::pathctx::resolve(&path).into_owned();
            let candidates = [folder_of(&resolved), Some(resolved.clone())];
            let grantable: Vec<String> = candidates
                .into_iter()
                .flatten()
                .filter_map(|p| crate::userconfig::grantable_path(&p).ok())
                .collect();
            if grantable.is_empty() {
                return match crate::userconfig::grantable_path(&resolved) {
                    Err(crate::userconfig::EditError::TooBroad(_) | crate::userconfig::EditError::Glob(_)) => {
                        Fix::Nothing(Because::TooWide { path })
                    }
                    Err(crate::userconfig::EditError::Protected(_, _)) => Fix::Nothing(Because::RunsLater { path }),
                    _ => Fix::Nothing(Because::BuiltInRule { names }),
                };
            }
            grantable
                .into_iter()
                .find_map(|p| grant_that_works(command, &p, approved).map(|access| Fix::Grant { path: p, access }))
                .unwrap_or(Fix::Nothing(Because::BuiltInRule { names }))
        }
        ReachReason::Unconfined => Fix::Nothing(Because::Unconfined { path }),
        reason => Fix::Nothing(Because::ProtectedPath { path, reason }),
    }
}

/// The folder holding `path`, when `path` looks like a file (its last part has an extension). A
/// grant is offered on the folder first, so writing a second file there needs no second grant.
fn folder_of(path: &str) -> Option<String> {
    let (folder, last) = path.trim_end_matches('/').rsplit_once('/')?;
    let looks_like_a_file = last.rsplit_once('.').is_some_and(|(stem, ext)| !stem.is_empty() && !ext.is_empty());
    (looks_like_a_file && !folder.is_empty()).then(|| folder.to_string())
}

/// The narrowest grant on `path` that would get `command` approved, if any would.
fn grant_that_works(command: &str, path: &str, approved: &dyn Fn(&str) -> bool) -> Option<Access> {
    [Access::Read, Access::Write, Access::ReadWrite].into_iter().find(|access| {
        let (read, write) = (matches!(access, Access::Read | Access::ReadWrite), matches!(access, Access::Write | Access::ReadWrite));
        crate::engine::resolve::regions::hypothetical::with_hypothetical_grant(path, read, write, || approved(command))
    })
}

fn command_names(command: &str) -> Vec<String> {
    let mut names: Vec<String> = crate::cst::explain(command).segments.iter().filter_map(|s| s.culprit.clone()).collect();
    if names.is_empty()
        && let suggest::Outcome::RecognizedButDenied { names: known } = suggest::analyze(command)
    {
        names = known;
    }
    names
}

const PERSON_APPROVES: &str = "safe-chains never approves a change to its own config, so a person approves that step.";

impl Fix {
    /// The fix as `--explain` prints it. Command-derived text is neutralized, since this is read by
    /// the agent that wrote the command.
    pub fn render(&self) -> String {
        match self {
            Fix::AddEntry { names, command } => format!(
                "Fix: add an entry for {}:\n  {}\nThe entry approves the command used the way it is \
                 here, with the same subcommand and flags. {PERSON_APPROVES}\n",
                code_list(names),
                step(Step::Add(command))
            ),
            Fix::Grant { path, access } => format!(
                "Fix: grant {} access to `{}`:\n  {}\n{PERSON_APPROVES}\n",
                access.faces(),
                sanitize_display(path),
                step(Step::Grant(path, *access))
            ),
            Fix::RaiseLevel => "Fix: none in the rules. safe-chains approves this at its default level, and the \
                                level in force is stricter. The `level` setting in ~/.config/safe-chains.toml, or \
                                the --level you passed, decides that.\n"
                .to_string(),
            Fix::Nothing(because) => {
                let by_hand = matches!(
                    because,
                    Because::ProtectedPath { reason: ReachReason::Credential | ReachReason::RawDevice, .. } | Because::RunsLater { .. }
                );
                let (lead, close) = if by_hand {
                    ("Fix: none that safe-chains will make.", "Otherwise a person approves the command each time it runs.")
                } else {
                    ("Fix: none in safe-chains' config.", "A person approves the command each time it runs.")
                };
                format!("{lead} {} {close}\n", because.render())
            }
        }
    }
}

impl Because {
    fn render(&self) -> String {
        match self {
            Because::BuiltInRule { names } if names.is_empty() => {
                "A built-in rule refused an option or a subcommand used here, and no entry or grant can change that.".to_string()
            }
            Because::BuiltInRule { names } => format!(
                "{} built in, and a built-in rule refused an option or a subcommand used here. No entry or grant \
                 can change that.",
                code_list(names) + if names.len() == 1 { " is" } else { " are" }
            ),
            Because::ProtectedPath { path, reason } => {
                let shown = sanitize_display(path);
                match reason {
                    ReachReason::Credential => format!(
                        "`{shown}` holds credentials, so neither `safe-chains config` nor its MCP tools will grant it. \
                         A grant you write into ~/.config/safe-chains.toml by hand would. That is the user's decision, not an agent's."
                    ),
                    ReachReason::RawDevice => format!(
                        "`{shown}` is a raw device, so neither `safe-chains config` nor its MCP tools will grant it. \
                         A read grant you write into ~/.config/safe-chains.toml by hand would allow reading it. That is the user's decision, not an agent's."
                    ),
                    _ => format!("A built-in rule protects `{shown}`, and no grant can cover it."),
                }
            }
            Because::Unconfined { path } => format!(
                "The path `{}` is built by an interpolation, so it could name anywhere, and a grant cannot cover anywhere.",
                sanitize_display(path)
            ),
            Because::RunsLater { path } => format!(
                "`{}` holds settings or programs that run later without anyone asking, so neither `safe-chains \
                 config` nor its MCP tools will grant it. A grant you write into ~/.config/safe-chains.toml by hand \
                 would. That is the user's decision, not an agent's.",
                sanitize_display(path)
            ),
            Because::TooWide { path } => format!(
                "The command reaches `{}`, which is too wide to grant in one rule, and a narrower grant would not \
                 cover it.",
                sanitize_display(path)
            ),
            Because::RunsAnAgent { name } => format!(
                "`{}` runs an AI coding agent, which can change its own approval settings, so safe-chains will \
                 not add an entry that approves it.",
                sanitize_display(name)
            ),
            Because::Unparseable => "safe-chains could not parse the command, so no entry or grant applies. Check its quoting.".to_string(),
        }
    }
}

fn code_list(names: &[String]) -> String {
    let shown: Vec<String> = names.iter().map(|n| format!("`{}`", sanitize_display(n))).collect();
    match shown.as_slice() {
        [] => "the command".to_string(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

#[cfg(test)]
mod tests;
