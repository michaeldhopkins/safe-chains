//! The build-time checks on `eval_safe` tags, out of `build.rs`. A child of `build`, whose `fail!` it uses.

use super::super::types::*;

pub(super) fn assert_eval_safe_flag_values_consistent(
    parent: &str,
    name: &str,
    eval_safe_flags: &[String],
    eval_safe_flag_values: &std::collections::HashMap<String, Vec<String>>,
) -> Result<(), String> {
    for (flag, values) in eval_safe_flag_values {
        if !eval_safe_flags.iter().any(|f| f == flag) {
            fail!(
                "command '{parent}' sub `{name}` lists `{flag}` in \
                 `eval_safe_flag_values` but not in `eval_safe_flags`. \
                 A value allowlist only takes effect when the flag itself \
                 is allowed. Add `{flag}` to `eval_safe_flags` or remove \
                 the value entry."
            );
        }
        for value in values {
            if value.is_empty() || !value.chars().all(is_bare_literal_char) {
                fail!(
                    "command '{parent}' sub `{name}` has eval_safe_flag_values \
                     for `{flag}` containing value `{value:?}` with characters \
                     outside `[a-zA-Z0-9_./=-]`. The allowed-value set must \
                     itself be bare-literal so it can never embed a shell-\
                     expansion trigger into the substituted invocation."
                );
            }
        }
    }
    Ok(())
}

fn is_bare_literal_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '=')
}

pub(super) fn assert_eval_safe_required_flags_consistent(
    parent: &str,
    name: &str,
    eval_safe_flags: &[String],
    eval_safe_required_flags: &[String],
) -> Result<(), String> {
    for flag in eval_safe_required_flags {
        if !eval_safe_flags.iter().any(|f| f == flag) {
            fail!(
                "command '{parent}' sub `{name}` lists `{flag}` in \
                 `eval_safe_required_flags` but not in `eval_safe_flags`. \
                 A required-flag constraint must be a subset of the \
                 allowed-flag set — otherwise the flag is required AND \
                 immediately denied. Add `{flag}` to `eval_safe_flags` or \
                 remove it from `eval_safe_required_flags`."
            );
        }
    }
    Ok(())
}

/// Every valued flag in `eval_safe_flags` must declare its value
/// posture in `eval_safe_flag_values`: either a concrete value
/// allowlist (`["env", "fish"]`) OR the explicit-unrestricted form
/// (`[]`). Without this, a contributor adding a short alias like
/// `-f` for an already-tagged `--format` would silently widen the
/// eval-safe surface to any value of `-f` — the v0.196.0 aws
/// near-miss in disguise.
pub(super) fn assert_eval_safe_valued_flags_declared(
    parent: &str,
    name: &str,
    eval_safe_flags: &[String],
    valued: &[String],
    eval_safe_flag_values: &std::collections::HashMap<String, Vec<String>>,
) -> Result<(), String> {
    for flag in eval_safe_flags {
        if !valued.iter().any(|v| v == flag) {
            continue;
        }
        if !eval_safe_flag_values.contains_key(flag) {
            fail!(
                "command '{parent}' sub `{name}` lists `{flag}` in \
                 `eval_safe_flags` AND in `valued`, but `{flag}` has no \
                 entry in `eval_safe_flag_values`. Every valued flag \
                 tagged eval-safe must declare its value posture: \
                 either a concrete allowlist of safe values \
                 (`{flag} = [\"value-a\", \"value-b\"]`) or the \
                 explicit-unrestricted form (`{flag} = []`) signaling \
                 the contributor vetted that any value preserves shell-\
                 init output. Omitting an entry means the walker can't \
                 tell whether the value-following-flag is supposed to \
                 be checked, and a future short alias of `{flag}` \
                 could silently widen the eval-safe surface."
            );
        }
    }
    Ok(())
}

pub(super) fn assert_eval_safe_flags_require_tag(parent: &str, name: &str, eval_safe: bool, flags: &[String]) -> Result<(), String> {
    if !flags.is_empty() && !eval_safe {
        fail!(
            "command '{parent}' sub `{name}` declares `eval_safe_flags` without \
             `eval_safe = true`. The flag allowlist only takes effect when the \
             sub is tagged eval-safe. Add `eval_safe = true` or drop \
             `eval_safe_flags`."
        );
    }
    Ok(())
}

pub(super) fn assert_sub_eval_safe_only_on_leaf(parent: &str, toml: &TomlSub) -> Result<(), String> {
    if toml.eval_safe != Some(true) {
        return Ok(());
    }
    if !toml.sub.is_empty() {
        fail!(
            "command '{parent}' sub `{}` sets `eval_safe = true` AND has \
             nested [[command.sub.sub]] blocks. eval_safe must be tagged on \
             a leaf node — move the tag onto the specific sub-sub that emits \
             shell-init code. Otherwise the walker accepts any unmatched \
             sub-sub name as a positional, which is counter-intuitive.",
            toml.name,
        );
    }
    if toml.handler.is_some() {
        fail!(
            "command '{parent}' sub `{}` sets `eval_safe = true` AND \
             `handler = \"...\"`. Handler-based subs run Rust dispatch logic \
             whose shape the eval walker cannot introspect — eval-safety \
             requires a declarative leaf the registry can reason about.",
            toml.name,
        );
    }
    if toml.delegate_after.is_some() || toml.delegate_skip.is_some() {
        fail!(
            "command '{parent}' sub `{}` sets `eval_safe = true` AND \
             delegates to an inner command (delegate_after / delegate_skip). \
             The inner command's output is unrelated to this sub's vetting \
             — drop `eval_safe`.",
            toml.name,
        );
    }
    Ok(())
}

pub(super) fn assert_eval_safe_tagged_command_has_researched_version(toml: &TomlCommand) -> Result<(), String> {
    let command_tagged = toml.eval_safe == Some(true);
    let any_sub_tagged = toml_has_any_eval_safe_sub(&toml.sub);
    if !command_tagged && !any_sub_tagged {
        return Ok(());
    }
    if toml.researched_version.is_none() {
        fail!(
            "command '{}' has `eval_safe = true` (on the command or a sub) \
             but no `researched_version`. eval-safe tags pin a per-tag trust \
             claim against a specific upstream snapshot — add the version \
             you researched (e.g. `researched_version = \"v2026.5.3\"`) so \
             the next contributor knows what to diff against.",
            toml.name,
        );
    }
    Ok(())
}

fn toml_has_any_eval_safe_sub(subs: &[TomlSub]) -> bool {
    subs.iter().any(|s| s.eval_safe == Some(true) || toml_has_any_eval_safe_sub(&s.sub))
}

pub(super) fn assert_command_eval_safe_only_on_leaf(toml: &TomlCommand) -> Result<(), String> {
    if toml.eval_safe != Some(true) {
        return Ok(());
    }
    if !toml.sub.is_empty() {
        fail!(
            "command '{}' sets `eval_safe = true` at the command level AND \
             has [[command.sub]] blocks. Move the tag onto the specific \
             sub that emits shell-init code (e.g. `mise activate`) — \
             command-level tagging on a structured command is counter-\
             intuitive.",
            toml.name,
        );
    }
    // Command-level eval_safe IS allowed alongside `handler = "..."`: the walker reads
    // `spec.eval_safe` at the leaf, and the contributor vouches that every invocation the handler
    // accepts AND that passes the eval_safe_* flag checks prints shell-init stdout (typically
    // narrowed via `eval_safe_required_flags`; see fzf's TOML).
    if toml.wrapper.is_some() {
        fail!(
            "command '{}' sets `eval_safe = true` AND `[command.wrapper]`. \
             Wrappers forward to an inner command — tagging the wrapper \
             would tag every wrapped invocation. Drop `eval_safe`.",
            toml.name,
        );
    }
    if toml.deny.unwrap_or(false) {
        fail!(
            "command '{}' sets both `deny = true` and `eval_safe = true`. \
             These are contradictory — deny silently dominates. Drop one.",
            toml.name,
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashMap;

    fn values_for(flag: &str, values: Vec<String>) -> (Vec<String>, HashMap<String, Vec<String>>) {
        (vec![flag.to_string()], HashMap::from([(flag.to_string(), values)]))
    }

    proptest! {
        /// A value allowlist built only from bare-literal characters is accepted.
        #[test]
        fn bare_literal_eval_safe_values_are_accepted(values in proptest::collection::vec("[a-zA-Z0-9_./=-]{1,8}", 0..4)) {
            let (flags, map) = values_for("--shell", values);
            prop_assert!(assert_eval_safe_flag_values_consistent("cmd", "sub", &flags, &map).is_ok());
        }

        /// One character outside the bare-literal set in any value rejects the allowlist.
        #[test]
        fn a_non_literal_eval_safe_value_is_rejected(prefix in "[a-z]{0,4}", bad in "[ $`;&|<>(){}*?~\"']", suffix in "[a-z]{0,4}") {
            let (flags, map) = values_for("--shell", vec![format!("{prefix}{bad}{suffix}")]);
            prop_assert!(assert_eval_safe_flag_values_consistent("cmd", "sub", &flags, &map).is_err());
        }
    }
}
