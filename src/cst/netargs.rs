//! Arguments the shell fills in at run time, handed to a command that reaches another host.
//!
//! `curl "https://example.com/?q=$HOME"` reads like a fetch, and the fetch alone is something the
//! reader band admits. What the line does not show is the value: `$HOME`, `$(cmd)`, an xargs item
//! or a `find -exec` placeholder is filled in from this machine, and the request then carries it
//! away. So a network command (`network.toml`) whose arguments carry such a value is classified as
//! sending host data to an unnamed destination, and only the levels that admit that pass it.
//!
//! The mark has to survive re-parsing. `timeout`, `env`, `xargs`, `find -exec`, `fd -x`, `sh -c`,
//! `ssh` and `tmux` re-join their inner words with `shell_words::join` and classify the result
//! again, and the join quotes `$X` into a literal `'$X'`. The inner command can no longer see the
//! expansion, so the outer one records it in a thread-local that nested classifications inherit.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use serde::Deserialize;

use super::{Redir, Script, SimpleCmd, Word, WordPart};
use crate::engine::facet::{Capability, NetDestination, NetDirection, NetPayload, Network, Operation, Profile, Provenance, RemoteReach};
use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

#[derive(Deserialize)]
struct NetworkList {
    commands: HashSet<String>,
    file_inputs: HashMap<String, Vec<String>>,
    subcommands: HashMap<String, Vec<String>>,
}

static LIST: LazyLock<NetworkList> =
    LazyLock::new(|| toml::from_str(include_str!("../../network.toml")).expect("network.toml is invalid TOML"));

thread_local! {
    static CARRIES: Cell<bool> = const { Cell::new(false) };
}

pub(crate) struct CarryGuard(bool);

impl Drop for CarryGuard {
    fn drop(&mut self) {
        CARRIES.with(|c| c.set(self.0));
    }
}

/// Mark the classification below as carrying run-time values, on top of any mark already set by an
/// enclosing command. Never clears a mark: a wrapper's inner command inherits it.
#[must_use]
pub(crate) fn enter(carries: bool) -> CarryGuard {
    let prev = CARRIES.with(Cell::get);
    CARRIES.with(|c| c.set(prev || carries));
    CarryGuard(prev)
}

fn carrying() -> bool {
    CARRIES.with(Cell::get)
}

/// Run `leaf` with the mark set when `cmd`'s arguments, environment values or redirects carry a
/// run-time value.
pub(crate) fn with_args(cmd: &SimpleCmd, leaf: impl FnOnce() -> Verdict) -> Verdict {
    let _mark = enter(simple_carries(cmd));
    leaf()
}

fn simple_carries(cmd: &SimpleCmd) -> bool {
    cmd.words.iter().skip(1).any(word_carries)
        || cmd.env.iter().any(|(_, v)| word_carries(v))
        || cmd.redirs.iter().any(|r| match r {
            Redir::Read { target, .. } | Redir::ReadWrite { target, .. } => word_carries(target),
            Redir::Write { .. } => false,
            Redir::HereStr(w) | Redir::HereDoc { body: w, .. } => word_carries(w),
            Redir::DupFd { .. } => false,
        })
}

/// Whether the shell fills in any part of `word` at run time. Every `$` in unquoted or
/// double-quoted text counts (`${X:-y}`, `$@`, `$'…'` and a stray `$` alike), except a plain `$name`
/// or `${name}` naming a `for` variable whose list was literal text.
pub(crate) fn word_carries(word: &Word) -> bool {
    word.0.iter().any(|part| match part {
        WordPart::Lit(s) => text_carries(s),
        WordPart::DQuote(inner) => word_carries(inner),
        WordPart::CmdSub(_) | WordPart::ProcSub(_) | WordPart::Backtick(_) | WordPart::Arith(_) => true,
        WordPart::SQuote(_) | WordPart::Escape(_) => false,
    })
}

fn text_carries(text: &str) -> bool {
    let mut rest = text;
    while let Some(i) = rest.find('$') {
        let after = &rest[i + 1..];
        match plain_reference(after) {
            Some((name, used)) if literal_var(name) => rest = &after[used..],
            _ => return true,
        }
    }
    false
}

fn plain_reference(after: &str) -> Option<(&str, usize)> {
    let (name, used) = match after.strip_prefix('{') {
        Some(braced) => {
            let close = braced.find('}')?;
            (&braced[..close], close + 2)
        }
        None => {
            let len = after.bytes().take_while(|b| b.is_ascii_alphanumeric() || *b == b'_').count();
            (&after[..len], len)
        }
    };
    is_identifier(name).then_some((name, used))
}

fn is_identifier(name: &str) -> bool {
    name.bytes().next().is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

thread_local! {
    static LOOP_VARS: RefCell<Vec<(String, bool)>> = const { RefCell::new(Vec::new()) };
}

struct LoopGuard;

impl Drop for LoopGuard {
    fn drop(&mut self) {
        LOOP_VARS.with(|v| {
            v.borrow_mut().pop();
        });
    }
}

/// Classify a `for` body, knowing whether `var` can only ever hold the literal text of its list.
///
/// `for h in a.example.com b.example.com; do dig "$h"; done` names its hosts in plain sight, so
/// `$h` carries nothing from the machine. That holds only while nothing in the body can give `h`
/// another value: an assignment, a builtin that assigns by name (`read`, `declare`, `printf -v`,
/// `eval`, `source`), or a call to a function, any of which could. A glob, `~` or expansion in the
/// list fills it from the machine instead.
pub(crate) fn with_loop<T>(var: &str, items: &[Word], body: &Script, run: impl FnOnce() -> T) -> T {
    let literal = items.iter().all(is_plain_item) && !may_rebind(var, &body.to_string());
    LOOP_VARS.with(|v| v.borrow_mut().push((var.to_string(), literal)));
    let _pop = LoopGuard;
    run()
}

fn literal_var(name: &str) -> bool {
    LOOP_VARS.with(|v| v.borrow().iter().rev().find(|(n, _)| n == name).is_some_and(|(_, literal)| *literal))
}

fn is_plain_item(word: &Word) -> bool {
    word.0.iter().all(|part| match part {
        WordPart::Lit(s) => !s.contains(['$', '*', '?', '[', '{', '~']),
        WordPart::DQuote(inner) => is_plain_item(inner),
        WordPart::SQuote(_) | WordPart::Escape(_) => true,
        WordPart::CmdSub(_) | WordPart::ProcSub(_) | WordPart::Backtick(_) | WordPart::Arith(_) => false,
    })
}

const ASSIGNS_BY_NAME: &[&str] =
    &["declare", "typeset", "local", "export", "readonly", "read", "mapfile", "readarray", "eval", "let", "getopts", "source", "."];

fn may_rebind(var: &str, body: &str) -> bool {
    let words: Vec<&str> = body
        .split(|c: char| c.is_whitespace() || ";|&()<>\"'`".contains(c))
        .filter(|w| !w.is_empty())
        .collect();
    if words.iter().any(|w| ASSIGNS_BY_NAME.contains(w) || super::check::lookup_function(w).is_some())
        || (words.contains(&"printf") && words.iter().any(|w| w.starts_with("-v")))
    {
        return true;
    }
    let mut from = 0;
    while let Some(offset) = body[from..].find(var) {
        let (start, end) = (from + offset, from + offset + var.len());
        from = end;
        let (before, after) = (&body[..start], &body[end..]);
        let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
        if before.chars().next_back().is_some_and(ident) || after.chars().next().is_some_and(ident) {
            continue;
        }
        let braced = (before.ends_with("${") || before.ends_with("${#")) && !after.starts_with('=') && !after.starts_with(":=");
        if !before.ends_with('$') && !braced {
            return true;
        }
    }
    false
}

/// Whether this invocation reaches another host, by `network.toml`.
pub(crate) fn reaches_network(tokens: &[Token]) -> bool {
    let Some(first) = tokens.first() else { return false };
    let name = crate::registry::canonical_name(first.command_name());
    if LIST.commands.contains(name) {
        return true;
    }
    LIST.subcommands
        .get(name)
        .is_some_and(|subs| tokens[1..].iter().any(|t| subs.iter().any(|s| names_word(t.as_str(), s))))
}

fn names_word(token: &str, word: &str) -> bool {
    token == word || (word.starts_with("--") && token.strip_prefix(word).is_some_and(|rest| rest.starts_with('=')))
}

/// Whether a flag makes the command read its targets or payload from a file (`wget -i FILE`).
pub(crate) fn reads_targets_from_file(tokens: &[Token]) -> bool {
    let Some(first) = tokens.first() else { return false };
    let name = crate::registry::canonical_name(first.command_name());
    let Some(flags) = LIST.file_inputs.get(name) else { return false };
    tokens[1..]
        .iter()
        .take_while(|t| t.as_str() != "--")
        .any(|t| flags.iter().any(|f| flag_present(t.as_str(), f)))
}

fn flag_present(token: &str, flag: &str) -> bool {
    if token == flag || token.strip_prefix(flag).is_some_and(|rest| rest.starts_with('=')) {
        return true;
    }
    let is_single_letter = flag.len() == 2 && flag.starts_with('-') && !flag.starts_with("--");
    if !is_single_letter || token.starts_with("--") || !token.starts_with('-') {
        return false;
    }
    if token.starts_with(flag) {
        return true;
    }
    let letter = &flag[1..];
    token[1..].chars().all(|c| c.is_ascii_alphanumeric()) && token[1..].contains(letter)
}

/// `verdict`, combined with what a network leaf adds.
pub(crate) fn with_egress(tokens: &[Token], verdict: Verdict) -> Verdict {
    verdict.combine(egress_verdict(tokens))
}

/// The verdict a network leaf adds: `Allowed(Inert)` when nothing run-time reaches it, otherwise
/// the projection of an opaque send of host data.
pub(crate) fn egress_verdict(tokens: &[Token]) -> Verdict {
    let carries = carrying() || reads_targets_from_file(tokens);
    if !carries || !reaches_network(tokens) {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let mut c = Capability::new(Operation::Communicate);
    c.locus.remote = RemoteReach::Arbitrary;
    c.locus.provenance = Provenance::Opaque;
    c.network = Network { direction: NetDirection::Outbound, destination: NetDestination::Arbitrary, payload: NetPayload::SendsHostData };
    c.because = "a network command's arguments carry a value filled in at run time".to_string();
    crate::engine::bridge::project(&Profile::of(vec![c]))
}

#[cfg(test)]
#[path = "netargs_tests.rs"]
mod tests;
