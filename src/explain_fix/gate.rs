//! Which kind of refusal a gated command is: one safe-chains has not approved YET, or a boundary.
//!
//! A harness that can put a command to a person in the moment (Grok's `ask`) can let a person decide
//! the first kind, since nothing about it is known to be dangerous: there is simply no entry or grant
//! for it. The second kind stays a hard refusal whatever the harness can do, because it is what
//! safe-chains exists to stop an agent from getting through by asking: credentials, raw devices,
//! the files safe-chains and the system read their trust from, the places whose contents run later
//! without anyone being asked, other AI agents, and safe-chains' own rule edits.
//!
//! `NotYet` is the narrow answer and `Boundary` the default. A command is `NotYet` only when every
//! check below passes; anything the checks cannot read with confidence is a `Boundary`, which is
//! today's deny:
//!
//! - The command text holds nothing that makes its words unknowable before the shell runs it: no
//!   `$` expansion, backtick, glob or brace, `~user`, change of directory, or program that runs code
//!   it was handed or names (a shell, `eval`, an interpreter, `xargs`, `env`, a package runner), and
//!   no `-P` cluster keeping absolute names.
//! - With quotes and backslashes removed, no word of the WHOLE command (approved segments, function
//!   bodies and `-c` strings included, attached option values too) names a protected place, a place
//!   outside the workspace but for a temporary folder, a project file that runs later (`.git`,
//!   `.envrc`, an agent's or editor's project settings), an AI agent or its package, or safe-chains.
//!   Removing them can only join or split words, so this finds every name the shell would see.
//! - The whole command reaches no protected place by [`crate::workspace_overreach`].
//! - Each refused segment's [`fix_for`] answer is an entry, a grant, a level, or a read of a
//!   runs-later place, never a credential, a device, a frozen file, an agent or an interpolation.

use super::{Because, Fix, fix_for};
use crate::ReachReason;

/// The kind of a gated command. See the module docs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GateKind {
    /// Nothing approves it yet, and nothing marks it as dangerous.
    NotYet,
    /// It touches something safe-chains never lets an agent through to by asking, or it cannot be
    /// read well enough to say it does not.
    Boundary,
}

/// The kind of the gated `command`, from its text and its explanation.
///
/// `approved` answers "is this segment approved?" at the caller's level; `approved_by_default`
/// answers it at the default level.
pub fn gate_kind(
    command: &str,
    explanation: &crate::cst::Explanation,
    approved: &dyn Fn(&str) -> bool,
    approved_by_default: &dyn Fn(&str) -> bool,
) -> GateKind {
    if text_is_opaque(command) || words_name_a_boundary(command) || reaches_a_boundary(command) {
        return GateKind::Boundary;
    }
    let refused: Vec<&str> = explanation
        .segments
        .iter()
        .map(|s| s.text.as_str())
        .filter(|text| !explanation.parsed || !approved(text))
        .collect();
    if refused.is_empty() {
        return GateKind::Boundary;
    }
    let any_boundary = refused.iter().any(|segment| {
        text_is_opaque(segment)
            || words_name_a_boundary(segment)
            || segment_kind(segment, approved, approved_by_default(segment)) == GateKind::Boundary
    });
    if any_boundary { GateKind::Boundary } else { GateKind::NotYet }
}

/// The kind of one refused segment, from what [`fix_for`] says would approve it.
pub fn segment_kind(segment: &str, approved: &dyn Fn(&str) -> bool, approved_by_default: bool) -> GateKind {
    match fix_for(segment, approved, approved_by_default) {
        Fix::Nothing(Because::ProtectedPath { reason, .. }) if is_boundary_reason(reason) => GateKind::Boundary,
        Fix::Nothing(Because::RunsAnAgent { .. } | Because::Unconfined { .. }) => GateKind::Boundary,
        Fix::Nothing(Because::RunsLater { path }) => {
            if reads_only(segment, &path, approved) {
                GateKind::NotYet
            } else {
                GateKind::Boundary
            }
        }
        _ => GateKind::NotYet,
    }
}

fn is_boundary_reason(reason: ReachReason) -> bool {
    match reason {
        ReachReason::Credential
        | ReachReason::OtherUsersHome
        | ReachReason::RawDevice
        | ReachReason::FrozenTrustFile
        | ReachReason::FrozenTrustRoot
        | ReachReason::FrozenSystemIntegrity
        | ReachReason::Unconfined => true,
        ReachReason::RepositorySnapshot | ReachReason::OutsideWorkspace | ReachReason::ForeignTemp => false,
    }
}

fn reaches_a_boundary(command: &str) -> bool {
    crate::workspace_overreach(command).is_some_and(|(_, reason)| is_boundary_reason(reason))
}

/// Whether a READ grant on `path` alone would get `segment` approved: the segment only reads the
/// runs-later place it reached, which is not a boundary. A write or an exec there needs more.
fn reads_only(segment: &str, path: &str, approved: &dyn Fn(&str) -> bool) -> bool {
    let resolved = crate::pathctx::resolve(path).into_owned();
    crate::engine::resolve::regions::hypothetical::with_hypothetical_grant(&resolved, true, false, || approved(segment))
}

/// Characters after which the shell, not the text, decides what a word names.
const OPAQUE: &[char] = &['$', '`', '*', '?', '[', ']', '{', '}'];

/// Programs that change the directory later words are read against, run code handed to them, or
/// run a program named by their input or by a package.
const OPAQUE_PROGRAMS: &[&str] = &[
    "cd", "pushd", "popd", "eval", "source", "exec", "sh", "bash", "zsh", "dash", "ksh", "mksh", "fish", "csh", "tcsh", "python",
    "python3", "perl", "ruby", "node", "deno", "bun", "php", "lua", "osascript", "tclsh", "xargs", "env", "nohup", "timeout", "nice",
    "awk", "gawk", "mawk", "nawk", "npx", "bunx", "pnpx", "uvx", "pipx", "dlx", "-exec", "-execdir", "-ok", "-okdir", "-P",
    "--absolute-names",
];

/// Option prefixes, attached or not, that change the directory later words are read against.
const DIRECTORY_PREFIXES: &[&str] = &["-C"];

/// Options that change the directory later words are read against.
const DIRECTORY_OPTIONS: &[&str] = &["-C", "--directory", "--chdir", "--cwd", "--work-tree", "--git-dir"];

/// A cluster of short options holding `-P`, which makes `tar` (and others) keep absolute names.
fn keeps_absolute_names(word: &str) -> bool {
    word.strip_prefix('-')
        .is_some_and(|flags| !flags.starts_with('-') && flags.chars().all(|c| c.is_ascii_alphabetic()) && flags.contains('P'))
}

/// Whether `text` holds anything that makes what its words name unknowable from the text alone.
fn text_is_opaque(text: &str) -> bool {
    text.contains(OPAQUE)
        || words(text).any(|word| {
            let name = word.rsplit('/').next().unwrap_or(&word);
            (word.starts_with('~') && word != "~" && !word.starts_with("~/"))
                || OPAQUE_PROGRAMS.contains(&name)
                || DIRECTORY_OPTIONS.contains(&word.as_str())
                || DIRECTORY_PREFIXES.iter().any(|prefix| word.starts_with(prefix))
                || keeps_absolute_names(&word)
        })
}

/// The words of `text` as the shell would see their names: quotes and backslashes removed, then
/// split on whitespace and on the operators, redirections and separators that end a word.
fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    let dequoted: String = text.chars().filter(|c| !matches!(c, '\'' | '"' | '\\')).collect();
    dequoted
        .split(|c: char| c.is_whitespace() || matches!(c, ';' | '|' | '&' | '(' | ')' | '<' | '>' | '=' | ',' | ':'))
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>()
        .into_iter()
}

/// Whether any word of `text` names a protected place, a place outside the workspace, an AI agent
/// (by its command or a package name holding it), or safe-chains itself.
fn words_name_a_boundary(text: &str) -> bool {
    words(text).any(|word| {
        let name = word.rsplit('/').next().unwrap_or(&word);
        name == "safe-chains"
            || crate::userconfig::runs_an_agent(name)
            || names_an_agent_package(&word)
            || path_parts(&word).any(|path| names_a_protected_place(path) || leaves_the_workspace(path))
            || path_parts(&word).any(names_a_place_that_runs_later_here)
    })
}

/// Names, as one part of a path, of what runs later from inside a project without anyone being
/// asked: git's hooks and config, direnv, task runners' and agents' project settings.
const RUNS_LATER_HERE: &[&str] = &[
    ".git", ".envrc", ".env", ".claude", ".grok", ".gemini", ".codex", ".qwen", ".cursor", ".factory", ".agy", ".antigravity", ".opencode",
    ".vscode", ".idea", ".husky", ".mise.toml", "mise.toml", ".tool-versions", ".github", ".devcontainer",
];

fn names_a_place_that_runs_later_here(path: &str) -> bool {
    path.split('/').any(|part| RUNS_LATER_HERE.contains(&part))
}

/// Agent names long enough to look for inside a word (`@anthropic-ai/claude-code`, `aider-chat`).
fn names_an_agent_package(word: &str) -> bool {
    let word = word.to_ascii_lowercase();
    crate::userconfig::agent_commands()
        .iter()
        .filter(|name| name.len() >= 5)
        .any(|name| word.contains(name))
}

/// The parts of a word that may be a path: the word, and for an option with its value attached
/// (`-o../x`, `-t/usr/bin`) the value, from its first `/`, `.` or `~`.
fn path_parts(word: &str) -> impl Iterator<Item = &str> {
    let attached = word
        .strip_prefix('-')
        .filter(|rest| !rest.starts_with('-'))
        .and_then(|rest| rest.find(['/', '.', '~']).map(|at| &rest[at..]));
    std::iter::once(word).chain(attached)
}

/// Whether `word`, read as a path, names somewhere outside the workspace other than a temporary
/// folder. [`crate::pathctx::resolve`] returns a path inside the workspace relative to it.
fn leaves_the_workspace(word: &str) -> bool {
    if !(word.starts_with(['~', '/', '.']) || word.contains('/')) {
        return false;
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let expanded = match word.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => format!("{home}{rest}"),
        _ => word.to_string(),
    };
    let resolved = crate::pathctx::resolve(&expanded);
    let outside = resolved.starts_with(['/', '~']) || resolved == ".." || resolved.starts_with("../");
    outside && !crate::pathctx::under_temp_root(&resolved) && !crate::pathctx::in_session_scratchpad(&resolved)
}

fn names_a_protected_place(word: &str) -> bool {
    let path = if word.starts_with('~') || word.starts_with('/') {
        word.to_string()
    } else if word.contains('/') || word.starts_with('.') {
        crate::pathctx::resolve(word).into_owned()
    } else {
        return false;
    };
    let home = std::env::var("HOME").unwrap_or_default();
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "~" || (!home.is_empty() && trimmed == home.trim_end_matches('/')) {
        return true;
    }
    matches!(crate::userconfig::grantable_path(&path), Err(crate::userconfig::EditError::Protected(_, _)))
}

#[cfg(test)]
mod tests;
