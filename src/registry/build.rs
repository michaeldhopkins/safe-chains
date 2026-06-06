use std::collections::HashMap;

use crate::policy::{FlagTolerance, UnknownTolerance};
use crate::verdict::SafetyLevel;

use super::types::*;

pub(super) fn build_policy(
    standalone: Vec<String>,
    valued: Vec<String>,
    bare: Option<bool>,
    max_positional: Option<usize>,
    tolerate_unknown_short: Option<bool>,
    tolerate_unknown_long: Option<bool>,
    numeric_dash: Option<bool>,
) -> OwnedPolicy {
    let unknown = match (
        tolerate_unknown_short.unwrap_or(false),
        tolerate_unknown_long.unwrap_or(false),
    ) {
        (false, false) => UnknownTolerance::Strict,
        (true, false) => UnknownTolerance::Short,
        (false, true) => UnknownTolerance::Long,
        (true, true) => UnknownTolerance::Both,
    };
    OwnedPolicy {
        standalone,
        valued,
        bare: bare.unwrap_or(true),
        max_positional,
        tolerance: FlagTolerance {
            unknown,
            numeric_dash: numeric_dash.unwrap_or(false),
        },
    }
}

fn build_matrix(toml: TomlMatrix) -> MatrixSpec {
    let actions = toml
        .actions
        .into_iter()
        .map(|(name, action)| {
            let built = match action {
                TomlMatrixAction::Policy(policy_key) => MatrixAction {
                    policy_key,
                    guard: None,
                    guard_short: None,
                },
                TomlMatrixAction::Detailed { policy, guard, guard_short } => {
                    MatrixAction { policy_key: policy, guard, guard_short }
                }
            };
            (name, built)
        })
        .collect();
    MatrixSpec {
        parents: toml.parents,
        level: toml.level.into(),
        actions,
    }
}

fn build_handler_policy(toml: TomlHandlerPolicy) -> OwnedPolicy {
    build_policy(
        toml.standalone,
        toml.valued,
        toml.bare,
        toml.max_positional,
        toml.tolerate_unknown_short,
        toml.tolerate_unknown_long,
        toml.numeric_dash,
    )
}

fn build_fallback(parent: &str, toml: TomlFallback) -> FallbackSpec {
    let policy = build_policy(
        toml.standalone,
        toml.valued,
        toml.bare,
        toml.max_positional,
        toml.tolerate_unknown_short,
        toml.tolerate_unknown_long,
        toml.numeric_dash,
    );
    let level: SafetyLevel = toml.level.unwrap_or(TomlLevel::Inert).into();
    let positional_shape = toml.positional_shape.as_deref().map(|name| {
        crate::policy::PositionalShape::from_name(name).unwrap_or_else(|| {
            panic!(
                "{}: unknown fallback positional_shape `{}` (known: path)",
                parent, name
            )
        })
    });
    FallbackSpec {
        policy,
        level,
        positional_shape,
    }
}

fn allow_all_policy() -> OwnedPolicy {
    OwnedPolicy {
        standalone: Vec::new(),
        valued: Vec::new(),
        bare: true,
        max_positional: None,
        tolerance: FlagTolerance { unknown: UnknownTolerance::Both, numeric_dash: false },
    }
}

fn check_no_legacy_positional_style(name: &str, ps: Option<bool>) {
    if ps.is_some() {
        panic!(
            "command '{name}': `positional_style` was removed. Use \
             `tolerate_unknown_short = true` for tools with single-dash \
             flags (pdftotext -help, sample -mayDie). Use \
             `tolerate_unknown_long = true` ONLY for tools whose \
             double-dash flag surface is genuinely unbounded (AWS CLI \
             style); double-dash unknowns silently pass when this is \
             on, which has caused safety bugs. Most tools need neither."
        );
    }
}

fn filter_candidates(subs: Vec<TomlSub>) -> impl Iterator<Item = TomlSub> {
    subs.into_iter().filter(|s| !s.candidate.unwrap_or(false))
}

/// Builds one SubSpec per alias (canonical name first, then each alias).
/// All entries share the same kind via Clone — the dispatcher doesn't care
/// which name the user invoked. `handler_policies` is consulted only by
/// subs that set `policy = "key"`.
pub(super) fn build_subs(
    parent: &str,
    toml: TomlSub,
    handler_policies: &std::collections::HashMap<String, OwnedPolicy>,
) -> Vec<SubSpec> {
    let aliases = toml.aliases.clone();
    let canonical = build_sub(parent, toml, handler_policies);
    let mut out = Vec::with_capacity(1 + aliases.len());
    for alias in aliases {
        out.push(SubSpec {
            name: alias,
            kind: canonical.kind.clone(),
            policy_ref: canonical.policy_ref.clone(),
            eval_safe: canonical.eval_safe,
            eval_safe_flags: canonical.eval_safe_flags.clone(),
            eval_safe_flag_values: canonical.eval_safe_flag_values.clone(),
        });
    }
    out.push(canonical);
    out
}

pub(super) fn build_sub(
    parent: &str,
    mut toml: TomlSub,
    handler_policies: &std::collections::HashMap<String, OwnedPolicy>,
) -> SubSpec {
    check_no_legacy_positional_style(&toml.name, toml.positional_style);
    let name = toml.name.clone();
    let policy_ref = toml.policy.clone();
    let eval_safe = toml.eval_safe.unwrap_or(false);
    let eval_safe_flags = std::mem::take(&mut toml.eval_safe_flags);
    let eval_safe_flag_values = std::mem::take(&mut toml.eval_safe_flag_values);
    assert_eval_safe_flags_require_tag(parent, &name, eval_safe, &eval_safe_flags);
    assert_eval_safe_flag_values_consistent(parent, &name, &eval_safe_flags, &eval_safe_flag_values);
    assert_sub_eval_safe_only_on_leaf(parent, &toml);
    SubSpec {
        name,
        kind: build_sub_kind(parent, toml, handler_policies),
        policy_ref,
        eval_safe,
        eval_safe_flags,
        eval_safe_flag_values,
    }
}

fn assert_eval_safe_flag_values_consistent(
    parent: &str,
    name: &str,
    eval_safe_flags: &[String],
    eval_safe_flag_values: &std::collections::HashMap<String, Vec<String>>,
) {
    for (flag, values) in eval_safe_flag_values {
        if !eval_safe_flags.iter().any(|f| f == flag) {
            panic!(
                "command '{parent}' sub `{name}` lists `{flag}` in \
                 `eval_safe_flag_values` but not in `eval_safe_flags`. \
                 A value allowlist only takes effect when the flag itself \
                 is allowed. Add `{flag}` to `eval_safe_flags` or remove \
                 the value entry."
            );
        }
        for value in values {
            if value.is_empty() || !value.chars().all(is_bare_literal_char) {
                panic!(
                    "command '{parent}' sub `{name}` has eval_safe_flag_values \
                     for `{flag}` containing value `{value:?}` with characters \
                     outside `[a-zA-Z0-9_./=-]`. The allowed-value set must \
                     itself be bare-literal so it can never embed a shell-\
                     expansion trigger into the substituted invocation."
                );
            }
        }
    }
}

fn is_bare_literal_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '=')
}

fn assert_eval_safe_flags_require_tag(parent: &str, name: &str, eval_safe: bool, flags: &[String]) {
    if !flags.is_empty() && !eval_safe {
        panic!(
            "command '{parent}' sub `{name}` declares `eval_safe_flags` without \
             `eval_safe = true`. The flag allowlist only takes effect when the \
             sub is tagged eval-safe. Add `eval_safe = true` or drop \
             `eval_safe_flags`."
        );
    }
}

fn assert_sub_eval_safe_only_on_leaf(parent: &str, toml: &TomlSub) {
    if toml.eval_safe != Some(true) {
        return;
    }
    if !toml.sub.is_empty() {
        panic!(
            "command '{parent}' sub `{}` sets `eval_safe = true` AND has \
             nested [[command.sub.sub]] blocks. eval_safe must be tagged on \
             a leaf node — move the tag onto the specific sub-sub that emits \
             shell-init code. Otherwise the walker accepts any unmatched \
             sub-sub name as a positional, which is counter-intuitive.",
            toml.name,
        );
    }
    if toml.handler.is_some() {
        panic!(
            "command '{parent}' sub `{}` sets `eval_safe = true` AND \
             `handler = \"...\"`. Handler-based subs run Rust dispatch logic \
             whose shape the eval walker cannot introspect — eval-safety \
             requires a declarative leaf the registry can reason about.",
            toml.name,
        );
    }
    if toml.delegate_after.is_some() || toml.delegate_skip.is_some() {
        panic!(
            "command '{parent}' sub `{}` sets `eval_safe = true` AND \
             delegates to an inner command (delegate_after / delegate_skip). \
             The inner command's output is unrelated to this sub's vetting \
             — drop `eval_safe`.",
            toml.name,
        );
    }
}

fn assert_eval_safe_tagged_command_has_researched_version(toml: &TomlCommand) {
    let command_tagged = toml.eval_safe == Some(true);
    let any_sub_tagged = toml_has_any_eval_safe_sub(&toml.sub);
    if !command_tagged && !any_sub_tagged {
        return;
    }
    if toml.researched_version.is_none() {
        panic!(
            "command '{}' has `eval_safe = true` (on the command or a sub) \
             but no `researched_version`. eval-safe tags pin a per-tag trust \
             claim against a specific upstream snapshot — add the version \
             you researched (e.g. `researched_version = \"v2026.5.3\"`) so \
             the next contributor knows what to diff against.",
            toml.name,
        );
    }
}

fn toml_has_any_eval_safe_sub(subs: &[TomlSub]) -> bool {
    subs.iter().any(|s| s.eval_safe == Some(true) || toml_has_any_eval_safe_sub(&s.sub))
}

fn assert_command_eval_safe_only_on_leaf(toml: &TomlCommand) {
    if toml.eval_safe != Some(true) {
        return;
    }
    if !toml.sub.is_empty() {
        panic!(
            "command '{}' sets `eval_safe = true` at the command level AND \
             has [[command.sub]] blocks. Move the tag onto the specific \
             sub that emits shell-init code (e.g. `mise activate`) — \
             command-level tagging on a structured command is counter-\
             intuitive.",
            toml.name,
        );
    }
    if toml.handler.is_some() {
        panic!(
            "command '{}' sets `eval_safe = true` AND `handler = \"...\"`. \
             Handler-based commands run Rust dispatch logic whose shape the \
             eval walker cannot introspect.",
            toml.name,
        );
    }
    if toml.wrapper.is_some() {
        panic!(
            "command '{}' sets `eval_safe = true` AND `[command.wrapper]`. \
             Wrappers forward to an inner command — tagging the wrapper \
             would tag every wrapped invocation. Drop `eval_safe`.",
            toml.name,
        );
    }
    if toml.deny.unwrap_or(false) {
        panic!(
            "command '{}' sets both `deny = true` and `eval_safe = true`. \
             These are contradictory — deny silently dominates. Drop one.",
            toml.name,
        );
    }
}

fn build_sub_kind(
    parent: &str,
    toml: TomlSub,
    handler_policies: &std::collections::HashMap<String, OwnedPolicy>,
) -> DispatchKind {
    if let Some(handler_name) = toml.handler {
        return DispatchKind::Custom {
            handler_name,
            doc_body: toml.doc_body,
            subs: Vec::new(),
            fallback: None,
            handler_policies: std::collections::HashMap::new(),
            matrices: Vec::new(),
        };
    }
    if toml.allow_all.unwrap_or(false) {
        return DispatchKind::Policy {
            policy: allow_all_policy(),
            level: toml.level.unwrap_or(TomlLevel::Inert).into(),
        };
    }
    if let Some(sep) = toml.delegate_after {
        return DispatchKind::DelegateAfterSeparator { separator: sep };
    }
    if let Some(skip) = toml.delegate_skip {
        return DispatchKind::DelegateSkip { skip };
    }
    if !toml.sub.is_empty() {
        return DispatchKind::Branching {
            subs: filter_candidates(toml.sub)
                .flat_map(|s| build_subs(parent, s, handler_policies))
                .collect(),
            bare_flags: Vec::new(),
            bare_ok: toml.nested_bare.unwrap_or(false),
            pre_standalone: toml.standalone,
            pre_valued: toml.valued,
            first_arg: Vec::new(),
            first_arg_level: SafetyLevel::Inert,
        };
    }
    build_policy_sub_kind(parent, toml, handler_policies)
}

fn build_policy_sub_kind(
    parent: &str,
    toml: TomlSub,
    handler_policies: &std::collections::HashMap<String, OwnedPolicy>,
) -> DispatchKind {
    let policy = if let Some(key) = &toml.policy {
        if !toml.standalone.is_empty() || !toml.valued.is_empty() {
            panic!(
                "command '{parent}' sub `{}` sets both `policy = \"{}\"` and \
                 inline standalone/valued — pick one. Either drop the inline \
                 lists (and rely on the referenced handler_policy) or drop \
                 the `policy` field.",
                toml.name, key,
            );
        }
        handler_policies.get(key).cloned().unwrap_or_else(|| {
            panic!(
                "command '{parent}' sub `{}` references handler_policy \
                 `{key}` which is not declared. Add a \
                 [command.handler_policy.{key}] block or fix the typo.",
                toml.name,
            )
        })
    } else {
        build_policy(
            toml.standalone,
            toml.valued,
            toml.bare,
            toml.max_positional,
            toml.tolerate_unknown_short,
            toml.tolerate_unknown_long,
            toml.numeric_dash,
        )
    };
    let level: SafetyLevel = toml.level.unwrap_or(TomlLevel::Inert).into();
    if !toml.write_flags.is_empty() {
        return DispatchKind::WriteFlagged {
            policy,
            base_level: level,
            write_flags: toml.write_flags,
        };
    }
    if let Some(guard) = toml.guard {
        let mut require_any = vec![guard];
        if let Some(short) = toml.guard_short {
            require_any.push(short);
        }
        return DispatchKind::RequireAny {
            require_any,
            policy,
            level,
            accept_bare_help: true,
        };
    }
    if !toml.first_arg.is_empty() {
        return DispatchKind::FirstArg { patterns: toml.first_arg, level };
    }
    if !toml.require_any.is_empty() {
        return DispatchKind::RequireAny {
            require_any: toml.require_any,
            policy,
            level,
            accept_bare_help: false,
        };
    }
    DispatchKind::Policy { policy, level }
}

/// Diagnostic for a configuration class that silently breaks flag dispatch:
/// a structured command (with `[[command.sub]]` blocks) cannot also use the
/// flat-style top-level fields. When subs are present, top-level standalone/
/// valued/max_positional/positional_style/numeric_dash are dropped — the
/// dispatch routes through the Branching path. The fix is to either remove
/// the subs (if the command is meant to be flat) or move global flags into
/// a `[command.wrapper]` block.
fn assert_flat_or_structured(toml: &TomlCommand) {
    if toml.sub.is_empty() {
        return;
    }
    let mut conflicts = Vec::new();
    if !toml.standalone.is_empty() {
        conflicts.push("standalone");
    }
    if !toml.valued.is_empty() {
        conflicts.push("valued");
    }
    if toml.max_positional.is_some() {
        conflicts.push("max_positional");
    }
    if toml.tolerate_unknown_short.is_some() {
        conflicts.push("tolerate_unknown_short");
    }
    if toml.tolerate_unknown_long.is_some() {
        conflicts.push("tolerate_unknown_long");
    }
    if toml.numeric_dash.is_some() {
        conflicts.push("numeric_dash");
    }
    if !conflicts.is_empty() {
        panic!(
            "command '{}' mixes flat-style top-level fields ({}) with [[command.sub]] blocks. \
             When subs are present these fields are silently dropped. \
             Either drop the subs (if the command is flat) or move global \
             flags into a [command.wrapper] block.",
            toml.name,
            conflicts.join(", "),
        );
    }
}

fn assert_matrix_policy_keys_exist(toml: &TomlCommand) {
    if toml.matrix.is_empty() {
        return;
    }
    for matrix in &toml.matrix {
        for (action_name, action) in &matrix.actions {
            let policy_key = match action {
                TomlMatrixAction::Policy(k) => k,
                TomlMatrixAction::Detailed { policy, .. } => policy,
            };
            if !toml.handler_policy.contains_key(policy_key) {
                panic!(
                    "command '{}' matrix action `{}` references \
                     handler_policy `{}` which is not declared. \
                     Add a [command.handler_policy.{}] block or fix the typo.",
                    toml.name, action_name, policy_key, policy_key,
                );
            }
        }
    }
}

fn assert_matrix_no_duplicate_parent_action(toml: &TomlCommand) {
    if toml.matrix.len() < 2 {
        return;
    }
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for matrix in &toml.matrix {
        for parent in &matrix.parents {
            for action in matrix.actions.keys() {
                let key = (parent.clone(), action.clone());
                if !seen.insert(key) {
                    panic!(
                        "command '{}' matrix has duplicate (parent, action) pair \
                         (`{}`, `{}`). The first match would silently win — \
                         consolidate into one matrix block or remove the duplicate.",
                        toml.name, parent, action,
                    );
                }
            }
        }
    }
}

fn assert_fallback_requires_handler(toml: &TomlCommand) {
    if toml.fallback.is_some() && toml.handler.is_none() {
        panic!(
            "command '{}' declares [command.fallback] without a handler. \
             Fallback grammars are only consulted via \
             registry::try_fallback_grammar() from a Rust handler — without \
             handler = \"...\" the block is silently dropped. \
             Either set handler or remove [command.fallback].",
            toml.name,
        );
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn build_command(toml: TomlCommand, category: &str) -> CommandSpec {
    assert_flat_or_structured(&toml);
    assert_fallback_requires_handler(&toml);
    assert_matrix_policy_keys_exist(&toml);
    assert_matrix_no_duplicate_parent_action(&toml);
    assert_command_eval_safe_only_on_leaf(&toml);
    assert_eval_safe_tagged_command_has_researched_version(&toml);
    check_no_legacy_positional_style(&toml.name, toml.positional_style);
    let cat = category.to_string();
    let desc = toml.description.unwrap_or_default();
    let researched_version = toml.researched_version;
    let examples_safe = toml.examples_safe;
    let examples_denied = toml.examples_denied;
    let eval_safe = toml.eval_safe.unwrap_or(false);
    let eval_safe_flags = toml.eval_safe_flags;
    let eval_safe_flag_values = toml.eval_safe_flag_values;
    if !eval_safe_flags.is_empty() && !eval_safe {
        panic!(
            "command '{}' declares `eval_safe_flags` without `eval_safe = true`. \
             The flag allowlist only takes effect when the command is tagged \
             eval-safe. Add `eval_safe = true` or drop `eval_safe_flags`.",
            toml.name,
        );
    }
    assert_eval_safe_flag_values_consistent(
        &toml.name,
        "<command>",
        &eval_safe_flags,
        &eval_safe_flag_values,
    );
    if toml.deny.unwrap_or(false) {
        return CommandSpec {
            name: toml.name,
            description: desc,
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
            researched_version,
            examples_safe,
            examples_denied,
            eval_safe,
            eval_safe_flags: eval_safe_flags.clone(),
            eval_safe_flag_values: eval_safe_flag_values.clone(),
            kind: DispatchKind::Policy {
                policy: OwnedPolicy {
                    standalone: Vec::new(),
                    valued: Vec::new(),
                    bare: false,
                    max_positional: Some(0),
                    tolerance: FlagTolerance::default(),
                },
                level: SafetyLevel::Inert,
            },
        };
    }
    if let Some(handler_name) = toml.handler {
        // Build handler_policies first so subs that use `policy = "key"`
        // can resolve the reference at build time.
        let handler_policies: std::collections::HashMap<String, OwnedPolicy> = toml
            .handler_policy
            .into_iter()
            .map(|(k, v)| (k, build_handler_policy(v)))
            .collect();
        let parent_name = toml.name.clone();
        let subs = filter_candidates(toml.sub)
            .flat_map(|s| build_subs(&parent_name, s, &handler_policies))
            .collect();
        let fallback = toml.fallback.map(|f| build_fallback(&toml.name, f));
        let matrices = toml
            .matrix
            .into_iter()
            .map(build_matrix)
            .collect();
        return CommandSpec {
            name: toml.name,
            description: desc,
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
            researched_version,
            examples_safe,
            examples_denied,
            eval_safe,
            eval_safe_flags: eval_safe_flags.clone(),
            eval_safe_flag_values: eval_safe_flag_values.clone(),
            kind: DispatchKind::Custom {
                handler_name,
                doc_body: toml.doc_body,
                subs,
                fallback,
                handler_policies,
                matrices,
            },
        };
    }

    if let Some(w) = toml.wrapper {
        if !toml.sub.is_empty() || !toml.bare_flags.is_empty() {
            let first_arg_level = toml.level.unwrap_or(TomlLevel::Inert).into();
            let parent_name = toml.name.clone();
            return CommandSpec {
                name: toml.name,
                description: desc,
                aliases: toml.aliases,
                url: toml.url,
                category: cat,
                researched_version,
                examples_safe,
                examples_denied,
                eval_safe,
                eval_safe_flags: eval_safe_flags.clone(),
                eval_safe_flag_values: eval_safe_flag_values.clone(),
                kind: DispatchKind::Branching {
                    bare_flags: toml.bare_flags,
                    subs: filter_candidates(toml.sub)
                        .flat_map(|s| build_subs(&parent_name, s, &std::collections::HashMap::new()))
                        .collect(),
                    pre_standalone: w.standalone,
                    pre_valued: w.valued,
                    bare_ok: toml.bare.unwrap_or(false),
                    first_arg: toml.first_arg,
                    first_arg_level,
                },
            };
        }
        return CommandSpec {
            name: toml.name,
            description: desc,
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
            researched_version,
            examples_safe,
            examples_denied,
            eval_safe,
            eval_safe_flags: eval_safe_flags.clone(),
            eval_safe_flag_values: eval_safe_flag_values.clone(),
            kind: DispatchKind::Wrapper {
                standalone: w.standalone,
                valued: w.valued,
                positional_skip: w.positional_skip.unwrap_or(0),
                separator: w.separator,
                bare_ok: w.bare_ok.unwrap_or(false),
            },
        };
    }

    if !toml.sub.is_empty() || !toml.bare_flags.is_empty() {
        let first_arg_level = toml.level.unwrap_or(TomlLevel::Inert).into();
        let parent_name = toml.name.clone();
        return CommandSpec {
            name: toml.name,
            description: desc,
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
            researched_version,
            examples_safe,
            examples_denied,
            eval_safe,
            eval_safe_flags: eval_safe_flags.clone(),
            eval_safe_flag_values: eval_safe_flag_values.clone(),
            kind: DispatchKind::Branching {
                bare_flags: toml.bare_flags,
                subs: filter_candidates(toml.sub)
                    .flat_map(|s| build_subs(&parent_name, s, &std::collections::HashMap::new()))
                    .collect(),
                pre_standalone: Vec::new(),
                pre_valued: Vec::new(),
                bare_ok: toml.bare.unwrap_or(false),
                first_arg: toml.first_arg,
                first_arg_level,
            },
        };
    }

    let policy = build_policy(
        toml.standalone,
        toml.valued,
        toml.bare,
        toml.max_positional,
        toml.tolerate_unknown_short,
        toml.tolerate_unknown_long,
        toml.numeric_dash,
    );

    let level = toml.level.unwrap_or(TomlLevel::Inert).into();

    if !toml.first_arg.is_empty() {
        return CommandSpec {
            name: toml.name,
            description: desc,
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
            researched_version,
            examples_safe,
            examples_denied,
            eval_safe,
            eval_safe_flags: eval_safe_flags.clone(),
            eval_safe_flag_values: eval_safe_flag_values.clone(),
            kind: DispatchKind::FirstArg {
                patterns: toml.first_arg,
                level,
            },
        };
    }

    if !toml.write_flags.is_empty() {
        return CommandSpec {
            name: toml.name,
            description: desc,
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
            researched_version,
            examples_safe,
            examples_denied,
            eval_safe,
            eval_safe_flags: eval_safe_flags.clone(),
            eval_safe_flag_values: eval_safe_flag_values.clone(),
            kind: DispatchKind::WriteFlagged {
                policy,
                base_level: level,
                write_flags: toml.write_flags,
            },
        };
    }

    if !toml.require_any.is_empty() {
        return CommandSpec {
            name: toml.name,
            description: desc,
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
            researched_version,
            examples_safe,
            examples_denied,
            eval_safe,
            eval_safe_flags: eval_safe_flags.clone(),
            eval_safe_flag_values: eval_safe_flag_values.clone(),
            kind: DispatchKind::RequireAny {
                require_any: toml.require_any,
                policy,
                level,
                accept_bare_help: false,
            },
        };
    }

    CommandSpec {
        name: toml.name,
        description: desc,
        aliases: toml.aliases,
        url: toml.url,
        category: cat,
        researched_version,
        examples_safe,
        examples_denied,
        eval_safe,
        eval_safe_flags,
        eval_safe_flag_values,
        kind: DispatchKind::Policy {
            policy,
            level,
        },
    }
}

pub fn load_toml(source: &str, category: &str) -> Vec<CommandSpec> {
    let file: TomlFile = match toml::from_str(source) {
        Ok(f) => f,
        Err(e) => {
            let preview: String = source.chars().take(80).collect();
            panic!("invalid TOML command definition: {e}\n  source begins: {preview}");
        }
    };
    file.command.into_iter()
        .filter(|cmd| !cmd.candidate.unwrap_or(false))
        .map(|cmd| build_command(cmd, category))
        .collect()
}

pub fn build_registry(specs: Vec<CommandSpec>) -> HashMap<String, CommandSpec> {
    let mut map = HashMap::new();
    for spec in specs {
        insert_spec(&mut map, spec);
    }
    map
}

/// Insert a CommandSpec into the registry, registering both its canonical
/// name and each alias. Existing entries for the same command name are
/// removed first, so a custom-TOML override of `gh` replaces every
/// built-in alias of `gh` rather than leaving stale aliases pointing at
/// the old spec.
pub fn insert_spec(map: &mut HashMap<String, CommandSpec>, spec: CommandSpec) {
    map.retain(|_, s| s.name != spec.name);
    for alias in &spec.aliases {
        map.insert(alias.clone(), CommandSpec {
            name: spec.name.clone(),
            description: spec.description.clone(),
            aliases: vec![],
            url: spec.url.clone(),
            category: spec.category.clone(),
            researched_version: spec.researched_version.clone(),
            examples_safe: vec![],
            examples_denied: vec![],
            eval_safe: spec.eval_safe,
            eval_safe_flags: spec.eval_safe_flags.clone(),
            eval_safe_flag_values: spec.eval_safe_flag_values.clone(),
            kind: spec.kind.clone(),
        });
    }
    map.insert(spec.name.clone(), spec);
}
