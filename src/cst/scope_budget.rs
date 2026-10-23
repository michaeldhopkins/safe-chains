//! The scope walk's bounds: the sentinel value an uncertain binding carries, and how many of them
//! one classification may stack.
//!
//! `walk_with_scope` binds what earlier statements established — `VAR=value` assignments, function
//! definitions, and the rebinds a compound or a called function makes that it cannot attribute. A
//! command that can rebind a name we cannot name (`source`/`.`/`let`/`printf -v`, arithmetic that
//! assigns — see `opaque::declared_names`) re-binds EVERY name bound so far, because any of them
//! may be the one that changed. Those re-binds are the uncertain ones: the name is bound to
//! [`UNCERTAIN_VALUE`], which looks like a path and is unpinnable, so a later `$VAR` fail-closes in
//! both gate layers.
//!
//! That set of names FEEDS ITSELF: `bound_names` reports every binding, sentinel copies included,
//! and each report becomes a new binding. Enter them all and k `source` statements stack ~2^k
//! copies of the same names — a 1,099-byte input of 28 of them (the `explain_render` OOM find,
//! 2026-10-11) drove the walk past 2 GB before any verdict existed. So the walk re-binds a name
//! only while it is still pinned (a second sentinel binding is a no-op: lookup takes the innermost
//! match), and the ceiling below bounds whatever remains, in the same spirit as
//! `classify_budget`'s: a real command binds a handful of names (the registry corpus enters at most
//! one), the ceiling is two orders above that, and hitting it fails CLOSED — the verdict entry
//! points deny, they never allow.

use crate::pathctx::{self, Binding};

/// The value a `$VAR`/`$1` binds to when the assigned/argument value is UNCERTAIN (a substitution,
/// an unbound var, a reassignment to same). It looks like a path AND is unpinnable, so `$VAR/x`
/// fail-closes in both gate layers rather than resolving to a stale or dropped value.
pub(super) const UNCERTAIN_VALUE: &str = "/__SAFE_CHAINS_CMDSUB__";

/// How many uncertain bindings one classification's scope walk may enter before the command is
/// refused. Two orders of magnitude above the registry corpus (whose worst command enters one),
/// and above any real script that binds a few dozen names and then sources a file — each distinct
/// name is re-bound at most once, so a script binding N names plus `source` spends N plus the
/// trusted session names, not N per `source`.
pub(super) const MAX_UNCERTAIN_BINDINGS: u32 = 256;

thread_local! {
    /// Uncertain bindings entered by the scope walk so far, and whether the ceiling above was hit.
    /// Set by the walk, read by the verdict entry points, which fail closed on it.
    static SCOPE_BINDINGS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static SCOPE_SPENT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// How many scope walks are in progress. The OUTERMOST walk owns the budget: nested walks (a
    /// compound body, a function body, a backtick re-parse) are the same classification's work, so
    /// they share it rather than each getting a fresh one.
    static SCOPE_WALKS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// RAII ownership of the scope budget: entered at the top of `walk_with_scope`, reset only when no
/// other walk is in progress.
pub(super) struct ScopeWalk;

impl ScopeWalk {
    pub(super) fn enter() -> Self {
        let top = SCOPE_WALKS.with(|d| {
            let n = d.get();
            d.set(n + 1);
            n == 0
        });
        if top {
            SCOPE_BINDINGS.with(|c| c.set(0));
            SCOPE_SPENT.with(|s| s.set(false));
            super::check::forget_poisoned_functions();
            super::arith::forget_numeric_names();
        }
        ScopeWalk
    }
}

impl Drop for ScopeWalk {
    fn drop(&mut self) {
        SCOPE_WALKS.with(|d| d.set(d.get().saturating_sub(1)));
    }
}

/// Whether `name` is already bound to the unpinnable sentinel. A second sentinel binding for the
/// same name changes nothing — lookup takes the innermost match, which is the sentinel either way —
/// so the caller skips it. Skipping is what stops the doubling: entering the copies anyway made
/// each `source` re-bind the previous `source`'s copies as well.
fn already_uncertain(name: &str) -> bool {
    matches!(pathctx::binding(name), Binding::Value(v) if v == UNCERTAIN_VALUE)
}

/// Count one uncertain binding against the walk's budget; `false` (sticky) once it is spent.
fn enter_uncertain_binding() -> bool {
    let over = SCOPE_BINDINGS.with(|c| {
        let n = c.get().saturating_add(1);
        c.set(n);
        n > MAX_UNCERTAIN_BINDINGS
    });
    if over {
        SCOPE_SPENT.with(|s| s.set(true));
    }
    !over
}

/// Bind each of `names` to the sentinel, within the budget. A name already bound to the sentinel
/// gains nothing from a second binding, and entering one anyway is what let repeated `source`s
/// double the set; names still pinned keep their uncertain rebind, which is the soundness this
/// tracking exists for. Over budget it stops: the classification denies.
pub(super) fn rebind_uncertain(names: impl IntoIterator<Item = String>, vars: &mut Vec<pathctx::VarGuard>) {
    for name in names {
        if already_uncertain(&name) {
            continue;
        }
        if !enter_uncertain_binding() {
            break;
        }
        vars.push(pathctx::enter_var(name, UNCERTAIN_VALUE.to_string()));
    }
}

/// Whether the scope walk exceeded its binding budget — the verdict entry points deny on this,
/// exactly as they do on a spent classification budget.
pub(super) fn scope_budget_spent() -> bool {
    SCOPE_SPENT.with(std::cell::Cell::get)
}

/// What the finished walk could not vouch for: the budget above was spent, or a name read as a
/// number took something else later in the line.
pub(super) fn walk_untrusted() -> bool {
    scope_budget_spent() || super::arith::numbers_contradicted()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The count the walk entered for the LAST classification on this thread. Reset only when the
    /// next top-level walk starts, so it is readable after `is_safe_command` returns.
    fn entered() -> u32 {
        SCOPE_BINDINGS.with(std::cell::Cell::get)
    }

    proptest::proptest! {
        /// However many `source` statements a generated script stacks, the walk's uncertain
        /// bindings stay proportional to the DISTINCT names it binds — one re-bind per name, never
        /// one per name per `source`. Before the dedupe the count doubled with every `source`, which
        /// is the amplification the `explain_render` OOM find rode; this pins the fix over the whole
        /// class, not just the seed's 28. The verdict and the explanation must also keep agreeing —
        /// the first property that target asserts.
        #[test]
        fn sources_never_multiply_the_binding_set(
            names in proptest::collection::vec(proptest::string::string_regex("[A-Za-z_][A-Za-z0-9_]{0,5}").unwrap(), 0..8),
            sources in 0..24usize,
        ) {
            let mut cmd = String::new();
            for n in &names {
                cmd.push_str(&format!("{n}=./a; "));
            }
            for _ in 0..sources {
                cmd.push_str(". ./e; ");
            }
            cmd.push_str("ls");
            SCOPE_BINDINGS.with(|c| c.set(0));
            let allowed = crate::is_safe_command(&cmd);
            // One sentinel per distinct bound name (plus the trusted session names `.` re-binds),
            // whatever the source count — never a multiple of it.
            proptest::prop_assert!(
                entered() <= names.len() as u32 + 8,
                "{sources} sources over {} names entered {} bindings",
                names.len(),
                entered()
            );
            proptest::prop_assert_eq!(
                allowed,
                crate::cst::explain(&cmd).is_allowed(),
                "the explanation describes a different verdict than the one enforced: {:?}",
                cmd
            );
        }
    }
}
