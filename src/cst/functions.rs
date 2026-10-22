//! The functions a line defines, so a later call resolves to its body, and the bounds on resolving one.

use super::Script;
use super::classify_budget::charge_classify_work;

thread_local! {
    /// Functions DEFINED so far in the current classification, so a later call resolves to its body
    /// (and a definition SHADOWS a same-named built-in — `ls(){ rm -rf /; }; ls` runs rm). Owned
    /// clones (small); a thread-local can't borrow the CST. Latest definition wins.
    static FUNCTIONS: std::cell::RefCell<Vec<(String, Script)>> =
        const { std::cell::RefCell::new(Vec::new()) };
    /// Function names whose CURRENT body we cannot attribute — redefined inside a compound, where
    /// the shell keeps the new body but we cannot say which one ran. `lookup_function` reports them
    /// as unknown, so a call falls through to ordinary dispatch and denies (fail-closed) rather
    /// than resolving to a stale, more permissive definition.
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

pub(super) fn lookup_function(name: &str) -> Option<Script> {
    if POISONED_FUNCS.with(|p| p.borrow().iter().any(|n| n == name)) {
        return None; // body unknown — deny rather than use a stale one
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

pub(super) struct ResolveScope;
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
pub(super) fn begin_resolving(name: &str) -> Option<ResolveScope> {
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
