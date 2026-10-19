//! Which kind of refusal a gated command is: one safe-chains has not approved YET, or a boundary.
//!
//! A harness that can put a command to a person in the moment (Grok's `ask`) can let a person decide
//! the first kind, since nothing about it is known to be dangerous: there is simply no entry or grant
//! for it. The second kind stays a hard refusal whatever the harness can do, because it is what
//! safe-chains exists to stop an agent from getting through by asking: credentials, raw devices,
//! the files safe-chains and the system read their trust from, the places whose contents run later
//! without anyone being asked, other AI agents, and safe-chains' own rule edits.
//!
//! The split is read from [`fix_for`]: it already says, for each refused segment, whether an entry
//! or a grant would change the answer, or why nothing can. Where that answer does not see a boundary
//! (an option refused by a built-in rule may still write to `~/.zshrc`, as `git diff --output` does),
//! the segment's words are checked too. Every doubt resolves to `Boundary`, which is today's deny.

use super::{Because, Fix, fix_for};
use crate::ReachReason;

/// The kind of a gated command. See the module docs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GateKind {
    /// Nothing approves it yet, and nothing marks it as dangerous.
    NotYet,
    /// It touches something safe-chains never lets an agent through to by asking.
    Boundary,
}

/// The kind of a gated command, from its explanation.
///
/// `approved` answers "is this segment approved?" at the caller's level; `approved_by_default`
/// answers it at the default level. A command is a `Boundary` when any refused segment is one.
pub fn gate_kind(
    explanation: &crate::cst::Explanation,
    approved: &dyn Fn(&str) -> bool,
    approved_by_default: &dyn Fn(&str) -> bool,
) -> GateKind {
    let refused: Vec<&str> = explanation
        .segments
        .iter()
        .map(|s| s.text.as_str())
        .filter(|text| !explanation.parsed || !approved(text))
        .collect();
    if refused.is_empty() {
        return GateKind::Boundary;
    }
    let any_boundary = refused
        .iter()
        .any(|segment| segment_kind(segment, approved, approved_by_default(segment)) == GateKind::Boundary);
    if any_boundary { GateKind::Boundary } else { GateKind::NotYet }
}

/// The kind of one refused segment.
pub fn segment_kind(segment: &str, approved: &dyn Fn(&str) -> bool, approved_by_default: bool) -> GateKind {
    match fix_for(segment, approved, approved_by_default) {
        Fix::Nothing(Because::ProtectedPath { reason, .. }) if is_boundary_reason(reason) => GateKind::Boundary,
        Fix::Nothing(Because::RunsAnAgent { .. }) => GateKind::Boundary,
        Fix::Nothing(Because::RunsLater { path }) => {
            if reads_only(segment, &path, approved) {
                GateKind::NotYet
            } else {
                GateKind::Boundary
            }
        }
        _ if words_name_a_boundary(segment) => GateKind::Boundary,
        _ => GateKind::NotYet,
    }
}

fn is_boundary_reason(reason: ReachReason) -> bool {
    match reason {
        ReachReason::Credential
        | ReachReason::RawDevice
        | ReachReason::FrozenTrustFile
        | ReachReason::FrozenTrustRoot
        | ReachReason::FrozenSystemIntegrity => true,
        ReachReason::RepositorySnapshot | ReachReason::OutsideWorkspace | ReachReason::ForeignTemp | ReachReason::Unconfined => false,
    }
}

/// Whether a READ grant on `path` alone would get `segment` approved: the segment only reads the
/// runs-later place it reached, which is not a boundary. A write or an exec there needs more.
fn reads_only(segment: &str, path: &str, approved: &dyn Fn(&str) -> bool) -> bool {
    let resolved = crate::pathctx::resolve(path).into_owned();
    crate::engine::resolve::regions::hypothetical::with_hypothetical_grant(&resolved, true, false, || approved(segment))
}

/// Whether any word of `segment` names a protected place, an AI agent, or safe-chains itself.
///
/// The words are taken loosely (split on whitespace, with quotes, redirections and `--flag=`
/// prefixes removed), which can only find more boundaries than there are, never fewer.
fn words_name_a_boundary(segment: &str) -> bool {
    segment.split_whitespace().map(bare_word).any(|word| {
        let name = word.rsplit('/').next().unwrap_or(word);
        name == "safe-chains" || crate::userconfig::runs_an_agent(name) || names_a_protected_place(word)
    })
}

/// A word as a path or a command name: the part after the last `=`, `>` or `<`, without quotes.
fn bare_word(word: &str) -> &str {
    let quotes: &[char] = &['\'', '"', '`', '(', ')', ';', '&', '|', '{', '}'];
    let tail = word.rsplit(['=', '>', '<']).next().unwrap_or(word);
    tail.trim_matches(quotes)
}

fn names_a_protected_place(word: &str) -> bool {
    let home = std::env::var("HOME").unwrap_or_default();
    let expanded = if let Some(rest) = word.strip_prefix("$HOME").or_else(|| word.strip_prefix("${HOME}")) {
        format!("{home}{rest}")
    } else if word.starts_with('~') || word.starts_with('/') {
        word.to_string()
    } else if word.contains('/') || word.starts_with('.') {
        crate::pathctx::resolve(word).into_owned()
    } else {
        return false;
    };
    matches!(crate::userconfig::grantable_path(&expanded), Err(crate::userconfig::EditError::Protected(_, _)))
}

#[cfg(test)]
mod tests;
