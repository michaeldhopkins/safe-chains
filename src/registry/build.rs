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
                TomlMatrixAction::Detailed(d) => {
                    MatrixAction {
                        policy_key: d.policy,
                        guard: d.guard,
                        guard_short: d.guard_short,
                    }
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

fn build_verb_chain(toml: TomlVerbChain) -> VerbChainSpec {
    VerbChainSpec {
        level: toml.level.unwrap_or(TomlLevel::Inert).into(),
        separator: toml.separator.unwrap_or_else(|| "then".to_string()),
        main_standalone: toml.main_standalone,
        main_valued: toml.main_valued,
        main_variadic: toml.main_variadic,
        verbs: toml.verbs.into_iter().collect(),
    }
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
    let executor = toml.executor.as_deref().map(|name| {
        ExecutorKind::from_name(name)
            .unwrap_or_else(|| panic!("{parent}: unknown fallback executor `{name}` (known: file, project)"))
    });
    FallbackSpec {
        policy,
        level,
        positional_shape,
        executor,
        executor_redirect_flag: toml.executor_redirect_flag,
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

/// Whether `name` is matched by a `first_arg` pattern (`get-*` prefix glob, or an exact token).
fn first_arg_matches(name: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| match p.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => p == name,
    })
}

/// Fail-closed guard for the `candidate`-under-glob footgun. A `candidate = true` sub is REMOVED from
/// the registry (so an older client sees "not found" and denies) — but if a sibling `first_arg` glob
/// would MATCH that removed name, the token falls through to the glob and AUTO-APPROVES, silently
/// inverting the author's intent (a deny becomes an allow). This panics at load when it detects that
/// shape, directing the author to a `profile`/explicit sub-sub instead. (The AWS blob-readers hit
/// exactly this: a `candidate` under `get-*` would have auto-approved.)
fn assert_no_candidate_shadowed_by_glob(parent: &str, subs: &[TomlSub], first_arg: &[String]) {
    if first_arg.is_empty() {
        return;
    }
    for s in subs {
        if s.candidate.unwrap_or(false) && first_arg_matches(&s.name, first_arg) {
            panic!(
                "'{parent}' sub `{}` is `candidate = true` but its name is MATCHED by the sibling \
                 first_arg glob {first_arg:?} — it would fall through the filter and AUTO-APPROVE \
                 (a silent deny→allow inversion). Use `profile`/an explicit sub-sub to deny it, or \
                 drop it from the glob.",
                s.name,
            );
        }
    }
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
            profile: canonical.profile.clone(),
            flags: canonical.flags.clone(),
            unless_flags: canonical.unless_flags.clone(),
            eval_safe: canonical.eval_safe,
            eval_safe_flags: canonical.eval_safe_flags.clone(),
            eval_safe_flag_values: canonical.eval_safe_flag_values.clone(),
            eval_safe_required_flags: canonical.eval_safe_required_flags.clone(),
            network_destination: canonical.network_destination,
            destination_flag: canonical.destination_flag.clone(),
            output_path_flags: canonical.output_path_flags.clone(),
        });
    }
    out.push(canonical);
    out
}

/// Enforce the research standard at build time (reading the TOML fields, so provenance is a
/// production-validated part of the tree, not dead metadata). A sub with a `profile` must name a
/// real archetype and cite a `fact` + `source`; each escalating `[[command.sub.flag]]` must name a
/// real archetype (or the `unclassified` fail-closed escape) and cite its own `fact` + `source`. A
/// mis-authored classification fails CLOSED — the registry panics at load rather than silently
/// under-recording why a subcommand sits above the auto-approve line.
fn assert_sub_provenance(parent: &str, toml: &TomlSub) {
    let cited = |o: &Option<String>| o.as_deref().is_some_and(|s| !s.trim().is_empty());
    // A `judgment` is optional (the discretionary layer), but if given it must say something.
    let judged = |o: &Option<String>| o.as_deref().is_none_or(|s| !s.trim().is_empty());
    if let Some(p) = &toml.profile {
        assert!(
            crate::engine::archetype::archetype(p).is_some(),
            "{parent} sub `{}`: profile `{p}` is not a known archetype (archetypes.toml)",
            toml.name,
        );
        assert!(cited(&toml.fact), "{parent} sub `{}`: `profile` requires a `fact`", toml.name);
        assert!(cited(&toml.source), "{parent} sub `{}`: `profile` requires a `source`", toml.name);
        assert!(judged(&toml.judgment), "{parent} sub `{}`: `judgment`, if given, must not be blank", toml.name);
        // A profiled sub is a leaf: the engine classifies it by archetype, and its legacy kind is
        // forced to deny-all (below) — a nested Branching would sidestep that.
        assert!(toml.sub.is_empty(), "{parent} sub `{}`: a profiled sub must be a leaf (no nested subs)", toml.name);
    }
    // `network_destination` classifies the destination onto the archetype's `locus.provenance`, so
    // it only has meaning on a profiled sub.
    assert!(
        toml.network_destination != Some(true) || toml.profile.is_some(),
        "{parent} sub `{}`: `network_destination` requires a `profile`",
        toml.name,
    );
    assert!(
        toml.destination_flag.is_none() || toml.network_destination == Some(true),
        "{parent} sub `{}`: `destination_flag` requires `network_destination`",
        toml.name,
    );
    // An output-file path is only meaningful on a profiled (`data-export`) sub — the engine gates
    // that file's write onto the sub's derived profile.
    assert!(
        toml.output_path_flags.is_empty() || toml.profile.is_some(),
        "{parent} sub `{}`: `output_path_flags` requires a `profile`",
        toml.name,
    );
    for f in &toml.flag {
        assert!(
            f.classifies == "unclassified" || crate::engine::archetype::archetype(&f.classifies).is_some(),
            "{parent} sub `{}` flag `{}`: classifies `{}` is not a known archetype",
            toml.name, f.name, f.classifies,
        );
        assert!(cited(&f.fact), "{parent} sub `{}` flag `{}`: requires a `fact`", toml.name, f.name);
        assert!(cited(&f.source), "{parent} sub `{}` flag `{}`: requires a `source`", toml.name, f.name);
        assert!(judged(&f.judgment), "{parent} sub `{}` flag `{}`: `judgment`, if given, must not be blank", toml.name, f.name);
        assert!(
            !(f.when_absent == Some(true) && f.value_prefix.is_some()),
            "{parent} sub `{}` flag `{}`: `when_absent` and `value_prefix` are mutually exclusive",
            toml.name, f.name,
        );
    }
}

pub(super) fn build_sub(
    parent: &str,
    mut toml: TomlSub,
    handler_policies: &std::collections::HashMap<String, OwnedPolicy>,
) -> SubSpec {
    check_no_legacy_positional_style(&toml.name, toml.positional_style);
    let name = toml.name.clone();
    let policy_ref = toml.policy.clone();
    let profile = toml.profile.clone();
    let unless_flags = std::mem::take(&mut toml.unless_flags);
    assert!(
        unless_flags.is_empty() || profile.is_some() || !toml.flag.is_empty(),
        "{parent} sub `{name}`: `unless_flags` requires a `profile` or escalating `flag` (it neutralizes them)",
    );
    assert_sub_provenance(parent, &toml);
    assert_no_candidate_shadowed_by_glob(&format!("{parent} {}", toml.name), &toml.sub, &toml.first_arg);
    let flags = std::mem::take(&mut toml.flag)
        .into_iter()
        .map(|f| crate::registry::types::FlagProvenance {
            name: f.name,
            classifies: f.classifies,
            value_prefix: f.value_prefix,
            when_absent: f.when_absent.unwrap_or(false),
        })
        .collect();
    // A profiled sub is engine-classified and above the auto-approve line. Its LEGACY dispatch is
    // reached only when the engine ABSTAINS — e.g. a global flag (`git -c …`, `git -C …`) intervenes
    // before the subcommand, so `sub_archetypes`'s walk stops early. In that case the legacy kind
    // MUST deny outright (fail-closed), never fall through to a permissive default: force bare off,
    // no flags, no positionals. A sub with `unless_flags` is EXEMPT: when a neutralizing flag is
    // present the engine deliberately abstains so the sub's real (`level`/`allow_all`) classification
    // applies (`openssl rsa -pubout` is a safe SafeWrite), so its legacy kind must be kept, not razed.
    if profile.is_some() && unless_flags.is_empty() {
        toml.bare = Some(false);
        toml.standalone = Vec::new();
        toml.valued = Vec::new();
        toml.max_positional = Some(0);
    }
    let eval_safe = toml.eval_safe.unwrap_or(false);
    let eval_safe_flags = std::mem::take(&mut toml.eval_safe_flags);
    let eval_safe_flag_values = std::mem::take(&mut toml.eval_safe_flag_values);
    let eval_safe_required_flags = std::mem::take(&mut toml.eval_safe_required_flags);
    let network_destination = toml.network_destination.unwrap_or(false);
    let destination_flag = toml.destination_flag.clone();
    let output_path_flags = toml.output_path_flags.clone();
    let valued_for_check = toml.valued.clone();
    assert_eval_safe_flags_require_tag(parent, &name, eval_safe, &eval_safe_flags);
    assert_eval_safe_flag_values_consistent(parent, &name, &eval_safe_flags, &eval_safe_flag_values);
    assert_eval_safe_valued_flags_declared(parent, &name, &eval_safe_flags, &valued_for_check, &eval_safe_flag_values);
    assert_eval_safe_required_flags_consistent(parent, &name, &eval_safe_flags, &eval_safe_required_flags);
    assert_sub_eval_safe_only_on_leaf(parent, &toml);
    SubSpec {
        name,
        kind: build_sub_kind(parent, toml, handler_policies),
        policy_ref,
        profile,
        flags,
        unless_flags,
        eval_safe,
        eval_safe_flags,
        eval_safe_flag_values,
        eval_safe_required_flags,
        network_destination,
        destination_flag,
        output_path_flags,
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

fn assert_eval_safe_required_flags_consistent(
    parent: &str,
    name: &str,
    eval_safe_flags: &[String],
    eval_safe_required_flags: &[String],
) {
    for flag in eval_safe_required_flags {
        if !eval_safe_flags.iter().any(|f| f == flag) {
            panic!(
                "command '{parent}' sub `{name}` lists `{flag}` in \
                 `eval_safe_required_flags` but not in `eval_safe_flags`. \
                 A required-flag constraint must be a subset of the \
                 allowed-flag set — otherwise the flag is required AND \
                 immediately denied. Add `{flag}` to `eval_safe_flags` or \
                 remove it from `eval_safe_required_flags`."
            );
        }
    }
}

/// Every valued flag in `eval_safe_flags` must declare its value
/// posture in `eval_safe_flag_values`: either a concrete value
/// allowlist (`["env", "fish"]`) OR the explicit-unrestricted form
/// (`[]`). Without this, a contributor adding a short alias like
/// `-f` for an already-tagged `--format` would silently widen the
/// eval-safe surface to any value of `-f` — the v0.196.0 aws
/// near-miss in disguise.
fn assert_eval_safe_valued_flags_declared(
    parent: &str,
    name: &str,
    eval_safe_flags: &[String],
    valued: &[String],
    eval_safe_flag_values: &std::collections::HashMap<String, Vec<String>>,
) {
    for flag in eval_safe_flags {
        if !valued.iter().any(|v| v == flag) {
            continue;
        }
        if !eval_safe_flag_values.contains_key(flag) {
            panic!(
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
    // Note: command-level eval_safe IS allowed alongside `handler =
    // "..."`. The walker reads `spec.eval_safe` directly at the leaf
    // — it does not need to introspect the handler's dispatch logic.
    // The contributor takes responsibility for vouching that every
    // invocation the handler accepts AND that passes the eval_safe_*
    // flag checks produces shell-init stdout (typically narrowed via
    // `eval_safe_required_flags`). See fzf's TOML for the canonical
    // pattern.
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
    // A `credential_first_arg` gate forces the Branching path even with no sub-subs: only Branching
    // runs `skip_pre_flags`, so the credential check sees the real resource arg (`get -o yaml secret`),
    // not a leading flag — `FirstArg` reads `tokens[1]` verbatim and would be bypassed by a flag.
    if !toml.sub.is_empty() || !toml.credential_first_arg.is_empty() {
        // A sub may carry BOTH explicit sub-subs AND a fallback `first_arg` glob (a service that
        // auto-approves its read verbs via `get-*`/`describe-*` but carves specific dangerous actions
        // out to profiled sub-subs). Dispatch checks the explicit subs FIRST, then the glob
        // (dispatch.rs), so the carve-outs escalate while the benign reads still glob-match. Mirror
        // the command-level Branching, which already threads `first_arg` through; the sub level used
        // to hard-drop it (`Vec::new()`), which made "glob + carve-out" inexpressible.
        let first_arg_level = toml.level.unwrap_or(TomlLevel::Inert).into();
        return DispatchKind::Branching {
            subs: filter_candidates(toml.sub)
                .flat_map(|s| build_subs(parent, s, handler_policies))
                .collect(),
            bare_flags: Vec::new(),
            bare_ok: toml.nested_bare.unwrap_or(false),
            pre_standalone: toml.standalone,
            pre_valued: toml.valued,
            first_arg: toml.first_arg,
            first_arg_level,
            credential_first_arg: toml.credential_first_arg,
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
    if let Some(name) = toml.executor.as_deref() {
        let kind = ExecutorKind::from_name(name).unwrap_or_else(|| {
            panic!("command '{parent}' sub `{}`: unknown executor `{name}` (known: file, project)", toml.name)
        });
        let shape = toml.positional_shape.as_deref().map(|s| {
            crate::policy::PositionalShape::from_name(s)
                .unwrap_or_else(|| panic!("command '{parent}' sub `{}`: unknown positional_shape `{s}`", toml.name))
        });
        return DispatchKind::Executor {
            policy,
            level,
            kind,
            redirect_flag: toml.executor_redirect_flag,
            shape,
        };
    }
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
                TomlMatrixAction::Detailed(d) => &d.policy,
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

/// Lower a `[command.behavior]` block into a typed `BehaviorSpec`. Every facet string is
/// resolved to its enum here (via `FacetTerm::from_term`); an unknown term PANICS naming the
/// command, so a typo fails the build (the registry loads in a test) rather than silently
/// mis-classifying. `None` when the command declares no behavior.
fn lower_behavior(name: &str, b: Option<&TomlBehavior>) -> Option<BehaviorSpec> {
    use crate::engine::facet::{FacetTerm, Operation};
    let b = b?;
    let operation = Operation::from_term(&b.operation)
        .unwrap_or_else(|| panic!("command '{name}': unknown behavior operation `{}`", b.operation));
    let positionals = match b.positionals.as_str() {
        "none" => PositionalRole::None,
        "read" => PositionalRole::Read,
        "write" => PositionalRole::Write,
        "pattern-then-read" => PositionalRole::PatternThenRead,
        "transfer" => PositionalRole::Transfer,
        other => panic!("command '{name}': unknown behavior positionals `{other}` (known: none, read, write, pattern-then-read, transfer)"),
    };
    let scale = match b.scale.as_deref() {
        None | Some("single") => ScaleModel::Single,
        Some("breadth") => ScaleModel::Breadth,
        Some(other) => panic!("command '{name}': unknown behavior scale `{other}` (known: single, breadth)"),
    };
    let hook = match b.hook.as_deref() {
        None => None,
        Some("grep") => Some(BehaviorHook::Grep),
        Some("dd") => Some(BehaviorHook::Dd),
        Some("tar") => Some(BehaviorHook::Tar),
        Some("sed") => Some(BehaviorHook::Sed),
        Some(other) => panic!("command '{name}': unknown behavior hook `{other}` (known: grep, dd, tar, sed)"),
    };
    let (short, long) = split_flag_forms(&b.standalone);
    let (valued_short, valued_long) = split_flag_forms(&b.valued);
    let mut unbounded_flags = Vec::new();
    let mut path_flags = Vec::new();
    for (flag, delta) in &b.flags {
        if delta.scale.as_deref() == Some("unbounded") {
            unbounded_flags.push(flag.clone());
        } else if let Some(other) = delta.scale.as_deref() {
            panic!("command '{name}': behavior flag `{flag}` has unknown scale `{other}` (known: unbounded)");
        }
        if let Some(kind) = delta.kind.as_deref() {
            let role = match kind {
                "read" => PathRole::Read,
                "write" => PathRole::Write,
                other => panic!("command '{name}': behavior flag `{flag}` has unknown kind `{other}` (known: read, write)"),
            };
            if !b.valued.contains(flag) {
                panic!("command '{name}': behavior path-flag `{flag}` (kind = {kind}) must also be listed in `valued`");
            }
            let (short, long) = if let Some(rest) = flag.strip_prefix("--") {
                (None, Some(format!("--{rest}")))
            } else if let Some(rest) = flag.strip_prefix('-') {
                if rest.len() == 1 {
                    (Some(rest.as_bytes()[0]), None)
                } else {
                    panic!("command '{name}': behavior path-flag `{flag}` must be a single-char short or a `--long`");
                }
            } else {
                panic!("command '{name}': behavior path-flag `{flag}` must start with `-`");
            };
            path_flags.push(PathFlag { short, long, role });
        }
    }
    let transfer = lower_transfer(name, b.transfer.as_ref());
    // A transfer role needs its knobs; anything else must not carry them.
    match (positionals, &transfer) {
        (PositionalRole::Transfer, None) => {
            panic!("command '{name}': positionals = \"transfer\" requires a [command.behavior.transfer] block")
        }
        (role, Some(_)) if role != PositionalRole::Transfer => {
            panic!("command '{name}': [command.behavior.transfer] is only valid with positionals = \"transfer\"")
        }
        _ => {}
    }
    Some(BehaviorSpec {
        operation,
        positionals,
        scale,
        short,
        valued_short,
        long,
        valued_long,
        numeric_shorthand: b.numeric_shorthand.unwrap_or(false),
        unbounded_flags,
        path_flags,
        hook,
        transfer,
    })
}

/// Lower a `[command.behavior.transfer]` block, resolving the `source` term and enforcing that
/// the two clobber-flag sets are mutually exclusive (a command declares whether the default is
/// clobber or no-clobber, never both).
fn lower_transfer(name: &str, t: Option<&TomlTransfer>) -> Option<TransferSpec> {
    let t = t?;
    let source = match t.source.as_str() {
        "observe" => TransferSource::Observe,
        "relocate" => TransferSource::Relocate,
        other => panic!("command '{name}': unknown transfer source `{other}` (known: observe, relocate)"),
    };
    if !t.no_clobber_flags.is_empty() && !t.clobber_flags.is_empty() {
        panic!("command '{name}': transfer declares both no_clobber_flags and clobber_flags (mutually exclusive)");
    }
    Some(TransferSpec {
        source,
        no_clobber_flags: t.no_clobber_flags.clone(),
        clobber_flags: t.clobber_flags.clone(),
        recursive_flags: t.recursive_flags.clone(),
    })
}

/// Split a behavior flag list into (single-dash single-char shorts as bytes, `--long`
/// tokens). A single-dash multi-char token is kept whole in `long` — it then only matches as
/// a literal, which for a non-`--` token means it never classifies as known and fails closed.
fn split_flag_forms(tokens: &[String]) -> (Vec<u8>, Vec<String>) {
    let mut short = Vec::new();
    let mut long = Vec::new();
    for t in tokens {
        if t.starts_with("--") {
            long.push(t.clone());
        } else if let Some(rest) = t.strip_prefix('-') {
            if rest.len() == 1 {
                short.push(rest.as_bytes()[0]);
            } else {
                long.push(t.clone());
            }
        }
    }
    (short, long)
}

/// Validate and lower a top-level command's `[[command.flag]]` classifying flags — the flat-command
/// analog of a profiled sub's escalating flags. Each must name a known archetype (or `unclassified`)
/// and cite `fact`/`source`; `when_absent`/`value_prefix` are mutually exclusive. Mirrors the per-sub
/// check in `assert_sub_provenance`.
fn build_command_archetype_flags(
    cmd: &str,
    flags: Vec<TomlSubFlag>,
) -> Vec<crate::registry::types::FlagProvenance> {
    let cited = |o: &Option<String>| o.as_deref().is_some_and(|s| !s.trim().is_empty());
    let judged = |o: &Option<String>| o.as_deref().is_none_or(|s| !s.trim().is_empty());
    flags
        .into_iter()
        .map(|f| {
            assert!(
                f.classifies == "unclassified"
                    || crate::engine::archetype::archetype(&f.classifies).is_some(),
                "command `{cmd}` flag `{}`: classifies `{}` is not a known archetype",
                f.name, f.classifies,
            );
            assert!(cited(&f.fact), "command `{cmd}` flag `{}`: requires a `fact`", f.name);
            assert!(cited(&f.source), "command `{cmd}` flag `{}`: requires a `source`", f.name);
            assert!(judged(&f.judgment), "command `{cmd}` flag `{}`: `judgment`, if given, must not be blank", f.name);
            assert!(
                !(f.when_absent == Some(true) && f.value_prefix.is_some()),
                "command `{cmd}` flag `{}`: `when_absent` and `value_prefix` are mutually exclusive",
                f.name,
            );
            crate::registry::types::FlagProvenance {
                name: f.name,
                classifies: f.classifies,
                value_prefix: f.value_prefix,
                when_absent: f.when_absent.unwrap_or(false),
            }
        })
        .collect()
}

#[allow(clippy::too_many_lines)]
pub(super) fn build_command(toml: TomlCommand, category: &str) -> CommandSpec {
    assert_flat_or_structured(&toml);
    assert_fallback_requires_handler(&toml);
    assert_matrix_policy_keys_exist(&toml);
    assert_no_candidate_shadowed_by_glob(&toml.name, &toml.sub, &toml.first_arg);
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
    let eval_safe_required_flags = toml.eval_safe_required_flags;
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
    assert_eval_safe_valued_flags_declared(
        &toml.name,
        "<command>",
        &eval_safe_flags,
        &toml.valued,
        &eval_safe_flag_values,
    );
    assert_eval_safe_required_flags_consistent(
        &toml.name,
        "<command>",
        &eval_safe_flags,
        &eval_safe_required_flags,
    );
    let behavior = lower_behavior(&toml.name, toml.behavior.as_ref());
    let archetype_flags = build_command_archetype_flags(&toml.name, toml.flag);
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
            eval_safe_required_flags: eval_safe_required_flags.clone(),
            path_gate: toml.path_gate,
            archetype_flags: archetype_flags.clone(),
            behavior: behavior.clone(),
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
    if let Some(vc) = toml.verb_chain {
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
            eval_safe_required_flags: eval_safe_required_flags.clone(),
            path_gate: toml.path_gate,
            archetype_flags: archetype_flags.clone(),
            behavior: behavior.clone(),
            kind: DispatchKind::VerbChain(build_verb_chain(vc)),
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
            eval_safe_required_flags: eval_safe_required_flags.clone(),
            path_gate: toml.path_gate,
            archetype_flags: archetype_flags.clone(),
            behavior: behavior.clone(),
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
                eval_safe_required_flags: eval_safe_required_flags.clone(),
                path_gate: toml.path_gate,
                archetype_flags: archetype_flags.clone(),
                behavior: behavior.clone(),
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
                    credential_first_arg: toml.credential_first_arg,
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
            eval_safe_required_flags: eval_safe_required_flags.clone(),
            path_gate: toml.path_gate,
            archetype_flags: archetype_flags.clone(),
            behavior: behavior.clone(),
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
            eval_safe_required_flags: eval_safe_required_flags.clone(),
            path_gate: toml.path_gate,
            archetype_flags: archetype_flags.clone(),
            behavior: behavior.clone(),
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
                credential_first_arg: toml.credential_first_arg,
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
            eval_safe_required_flags: eval_safe_required_flags.clone(),
            path_gate: toml.path_gate,
            archetype_flags: archetype_flags.clone(),
            behavior: behavior.clone(),
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
            eval_safe_required_flags: eval_safe_required_flags.clone(),
            path_gate: toml.path_gate,
            archetype_flags: archetype_flags.clone(),
            behavior: behavior.clone(),
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
            eval_safe_required_flags: eval_safe_required_flags.clone(),
            path_gate: toml.path_gate,
            archetype_flags: archetype_flags.clone(),
            behavior: behavior.clone(),
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
        eval_safe_required_flags,
        path_gate: toml.path_gate,
        archetype_flags,
        behavior,
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
            eval_safe_required_flags: spec.eval_safe_required_flags.clone(),
            // Aliases are canonicalized (`registry::canonical_name`) before `should_deny` and
            // before the engine's behavior lookup, so the canonical spec's `path_gate` /
            // `behavior` is what's consulted — the alias entry never needs either.
            path_gate: None,
            archetype_flags: Vec::new(),
            behavior: None,
            kind: spec.kind.clone(),
        });
    }
    map.insert(spec.name.clone(), spec);
}
