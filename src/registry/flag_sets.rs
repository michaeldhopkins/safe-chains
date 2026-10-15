//! The flag sets a handler-dispatched command keeps in TOML, for handlers that walk their own
//! grammar and only need the sets.

use super::handler_spec;
use super::types::DispatchKind;

/// The flag vocabulary a handler-dispatched command keeps in `[command.fallback]`, for handlers
/// that walk their own grammar and only need the SETS.
///
/// find is the case this exists for: its expression walk is genuine logic (a valued primary
/// consumes its value, `-newer*` matches by prefix, `-exec`/`-delete` delegate against the
/// traversal bases), but the two vocabularies it consults are flag lists, and flag lists belong in
/// TOML. Returning the slices rather than a verdict keeps the walk in the handler where it belongs
/// while the DATA lives in one place — which is the whole point: there is no second copy to drift
/// from, because the `WordSet` constants were deleted rather than kept in sync.
///
/// Linear scan, and deliberately so: `impl FlagSet for [String]` is `iter().any(..)`, so unlike
/// `WordSet` there is no sorted-order precondition to preserve. Ordering travels with the data
/// structure, not the data.
pub(crate) fn fallback_flag_sets(cmd_name: &str) -> Option<(&'static [String], &'static [String])> {
    let spec = handler_spec(cmd_name)?;
    let DispatchKind::Custom { fallback, .. } = &spec.kind else {
        return None;
    };
    let f = fallback.as_ref()?;
    Some((f.policy.standalone.as_slice(), f.policy.valued.as_slice()))
}

pub(crate) fn handler_policy_flag_sets(cmd_name: &str, key: &str) -> Option<(&'static [String], &'static [String])> {
    let spec = handler_spec(cmd_name)?;
    let DispatchKind::Custom { handler_policies, .. } = &spec.kind else {
        return None;
    };
    let policy = handler_policies.get(key)?;
    Some((policy.standalone.as_slice(), policy.valued.as_slice()))
}
