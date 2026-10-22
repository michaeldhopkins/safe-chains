//! The functions a line defines, so a later call resolves to its body, and the bounds on resolving one.

use super::classify_budget::charge_classify_work;
use super::{Script, SimpleCmd};
use crate::verdict::Verdict;

thread_local! {
    /// Functions DEFINED so far in the current classification, so a later call resolves to its body
    /// (and a definition SHADOWS a same-named built-in — `ls(){ rm -rf /; }; ls` runs rm). Owned
    /// clones (small); a thread-local can't borrow the CST. Latest definition wins.
    static FUNCTIONS: std::cell::RefCell<Vec<(String, Script)>> =
        const { std::cell::RefCell::new(Vec::new()) };
    /// Function names whose CURRENT body we cannot attribute — redefined inside a compound, where
    /// the shell keeps the new body but we cannot say which one ran. A call to one denies, rather
    /// than resolving to a stale definition or to the command the name shadowed.
    static POISONED_FUNCS: std::cell::RefCell<Vec<String>> =
        const { std::cell::RefCell::new(Vec::new()) };
    /// Names currently being resolved — bounds recursion (direct AND mutual) and total call depth,
    /// so `f(){ f; }` or a deep chain can't blow the stack; hitting the bound denies (fail-closed).
    static RESOLVING: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

const MAX_FUNC_DEPTH: usize = 32;

pub(super) struct FuncScope;
impl Drop for FuncScope {
    fn drop(&mut self) {
        FUNCTIONS.with(|f| {
            f.borrow_mut().pop();
        });
    }
}

pub(super) fn define_function(name: String, body: Script) -> FuncScope {
    FUNCTIONS.with(|f| f.borrow_mut().push((name, body)));
    FuncScope
}

/// Whether `name` may have been redefined with a body we cannot attribute.
pub(super) fn body_unknown(name: &str) -> bool {
    POISONED_FUNCS.with(|p| p.borrow().iter().any(|n| n == name))
}

pub(super) fn lookup_function(name: &str) -> Option<Script> {
    if body_unknown(name) {
        return None;
    }
    FUNCTIONS.with(|f| f.borrow().iter().rev().find(|(n, _)| n == name).map(|(_, b)| b.clone()))
}

/// Called as an outermost scope walk starts: what one command redefined is no fact about the next.
/// Never from a nested walk, whose caller still relies on the marks (`bash -c` inside a command).
pub(super) fn forget_poisoned_functions() {
    POISONED_FUNCS.with(|p| p.borrow_mut().clear());
}

/// Mark `name`'s body unknown for the rest of this evaluation. Not scoped by a guard: the shell's
/// redefinition is not scoped either, and the outermost scope walk clears the list.
pub(super) fn poison_function(name: String) {
    POISONED_FUNCS.with(|p| p.borrow_mut().push(name));
}

struct ResolveScope;
impl Drop for ResolveScope {
    fn drop(&mut self) {
        RESOLVING.with(|r| {
            r.borrow_mut().pop();
        });
    }
}

/// Begin resolving a call to `name`, unless it recurses, exceeds the depth cap, or exhausts the
/// per-invocation classification budget — then return `None` and the caller treats it as an ordinary
/// (unknown) command, which denies. The budget is what stops exponential FAN-OUT (`f(){ f2; f2; };
/// f2(){ f3; f3; }; …`): the depth cap alone bounds a linear chain, but branching multiplies, so each
/// resolution charges the shared `CLASSIFY_WORK` counter that also caps delegating-handler recursion.
fn begin_resolving(name: &str) -> Option<ResolveScope> {
    if !charge_classify_work(1) {
        return None;
    }
    RESOLVING.with(|r| {
        let mut stack = r.borrow_mut();
        if stack.len() >= MAX_FUNC_DEPTH || stack.iter().any(|n| n == name) {
            None
        } else {
            stack.push(name.to_string());
            Some(ResolveScope)
        }
    })
}

/// The verdict of a call to `name` when a definition rebinds it, `None` when none does.
///
/// The body is judged with $1..$N bound to the call's arguments (certain literals; uncertain →
/// unpinnable). The shadow is UNCONDITIONAL: a body we cannot attribute, or a resolution blocked by
/// recursion, depth or budget, denies and never falls through to the real command. Otherwise
/// `…512 calls…; ls(){ rm -rf /; }; ls` would exhaust the budget and then run the real `ls`.
pub(super) fn call_verdict(cmd: &SimpleCmd, name: &str) -> Option<Verdict> {
    if body_unknown(name) {
        return Some(Verdict::Denied);
    }
    let body = lookup_function(name)?;
    let Some(_resolving) = begin_resolving(name) else {
        return Some(Verdict::Denied);
    };
    let _args: Vec<crate::pathctx::VarGuard> = cmd.words[1..]
        .iter()
        .enumerate()
        .map(|(i, w)| crate::pathctx::enter_var((i + 1).to_string(), super::check::certain_value(w)))
        .collect();
    Some(super::netargs::with_args(cmd, || super::check::script_verdict(&body)))
}
