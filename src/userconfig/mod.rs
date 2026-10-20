//! `safe-chains config`: read and change `~/.config/safe-chains.toml` without opening it.
//!
//! The file decides what safe-chains approves, so changing it by hand is where things go wrong: a
//! hand-written grant has let `head ~/.ssh/id_rsa` through. These operations change
//! one rule at a time, keep the rest of the file exactly as it was, refuse anything a built-in rule
//! protects, and refuse to write a file safe-chains could not load.
//!
//! Nothing here approves itself. The classifier treats `safe-chains config add`, `remove` and
//! `grant` like any other write (see `commands/tools/safe-chains.toml`), so a person approves each
//! one, and the MCP server marks the same operations destructive.
//!
//! Each operation reads the file afresh. The hook's registry and grants are loaded once per process,
//! which is right for a process that classifies one command and exits, and wrong for a long-lived
//! MCP server, so the server runs these through the CLI rather than calling them in-process.

mod edit;
mod guard;
mod list;
mod surgery;

use std::fs;
use std::path::{Path, PathBuf};

pub use edit::Access;
pub use guard::Protected;
pub use list::list;

/// Where the user config lives, written the way every message shows it.
pub const SHOWN_PATH: &str = "~/.config/safe-chains.toml";

/// What a `config` operation could not do. Every message says what was wrong, what would work
/// instead, and that nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    NoHome,
    Unreadable(String),
    DoesNotLoad(String),
    AlreadyApproved,
    Unparseable,
    BuiltIn(Vec<String>),
    RunsAnAgent(String),
    AlreadyHasEntry(String),
    UnknownLevel(String),
    NoAccess,
    NotAbsolute(String),
    Glob(String),
    TooBroad(String),
    Protected(String, Protected),
    NotFound { kind: &'static str, key: String, present: Vec<String> },
    WriteFailed(String),
    CannotEditCleanly,
    OddName(String),
    PathSpelled(String),
    SubAfterFlags(String, String),
    LevelInCommand,
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let unchanged = "Nothing was changed.";
        match self {
            EditError::NoHome => write!(f, "HOME is not set, so safe-chains cannot find {SHOWN_PATH}. {unchanged}"),
            EditError::Unreadable(why) => write!(f, "safe-chains could not read {SHOWN_PATH}: {why}. {unchanged}"),
            EditError::DoesNotLoad(why) => write!(
                f,
                "{SHOWN_PATH} does not load ({why}), so safe-chains is ignoring the rules in it. Fix the \
                 file by hand first. {unchanged}"
            ),
            EditError::AlreadyApproved => write!(f, "safe-chains already approves this command, so it needs no entry. {unchanged}"),
            EditError::Unparseable => write!(f, "safe-chains could not parse this command. Check its quoting. {unchanged}"),
            EditError::BuiltIn(names) => write!(
                f,
                "{} built into safe-chains, and an entry cannot replace a built-in rule. If safe-chains \
                 did not approve this, a built-in rule refused one of its options or paths, and a person \
                 has to approve it when it runs. {unchanged}",
                quoted_list(names, "is", "are")
            ),
            EditError::RunsAnAgent(name) => write!(
                f,
                "`{name}` runs an AI coding agent, which can change its own approval settings. safe-chains \
                 will not add an entry that approves it. {unchanged}"
            ),
            EditError::AlreadyHasEntry(name) => write!(
                f,
                "{SHOWN_PATH} already has an entry for `{name}`. To replace it, remove it first: {}. \
                 {unchanged}",
                step(Step::RemoveCommand(name))
            ),
            EditError::UnknownLevel(level) => {
                write!(f, "`{level}` is not a level an entry can have. Use inert, safe-read or safe-write. {unchanged}")
            }
            EditError::NoAccess => write!(f, "Say what to grant: --read, --write, or both. {unchanged}"),
            EditError::NotAbsolute(path) => {
                write!(f, "`{path}` is not a path safe-chains can grant. Give a full path or one starting with ~/. {unchanged}")
            }
            EditError::Glob(path) => write!(f, "`{path}` contains a wildcard. Grant a folder by its plain path. {unchanged}"),
            EditError::TooBroad(path) => {
                write!(f, "`{path}` covers too much to grant in one rule. Grant the folder you need inside it. {unchanged}")
            }
            EditError::Protected(path, why) => write!(f, "`{path}` {}. {unchanged}", why.explain()),
            EditError::NotFound { kind, key, present } if present.is_empty() => {
                write!(f, "{SHOWN_PATH} has no {kind} for `{key}`, and no {} at all. {unchanged}", plural(kind))
            }
            EditError::NotFound { kind, key, present } => write!(
                f,
                "{SHOWN_PATH} has no {kind} for `{key}`. The {} it has: {}. {unchanged}",
                plural(kind),
                present.iter().map(|p| format!("`{p}`")).collect::<Vec<_>>().join(", ")
            ),
            EditError::WriteFailed(why) => write!(f, "safe-chains could not write {SHOWN_PATH}: {why}. {unchanged}"),
            EditError::OddName(name) => {
                write!(f, "`{name}` is not a command name an entry can have. Give the program's plain name. {unchanged}")
            }
            EditError::PathSpelled(word) => write!(
                f,
                "`{word}` names a program by its path, and an entry would approve every program with that name, \
                 wherever it is. Add it by its plain name only if that program is installed where your shell finds \
                 it. {unchanged}"
            ),
            EditError::SubAfterFlags(name, sub) => write!(
                f,
                "A flag comes before `{sub}`, so safe-chains cannot tell whether `{sub}` is a subcommand, and the \
                 entry would approve `{name}` with any words in its place. Put the subcommand first, as in \
                 `{name} {sub} …`, and add that. {unchanged}"
            ),
            EditError::LevelInCommand => write!(
                f,
                "`--level` came after the command, so it would be read as one of the command's own flags. Put it \
                 first: safe-chains config add --level inert \"…\". {unchanged}"
            ),
            EditError::CannotEditCleanly => write!(
                f,
                "safe-chains could not make this change without rewriting other parts of {SHOWN_PATH}, so it \
                 left the file alone. Make the change by hand. {unchanged}"
            ),
        }
    }
}

fn quoted_list(names: &[String], one: &str, many: &str) -> String {
    let shown: Vec<String> = names.iter().map(|n| format!("`{n}`")).collect();
    match shown.as_slice() {
        [only] => format!("{only} {one}"),
        _ => format!("{} {many}", shown.join(", ")),
    }
}

/// Something the reader can do next, named the way the surface they are using spells it.
///
/// The MCP server runs these operations through the CLI and sets `SAFE_CHAINS_SURFACE=mcp`, so
/// a model calling tools is told which tool to call rather than a command it may have no shell for.
/// The variable changes wording only.
pub enum Step<'a> {
    Add(&'a str),
    Grant(&'a str, Access),
    RemoveCommand(&'a str),
    RemoveGrant(&'a str),
    Explain,
}

pub fn step(step: Step<'_>) -> String {
    let mcp = std::env::var_os("SAFE_CHAINS_SURFACE").is_some_and(|v| v == "mcp");
    let quote = |s: &str| crate::sanitize_display(&shell_words::quote(s));
    let quote_path = |s: &str| match s.strip_prefix("~/") {
        Some(rest) => format!("~/{}", quote(rest)),
        None => quote(s),
    };
    let json = |s: &str| crate::sanitize_display(&serde_json::Value::from(s).to_string());
    match (step, mcp) {
        (Step::Add(command), false) => format!("safe-chains config add {}", quote(command)),
        (Step::Add(command), true) => format!("call add_command_entry with command {}", json(command)),
        (Step::Grant(path, access), false) => format!("safe-chains config grant {} {}", quote_path(path), access.flags()),
        (Step::Grant(path, access), true) => format!("call grant_path with path {} and access \"{}\"", json(path), access.name()),
        (Step::RemoveCommand(name), false) => format!("safe-chains config remove command {}", quote(name)),
        (Step::RemoveCommand(name), true) => format!("call remove_rule with kind \"command\" and name {}", json(name)),
        (Step::RemoveGrant(path), false) => format!("safe-chains config remove grant {}", quote_path(path)),
        (Step::RemoveGrant(path), true) => format!("call remove_rule with kind \"grant\" and name {}", json(path)),
        (Step::Explain, false) => "`safe-chains --explain` on it".to_string(),
        (Step::Explain, true) => "check_command on it".to_string(),
    }
}

/// Whether `name` runs an AI coding agent, which no entry may approve.
pub fn runs_an_agent(name: &str) -> bool {
    guard::check_entry_name(name).is_err()
}

/// The commands that run an AI coding agent.
pub fn agent_commands() -> &'static [&'static str] {
    guard::AGENT_COMMANDS
}

/// `path` as `safe-chains config grant` would write it, or why that command would refuse it.
pub fn grantable_path(path: &str) -> Result<String, EditError> {
    let path = guard::normalize_path(path)?;
    guard::check_grant_path(&path)?;
    Ok(path)
}

/// The level an added entry gets, as the CLI spells it and as the file spells it.
pub fn entry_level(name: &str) -> Result<&'static str, EditError> {
    match name.trim().to_ascii_lowercase().replace('_', "-").as_str() {
        "inert" => Ok("Inert"),
        "safe-read" | "saferead" => Ok("SafeRead"),
        "safe-write" | "safewrite" | "" => Ok("SafeWrite"),
        _ => Err(EditError::UnknownLevel(name.to_string())),
    }
}

/// `~/.config/safe-chains.toml` under `$HOME`. The same rule as the loader: never `XDG_CONFIG_HOME`.
pub fn user_config_path() -> Result<PathBuf, EditError> {
    let home = std::env::var_os("HOME").filter(|h| !h.is_empty()).ok_or(EditError::NoHome)?;
    Ok(PathBuf::from(home).join(".config/safe-chains.toml"))
}

fn read_existing(path: &Path) -> Result<String, EditError> {
    match fs::read_to_string(path) {
        Ok(source) => Ok(source),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(EditError::Unreadable(e.to_string())),
    }
}

/// Replace the file's contents in one step, so a reader never sees half of it.
fn write_atomically(path: &Path, contents: &str) -> Result<(), EditError> {
    let fail = |e: std::io::Error| EditError::WriteFailed(e.to_string());
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(fail)?;
    }
    let staging = path.with_extension(format!("toml.{}.tmp", std::process::id()));
    fs::write(&staging, contents).map_err(fail)?;
    if let Ok(meta) = fs::metadata(path) {
        let _ = fs::set_permissions(&staging, meta.permissions());
    }
    fs::rename(&staging, path).map_err(|e| {
        let _ = fs::remove_file(&staging);
        fail(e)
    })
}

/// Apply `change` to the user config and write the result, or change nothing.
fn update(change: impl FnOnce(&str) -> Result<(String, String), EditError>) -> Result<String, EditError> {
    let path = user_config_path()?;
    let existing = read_existing(&path)?;
    edit::check_loads(&existing)?;
    let (updated, message) = change(&existing)?;
    edit::check_loads(&updated)?;
    write_atomically(&path, &updated)?;
    Ok(message)
}

/// Add an entry that approves `command` as it was written, at `level` (`inert`, `safe-read`,
/// `safe-write`). Returns what to tell the person.
pub fn add_entry(command: &str, level: &str) -> Result<String, EditError> {
    let level = entry_level(level)?;
    update(|existing| edit::add_entry(existing, command, level))
}

/// Remove the command entry named `name`.
pub fn remove_command(name: &str) -> Result<String, EditError> {
    update(|existing| edit::remove_command(existing, name.trim()))
}

/// Remove the grant on `path`.
pub fn remove_grant(path: &str) -> Result<String, EditError> {
    let path = guard::normalize_path(path)?;
    update(|existing| edit::remove_grant(existing, &path))
}

/// Grant `access` to `path`, merging with a grant already on that path.
pub fn grant(path: &str, access: Access) -> Result<String, EditError> {
    let path = guard::normalize_path(path)?;
    guard::check_grant_path(&path)?;
    update(|existing| edit::grant(existing, &path, access))
}

#[cfg(test)]
mod tests;

fn plural(kind: &str) -> String {
    match kind.strip_suffix('y') {
        Some(stem) => format!("{stem}ies"),
        None => format!("{kind}s"),
    }
}

/// Fuzz seam onto the edits, which are otherwise private: the text in, the text out, no files.
///
/// The user's own file is the input these edits do not control. It may be anything a person or a
/// tool once wrote, and an edit that panicked on it, or wrote back something that no longer loads,
/// would switch off every rule in it.
#[doc(hidden)]
pub mod fuzzing {
    pub use super::edit::check_loads;

    pub fn grant(existing: &str, path: &str, access: super::Access) -> Result<String, super::EditError> {
        super::edit::grant(existing, path, access).map(|(text, _)| text)
    }

    pub fn remove_grant(existing: &str, path: &str) -> Result<String, super::EditError> {
        super::edit::remove_grant(existing, path).map(|(text, _)| text)
    }

    pub fn add_entry(existing: &str, command: &str) -> Result<String, super::EditError> {
        super::edit::add_entry(existing, command, "SafeRead").map(|(text, _)| text)
    }

    pub fn remove_command(existing: &str, name: &str) -> Result<String, super::EditError> {
        super::edit::remove_command(existing, name).map(|(text, _)| text)
    }
}
