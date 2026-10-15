//! Variables bound for one classification: `for`/`while read` loop variables, which stand for a
//! set of items, and `VAR=value` assignments or function positionals, which hold one certain value.
//!
//! Both kinds share one precedence: the binding entered LAST wins, whichever kind it is. A loop
//! variable reassigned in its own body (`for f in ./*; do f=/etc/shadow; cat "$f"; done`) holds
//! the assigned value from then on, and an assignment rebound by an inner loop holds the loop's
//! items. Looking loops up first let the stale loop binding answer for the assignment.

use std::borrow::Cow;
use std::cell::{Cell, RefCell};

/// A bound `for` loop variable: `$name` in the body inherits the loop's `in`-list locus (the
/// `find … {}`→path binding, one layer up). Read and write representatives can differ — a list
/// like `/etc/hosts ~/notes` reads worst at `~/notes` but writes worst at `/etc/hosts`.
struct LoopVar {
    name: String,
    read_repr: String,
    write_repr: String,
    seq: u64,
}

/// A bound `VAR=value` assignment or a function positional (`$1`). Unlike a loop var, a certain
/// literal value has ONE representative for both read and write.
struct VarBinding {
    name: String,
    value: String,
    seq: u64,
}

thread_local! {
    static LOOP_VARS: RefCell<Vec<LoopVar>> = const { RefCell::new(Vec::new()) };
    static VARS: RefCell<Vec<VarBinding>> = const { RefCell::new(Vec::new()) };
    static NEXT_SEQ: Cell<u64> = const { Cell::new(0) };
}

fn next_seq() -> u64 {
    NEXT_SEQ.with(|n| {
        let seq = n.get();
        n.set(seq.wrapping_add(1));
        seq
    })
}

/// What the innermost binding of a name holds.
pub(super) enum Bound {
    Loop { read_repr: String, write_repr: String },
    Value(String),
}

/// The binding of `name` entered last, of either kind.
pub(super) fn lookup(name: &str) -> Option<Bound> {
    let looped = LOOP_VARS.with(|v| {
        v.borrow()
            .iter()
            .rev()
            .find(|l| l.name == name)
            .map(|l| (l.seq, Bound::Loop { read_repr: l.read_repr.clone(), write_repr: l.write_repr.clone() }))
    });
    let valued = VARS.with(|v| v.borrow().iter().rev().find(|b| b.name == name).map(|b| (b.seq, Bound::Value(b.value.clone()))));
    match (looped, valued) {
        (Some((l, loop_bound)), Some((v, value))) => Some(if l > v { loop_bound } else { value }),
        (Some((_, b)), None) | (None, Some((_, b))) => Some(b),
        (None, None) => None,
    }
}

/// Every name bound so far, of either kind.
pub(super) fn bound_names() -> Vec<String> {
    let loops: Vec<String> = LOOP_VARS.with(|v| v.borrow().iter().map(|l| l.name.clone()).collect());
    VARS.with(|v| loops.into_iter().chain(v.borrow().iter().map(|b| b.name.clone())).collect())
}

/// Whether any loop variable is bound.
pub(super) fn any_loop() -> bool {
    LOOP_VARS.with(|v| !v.borrow().is_empty())
}

/// Bind `name` to a CERTAIN literal `value` (a `VAR=/path` assignment, or `$1` at a function call)
/// for the duration of the guard. The latest binding of a name wins, loop or not. The caller binds
/// only values it is certain of — an uncertain value (`VAR=$(cmd)`, `VAR=$UNBOUND`) is bound to the
/// unpinnable sentinel so `$VAR` still fail-closes rather than resolving to a stale or dropped value.
#[must_use]
pub fn enter_var(name: String, value: String) -> VarGuard {
    let seq = next_seq();
    VARS.with(|v| v.borrow_mut().push(VarBinding { name, value, seq }));
    VarGuard
}

pub struct VarGuard;

impl Drop for VarGuard {
    fn drop(&mut self) {
        VARS.with(|v| {
            v.borrow_mut().pop();
        });
    }
}

/// Bind loop variable `name` to its list's representative items for the duration of the guard
/// (the loop body's classification). Nested loops stack; the latest binding of a name wins.
#[must_use]
pub fn enter_loop_var(name: String, read_repr: String, write_repr: String) -> LoopGuard {
    let seq = next_seq();
    LOOP_VARS.with(|v| v.borrow_mut().push(LoopVar { name, read_repr, write_repr, seq }));
    LoopGuard
}

/// Pops the loop binding when dropped.
pub struct LoopGuard;

impl Drop for LoopGuard {
    fn drop(&mut self) {
        LOOP_VARS.with(|v| {
            v.borrow_mut().pop();
        });
    }
}

/// Expand any bound variable (`$name` / `${name}`) in `path` to its value, or a loop variable to
/// its representative list item — the read representative when `want_write` is false, the write
/// representative when true. Unbound `$…` is left untouched (so it still fail-closes to machine).
/// Returns `path` unchanged when nothing is bound.
pub fn expand_vars(path: &str, want_write: bool) -> Cow<'_, str> {
    if !path.contains('$') || bound_names().is_empty() {
        return Cow::Borrowed(path);
    }
    expand_with(path, want_write).map_or(Cow::Borrowed(path), Cow::Owned)
}

fn expand_with(path: &str, want_write: bool) -> Option<String> {
    let mut out = String::with_capacity(path.len());
    let mut rest = path;
    let mut replaced = false;
    while let Some(dollar) = rest.find('$') {
        out.push_str(&rest[..dollar]);
        let after = &rest[dollar + 1..];
        match parse_var(after) {
            Some((name, consumed)) => {
                match lookup(name) {
                    Some(Bound::Loop { read_repr, write_repr }) => out.push_str(if want_write { &write_repr } else { &read_repr }),
                    Some(Bound::Value(value)) => out.push_str(&value),
                    None => {
                        out.push('$');
                        out.push_str(&after[..consumed]);
                        rest = &after[consumed..];
                        continue;
                    }
                }
                replaced = true;
                rest = &after[consumed..];
            }
            None => {
                out.push('$');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    replaced.then_some(out)
}

/// Whether `text` refers to a loop variable whose items are not known: its representative stands in
/// for a locus, not for the text the shell will hold, so a copy of it is not a known value.
pub fn names_a_loop_var(text: &str) -> bool {
    let mut rest = text;
    while let Some(dollar) = rest.find('$') {
        let after = &rest[dollar + 1..];
        match parse_var(after) {
            Some((name, consumed)) => {
                if matches!(lookup(name), Some(Bound::Loop { .. })) {
                    return true;
                }
                rest = &after[consumed..];
            }
            None => rest = after,
        }
    }
    false
}

/// Parse a shell variable name immediately after a `$`: `name`, `{name}`, a single-digit positional
/// (`$1`; bash reads `$12` as `$1` then `2`), or a braced positional (`${10}`). Returns the name and
/// how many bytes of `after` it consumed, or `None` if it isn't a variable reference.
fn parse_var(after: &str) -> Option<(&str, usize)> {
    if let Some(braced) = after.strip_prefix('{') {
        let close = braced.find('}')?;
        let name = &braced[..close];
        is_var_name(name).then_some((name, close + 2))
    } else if after.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        Some((&after[..1], 1))
    } else {
        let len = after.bytes().take_while(|&b| b.is_ascii_alphanumeric() || b == b'_').count();
        let name = &after[..len];
        is_var_name(name).then_some((name, len))
    }
}

/// A `${…}` interior is a variable name — an identifier (`[A-Za-z_][A-Za-z0-9_]*`) OR an all-digit
/// positional (`${10}`).
fn is_var_name(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    if s.bytes().all(|b| b.is_ascii_digit()) {
        return true;
    }
    let mut bytes = s.bytes();
    matches!(bytes.next(), Some(b) if b.is_ascii_alphabetic() || b == b'_') && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_latest_binding_wins_whichever_kind_it_is() {
        let _loop = enter_loop_var("f".into(), "./a".into(), "./a".into());
        {
            let _assigned = enter_var("f".into(), "/etc/shadow".into());
            assert_eq!(expand_vars("$f", false), "/etc/shadow");
            assert!(matches!(lookup("f"), Some(Bound::Value(v)) if v == "/etc/shadow"));
            {
                let _inner = enter_loop_var("f".into(), "./b".into(), "./c".into());
                assert_eq!(expand_vars("$f", true), "./c");
                assert!(matches!(lookup("f"), Some(Bound::Loop { .. })));
            }
            assert_eq!(expand_vars("$f", false), "/etc/shadow");
        }
        assert_eq!(expand_vars("$f", false), "./a");
    }
}
