use super::{GATES, Role};
use crate::verdict::Verdict;

/// What the ROLE's judge says about `value` for a declared `cmd`/`flag` gate, or `None` when that
/// flag declares no gate.
///
/// Exposed for the `gate_prefilter` fuzz target, which asserts the one invariant the pre-filter can
/// break: a value the judge refuses must not be skipped before the judge ever sees it. Deliberately
/// returns the JUDGE's answer rather than the gate's, so the two can be compared.
///
/// `doc(hidden)` for the same reason as `registry::fuzz_load_config`: the fuzz target is a separate
/// crate so this must be `pub`, but this crate publishes to crates.io and a test seam is not API.
#[doc(hidden)]
pub fn judge_for_flag(cmd: &str, flag: &str, value: &str) -> Option<Verdict> {
    let role = GATES
        .roles
        .get(cmd)
        .and_then(|spec| spec.role_of(flag))
        .or_else(|| crate::registry::command_path_gate(cmd)?.role_of(flag))?;
    Some(match role {
        Role::Ignore => return None,
        Role::Read => crate::engine::resolve::read_content_verdict(value),
        Role::ReadTree => crate::engine::resolve::read_tree_verdict(value),
        Role::Write => crate::engine::resolve::write_target_verdict(value),
        Role::Exec => crate::engine::resolve::execute_file_verdict(value),
    })
}

/// What the POSITIONAL role's judge says about `value` for `cmd`, or `None` when the command
/// declares no positional role (or declares `ignore`).
///
/// The positional companion to [`judge_for_flag`], for the same fuzz target. The target still skips
/// flag-shaped values here, because `walk` peels those off before a token is treated as a
/// positional at all — feeding one in would test a path the real code never takes.
#[doc(hidden)]
pub fn judge_for_positional(cmd: &str, value: &str) -> Option<Verdict> {
    let role = GATES
        .roles
        .get(cmd)
        .map(|spec| spec.positional)
        .or_else(|| crate::registry::command_path_gate(cmd).map(|spec| spec.positional))?;
    match role {
        Role::Ignore => None,
        Role::Read => Some(crate::engine::resolve::read_content_verdict(value)),
        Role::ReadTree => Some(crate::engine::resolve::read_tree_verdict(value)),
        Role::Write => Some(crate::engine::resolve::write_target_verdict(value)),
        Role::Exec => Some(crate::engine::resolve::execute_file_verdict(value)),
    }
}
