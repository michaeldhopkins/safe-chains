mod build;
mod custom;
mod dispatch;
mod docs;
mod policy;
pub(crate) mod types;

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::parse::Token;
use crate::verdict::Verdict;

pub use build::{build_registry, load_toml};
pub(crate) use custom::user_config_level;
pub use dispatch::dispatch_spec;
pub use types::{CommandSpec, OwnedPolicy};

use types::DispatchKind;

type HandlerFn = fn(&[Token]) -> Verdict;

static CMD_HANDLERS: LazyLock<HashMap<&'static str, HandlerFn>> =
    LazyLock::new(crate::handlers::custom_cmd_handlers);

static SUB_HANDLERS: LazyLock<HashMap<&'static str, HandlerFn>> =
    LazyLock::new(crate::handlers::custom_sub_handlers);

static TOML_REGISTRY: LazyLock<HashMap<String, CommandSpec>> = LazyLock::new(||
    include!(concat!(env!("OUT_DIR"), "/toml_includes.rs"))
);

static CUSTOM_REGISTRY: LazyLock<HashMap<String, CommandSpec>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    custom::apply_custom(&mut map);
    map
});

pub fn toml_dispatch(tokens: &[Token]) -> Option<Verdict> {
    let cmd = tokens[0].command_name();
    TOML_REGISTRY.get(cmd).map(|spec| dispatch_spec(tokens, spec))
}

/// Looks up the command in the runtime custom registry (project-local
/// `.safe-chains.toml`, then user-level `~/.config/safe-chains.toml`).
/// A match here wins over the built-in hardcoded handlers, which is how
/// an override of `gh` takes effect.
pub fn custom_dispatch(tokens: &[Token]) -> Option<Verdict> {
    let cmd = tokens[0].command_name();
    CUSTOM_REGISTRY.get(cmd).map(|spec| dispatch_spec(tokens, spec))
}

/// The canonical command name `cmd` resolves to via the registry's alias map (`gcat` → `cat`,
/// `glink` → `ln`). Returns `cmd` unchanged when it is already canonical or unknown, so callers
/// can canonicalize unconditionally. This is what lets the engine's path-gate dispatch reach an
/// aliased invocation: without it, `gcat /etc/shadow` misses every resolver and falls through to
/// the (ungated) legacy classifier. Custom registry first (an override may rename), then TOML.
pub fn canonical_name(cmd: &str) -> &str {
    CUSTOM_REGISTRY
        .get(cmd)
        .or_else(|| TOML_REGISTRY.get(cmd))
        .map_or(cmd, |spec| spec.name.as_str())
}

/// The command's own declared path-argument gate (`[command.path_gate]`), if any — consulted by
/// `pathgate::should_deny` when a command isn't in `pathgates.toml`, so a path-bearing flag gates
/// from the command's own definition. `cmd` is already canonicalized by the caller.
pub(crate) fn command_path_gate(cmd: &str) -> Option<&'static crate::pathgate::RoleSpec> {
    CUSTOM_REGISTRY
        .get(cmd)
        .or_else(|| TOML_REGISTRY.get(cmd))
        .and_then(|spec| spec.path_gate.as_ref())
}

/// The command's declarative facet behavior (`[command.behavior]`), if any — the engine's
/// non-legacy classification path and the ONLY thing `engine::resolve::resolve` consults (the
/// hardcoded `RESOLVERS` table is gone; every facet-classified command declares behavior).
/// `cmd` is already canonicalized by the caller.
pub(crate) fn command_behavior(cmd: &str) -> Option<&'static crate::registry::types::BehaviorSpec> {
    CUSTOM_REGISTRY
        .get(cmd)
        .or_else(|| TOML_REGISTRY.get(cmd))
        .and_then(|spec| spec.behavior.as_ref())
}

/// The facet archetypes (`archetypes.toml`) the subcommand `tokens` resolve to — the Phase-1
/// `profile = …` classification plus a capability for every present escalating `[[command.sub.flag]]`
/// (`git push` → `[vcs-sync]`; `git push --force` → `[vcs-sync, remote-destroy-irreversible]`). The
/// engine emits a Capability per name and the level algebra takes the max. Descends `Branching`/
/// `Custom` subs to the DEEPEST profile-bearing sub (nested `<resource> <action>`). `None` when no
/// matched sub declares a profile.
pub(crate) fn sub_archetypes(tokens: &[Token]) -> Option<Vec<&'static str>> {
    let cmd = canonical_name(tokens.first()?.command_name());
    let spec = CUSTOM_REGISTRY.get(cmd).or_else(|| TOML_REGISTRY.get(cmd))?;
    let sub = walk_to_classified_sub(&tokens[1..], &spec.kind)?;
    // A sub with a base `profile` classifies as that archetype; each present escalating flag ADDS one.
    // A sub with NO base profile but escalating flags (`openssl enc -d`: bimodal — encrypt by default,
    // decrypt only with `-d`) contributes ONLY the flags that fire; if none fires (`openssl enc -e`),
    // it returns None so the caller falls through to the sub's ordinary (legacy) classification.
    let mut out = Vec::new();
    if let Some(p) = sub.profile.as_deref() {
        // `unless_flags` NEUTRALIZE the base profile: a safe-output flag (`openssl rsa -pubout`/`-out`/
        // `-noout`) diverts the disclosure, so the sub falls through to its ordinary classification.
        if !sub.unless_flags.iter().any(|f| flag_present(tokens, f)) {
            out.push(p);
        }
    }
    for flag in &sub.flags {
        if flag_escalates(tokens, flag) {
            out.push(flag.classifies.as_str());
        }
    }
    (!out.is_empty()).then_some(out)
}

/// The facet archetypes a flat command's PRESENT top-level classifying flags (`[[command.flag]]`)
/// resolve to — the command-level analog of `sub_archetypes` for a flag-triggered mode (`age -d` →
/// `[decrypt-read]`, `sops --decrypt` → `[decrypt-read]`). `None` when the command declares none or
/// none are present, so `engine::resolve` falls through to the command's ordinary resolution (the
/// bare/encrypt form of a bimodal tool). Uses the same `flag_escalates` predicate as the sub flags.
pub(crate) fn command_flag_archetypes(tokens: &[Token]) -> Option<Vec<&'static str>> {
    let cmd = canonical_name(tokens.first()?.command_name());
    let spec = CUSTOM_REGISTRY.get(cmd).or_else(|| TOML_REGISTRY.get(cmd))?;
    let present: Vec<&'static str> = spec
        .archetype_flags
        .iter()
        .filter(|f| flag_escalates(tokens, f))
        .map(|f| f.classifies.as_str())
        .collect();
    (!present.is_empty()).then_some(present)
}

/// Whether `flag` escalates given `tokens`. A bare flag (no `value_prefix`) escalates on presence; a
/// value-matched flag escalates only when its VALUE starts with the prefix — the space form
/// (`-c core.sshCommand=…`) or the glued form (`--flag=core.sshCommand=…`). Scans the whole line;
/// an escalator counts wherever it sits.
fn flag_escalates(tokens: &[Token], flag: &types::FlagProvenance) -> bool {
    if flag.when_absent {
        // A SAFETY flag whose ABSENCE is the escalation (`npm ci` without `--ignore-scripts`).
        // It must be AFFIRMATIVELY set — `--ignore-scripts=false` / `--no-ignore-scripts` re-ENABLE
        // scripts, so they escalate exactly like the flag being missing (a fail-open otherwise).
        return !flag_is_affirmatively_set(tokens, &flag.name);
    }
    let Some(prefix) = flag.value_prefix.as_deref() else {
        return flag_present(tokens, &flag.name);
    };
    // space form: `NAME VALUE`, VALUE starting with the prefix
    tokens.windows(2).any(|w| w[0].as_str() == flag.name && w[1].as_str().starts_with(prefix))
        // glued form: `NAME=VALUE`, VALUE starting with the prefix
        || tokens.iter().any(|t| {
            t.as_str()
                .strip_prefix(flag.name.as_str())
                .and_then(|r| r.strip_prefix('='))
                .is_some_and(|v| v.starts_with(prefix))
        })
}

/// Whether a boolean flag is AFFIRMATIVELY enabled: bare `--flag`, or `--flag=<truthy>`. A
/// `--flag=false/0/no/off` or a `--no-flag` DISABLES it (returns false), as does absence. Last
/// occurrence wins, the CLI convention. Used by the `when_absent` escalator so a re-enabling spelling
/// can't masquerade as the safety flag being set.
fn flag_is_affirmatively_set(tokens: &[Token], flag: &str) -> bool {
    let neg = format!("--no-{}", flag.trim_start_matches('-'));
    let mut set = false;
    for t in tokens {
        let s = t.as_str();
        if s == flag {
            set = true;
        } else if let Some(v) = s.strip_prefix(flag).and_then(|r| r.strip_prefix('=')) {
            set = !matches!(v.to_ascii_lowercase().as_str(), "false" | "0" | "no" | "off" | "");
        } else if s == neg {
            set = false;
        }
    }
    set
}

fn walk_to_classified_sub(
    remaining: &[Token],
    kind: &'static DispatchKind,
) -> Option<&'static types::SubSpec> {
    let subs = match kind {
        DispatchKind::Branching { subs, .. } | DispatchKind::Custom { subs, .. } => subs,
        _ => return None,
    };
    let arg = remaining.first()?;
    let sub = subs.iter().find(|s| s.name == arg.as_str())?;
    // Deepest classified match wins: a nested action's profile overrides its resource sub's. A sub is
    // "classified" if it declares a base `profile` OR any escalating `flag` (`openssl enc` has flags
    // but no base profile — bimodal, decrypt only with `-d`).
    walk_to_classified_sub(&remaining[1..], &sub.kind)
        .or_else(|| (sub.profile.is_some() || !sub.flags.is_empty()).then_some(sub))
}

/// Like `walk_to_profiled_sub`, but also returns the tokens AFTER the matched sub's name — the
/// operands the engine still needs to inspect (the destination positional, for `network_destination`).
fn walk_to_profiled_sub_rest<'a>(
    remaining: &'a [Token],
    kind: &'static DispatchKind,
) -> Option<(&'static types::SubSpec, &'a [Token])> {
    let subs = match kind {
        DispatchKind::Branching { subs, .. } | DispatchKind::Custom { subs, .. } => subs,
        _ => return None,
    };
    let arg = remaining.first()?;
    let sub = subs.iter().find(|s| s.name == arg.as_str())?;
    let rest = &remaining[1..];
    walk_to_profiled_sub_rest(rest, &sub.kind).or_else(|| sub.profile.is_some().then_some((sub, rest)))
}

/// For a profiled sub declaring `network_destination`, the first positional after it — the send
/// TARGET (`git push origin` → `origin`; bare `git push` → `None`, the configured default). `None`
/// (outer) when the resolved sub does not classify a destination. The engine maps the token's
/// PROVENANCE onto `locus.provenance` (`resolve::destination_provenance`).
///
/// "First non-`-` token" is a heuristic: a VALUED flag's value sitting before the target
/// (`git push -o $VAR origin`) is read as the destination. That only ever misreads CONSERVATIVELY —
/// a stray value classifies to `literal`/`opaque` (equal-or-stricter than the real `established`
/// target), never looser — so it can over-deny a rare form but never under-approve.
pub(crate) fn sub_destination_token(tokens: &[Token]) -> Option<Option<&str>> {
    let cmd = canonical_name(tokens.first()?.command_name());
    let spec = CUSTOM_REGISTRY.get(cmd).or_else(|| TOML_REGISTRY.get(cmd))?;
    let (sub, rest) = walk_to_profiled_sub_rest(&tokens[1..], &spec.kind)?;
    if !sub.network_destination {
        return None;
    }
    // A destination-carrying flag (`git push --repo=<dest>`) OVERRIDES the positional — else a
    // `--repo=ext::sh` RCE would slip past a benign positional (`origin`). Scanned across the whole
    // line since the flag may sit anywhere.
    if let Some(flag) = sub.destination_flag.as_deref()
        && let Some(v) = flag_value(tokens, flag)
    {
        return Some(Some(v));
    }
    Some(rest.iter().map(Token::as_str).find(|t| !t.starts_with('-')))
}

/// A flag's value, glued (`--repo=VALUE`) or space-separated (`--repo VALUE`); `None` if absent.
fn flag_value<'a>(tokens: &'a [Token], flag: &str) -> Option<&'a str> {
    if let Some(v) = tokens.iter().find_map(|t| t.as_str().strip_prefix(flag).and_then(|r| r.strip_prefix('='))) {
        return Some(v);
    }
    tokens.windows(2).find(|w| w[0].as_str() == flag).map(|w| w[1].as_str())
}

/// For a profiled `data-export` sub declaring `output_path_flags`, the output-file PATH one of them
/// carries — or `None` when the export streams to stdout (no output flag present). The engine adds a
/// path-gated write capability at this path's locus, so a dump to `/etc/cron.d/job` gates on locus
/// exactly as a redirect there would. `None` (outer) when the resolved sub declares none.
///
/// Values are matched in every spelling the flag admits: `--file=X`, `--file X`, `-f X`, `-f=X`, and
/// the glued short form `-fX` — the last mustn't be a bypass (`-f/etc/cron.d/job` reaching a system
/// path would otherwise drop the write cap and auto-approve). A bare `-f` with no value is a
/// malformed invocation the tool itself rejects, so a `None` there is harmless.
pub(crate) fn sub_output_path_token(tokens: &[Token]) -> Option<&str> {
    let cmd = canonical_name(tokens.first()?.command_name());
    let spec = CUSTOM_REGISTRY.get(cmd).or_else(|| TOML_REGISTRY.get(cmd))?;
    let (sub, _rest) = walk_to_profiled_sub_rest(&tokens[1..], &spec.kind)?;
    sub.output_path_flags.iter().find_map(|f| output_flag_value(tokens, f))
}

/// `flag_value`, plus the glued short form `-fVALUE` (a two-char `-x` flag with the value fused on).
/// Kept separate from `flag_value` because a glued short is only unambiguous for the single-letter
/// output flags this classifies (`-f`, `-r`); the destination-flag path (`--repo`) never needs it.
fn output_flag_value<'a>(tokens: &'a [Token], flag: &'a str) -> Option<&'a str> {
    if let Some(v) = flag_value(tokens, flag) {
        return Some(v);
    }
    if flag.len() == 2 && flag.starts_with('-') && !flag.starts_with("--") {
        return tokens
            .iter()
            .find_map(|t| t.as_str().strip_prefix(flag).filter(|r| !r.is_empty()));
    }
    None
}

/// Whether `flag` appears anywhere in `tokens` — bare (`--force`) or glued (`--flag=v`). A flag is
/// an escalator wherever it sits in the invocation, so this scans the whole line.
fn flag_present(tokens: &[Token], flag: &str) -> bool {
    tokens.iter().any(|t| {
        let s = t.as_str();
        s == flag || s.strip_prefix(flag).is_some_and(|rest| rest.starts_with('='))
    })
}

pub fn toml_command_names() -> Vec<&'static str> {
    TOML_REGISTRY
        .keys()
        .map(|k| k.as_str())
        .collect()
}

/// Every command's canonical name with its declared `examples_safe` / `examples_denied`
/// — the corpus the engine's never-looser corpus gate runs against.
#[cfg(test)]
pub(crate) fn corpus_examples()
-> Vec<(&'static str, &'static [String], &'static [String])> {
    TOML_REGISTRY
        .iter()
        .map(|(name, spec)| {
            (name.as_str(), spec.examples_safe.as_slice(), spec.examples_denied.as_slice())
        })
        .collect()
}

/// Look up `cmd_name`'s TOML-declared subs (set via `[[command.sub]]`
/// blocks alongside `handler = "..."`) and dispatch the one whose name
/// matches `tokens[1]`. Returns `None` if no sub matched, so the
/// handler can fall through to its fallback grammar (or deny).
pub fn try_sub_dispatch(cmd_name: &str, tokens: &[Token]) -> Option<Verdict> {
    let spec = handler_spec(cmd_name)?;
    let DispatchKind::Custom { subs, .. } = &spec.kind else {
        return None;
    };
    let arg = tokens.get(1)?.as_str();
    let sub = subs.iter().find(|s| s.name == arg)?;
    Some(dispatch::dispatch_sub_kind(&tokens[1..], &sub.kind))
}

/// Apply `cmd_name`'s TOML-declared `[command.fallback]` grammar.
/// Returns `None` if no fallback is declared.
pub fn try_fallback_grammar(cmd_name: &str, tokens: &[Token]) -> Option<Verdict> {
    let spec = handler_spec(cmd_name)?;
    let DispatchKind::Custom { fallback, .. } = &spec.kind else {
        return None;
    };
    let f = fallback.as_ref()?;
    Some(dispatch::dispatch_fallback(tokens, f))
}

/// Dispatch `tokens` against `cmd_name`'s `[[command.matrix]]`
/// blocks. Looks at `tokens[1]` (parent) and `tokens[2]` (action),
/// finds the first matrix whose `parents` contains the parent and
/// whose `actions` map contains the action, then validates
/// `tokens[2..]` against the named policy (and a guard flag if the
/// matrix entry declared one). Returns `None` if no matrix matched —
/// the handler can then fall through to its remaining special cases
/// or deny.
pub fn try_matrix_dispatch(cmd_name: &str, tokens: &[Token]) -> Option<Verdict> {
    let spec = handler_spec(cmd_name)?;
    let DispatchKind::Custom { matrices, handler_policies, .. } = &spec.kind else {
        return None;
    };
    let parent = tokens.get(1)?.as_str();
    let action = tokens.get(2)?.as_str();
    for matrix in matrices {
        if !matrix.parents.iter().any(|p| p == parent) {
            continue;
        }
        let Some(action_spec) = matrix.actions.get(action) else { continue; };
        if let Some(long) = action_spec.guard.as_deref()
            && !crate::parse::has_flag(&tokens[2..], action_spec.guard_short.as_deref(), Some(long))
        {
            return Some(Verdict::Denied);
        }
        let Some(policy) = handler_policies.get(&action_spec.policy_key) else {
            return Some(Verdict::Denied);
        };
        return Some(dispatch::dispatch_matrix_action(&tokens[2..], policy, matrix.level));
    }
    None
}

/// Validate `tokens` against `cmd_name`'s named flag policy declared
/// in a `[command.handler_policy.KEY]` block. Returns `false` if no
/// such policy is declared or the tokens fail it. Used by handlers
/// whose dispatch logic genuinely can't move to TOML (e.g. gh's
/// sub × action matrix) but whose per-policy WordSets should live
/// in TOML rather than as Rust `WordSet` constants.
pub fn check_handler_policy(cmd_name: &str, key: &str, tokens: &[Token]) -> bool {
    let Some(spec) = handler_spec(cmd_name) else { return false; };
    let DispatchKind::Custom { handler_policies, .. } = &spec.kind else {
        return false;
    };
    let Some(policy) = handler_policies.get(key) else { return false; };
    dispatch::check_handler_policy_owned(tokens, policy)
}

fn handler_spec(cmd_name: &str) -> Option<&'static CommandSpec> {
    CUSTOM_REGISTRY
        .get(cmd_name)
        .or_else(|| TOML_REGISTRY.get(cmd_name))
}

/// Returns true iff this invocation is tagged eval-safe — meaning its
/// stdout is documented shell-init code that can safely be substituted
/// inside `eval "$(...)"`.
///
/// The walker descends through `DispatchKind::Branching` AND
/// `DispatchKind::Custom` matching subs token-by-token (handler-based
/// commands such as `gh` can have tagged TOML-declared subs even though
/// the handler does the actual dispatch). The leaf is the deepest matched
/// node (where no further sub matches). `eval_safe` is checked only at
/// the leaf — ancestor tags do NOT propagate. After confirming the leaf
/// is tagged, every `-`-prefixed token in the remaining tail must appear
/// in `eval_safe_flags`; positionals are unrestricted.
///
/// Tagged nodes are vetted manually per-command (see SAMPLE.toml). This
/// function does not validate that `tokens` is syntactically allowed —
/// callers must have already passed it through the regular dispatcher.
pub fn is_eval_safe_invocation(tokens: &[Token]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let cmd = tokens[0].command_name();
    let Some(spec) = CUSTOM_REGISTRY.get(cmd).or_else(|| TOML_REGISTRY.get(cmd)) else {
        return false;
    };
    is_eval_safe_for_spec(spec, tokens)
}

/// Spec-local variant used by tests so they can build a `CommandSpec`
/// via `load_toml` and exercise the walker without touching the global
/// `TOML_REGISTRY`.
pub(crate) fn is_eval_safe_for_spec(spec: &CommandSpec, tokens: &[Token]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    walk_to_eval_safe_leaf(
        &tokens[1..],
        &spec.kind,
        spec.eval_safe,
        &spec.eval_safe_flags,
        &spec.eval_safe_flag_values,
        &spec.eval_safe_required_flags,
    )
}

fn walk_to_eval_safe_leaf(
    remaining: &[Token],
    kind: &DispatchKind,
    eval_safe: bool,
    eval_safe_flags: &[String],
    eval_safe_flag_values: &std::collections::HashMap<String, Vec<String>>,
    eval_safe_required_flags: &[String],
) -> bool {
    let subs_opt = match kind {
        DispatchKind::Branching { subs, .. } | DispatchKind::Custom { subs, .. } => Some(subs),
        _ => None,
    };
    if let Some(subs) = subs_opt
        && let Some(arg) = remaining.first()
        && let Some(sub) = subs.iter().find(|s| s.name == arg.as_str())
    {
        return walk_to_eval_safe_leaf(
            &remaining[1..],
            &sub.kind,
            sub.eval_safe,
            &sub.eval_safe_flags,
            &sub.eval_safe_flag_values,
            &sub.eval_safe_required_flags,
        );
    }
    if !eval_safe {
        return false;
    }
    let mut i = 0;
    let mut seen_required = false;
    while i < remaining.len() {
        let s = remaining[i].as_str();
        if !s.starts_with('-') {
            i += 1;
            continue;
        }
        let (bare, eq_value) = match s.split_once('=') {
            Some((k, v)) => (k, Some(v)),
            None => (s, None),
        };
        if !eval_safe_flags.iter().any(|f| f == bare) {
            return false;
        }
        if eval_safe_required_flags.iter().any(|f| f == bare) {
            seen_required = true;
        }
        if let Some(allowed) = eval_safe_flag_values.get(bare) {
            // Valued flag declared in eval_safe_flag_values. The value
            // arrives either as `--flag=VALUE` (eq_value is Some) or as
            // the next token (`--flag VALUE`); either way the walker
            // consumes it because a flag in eval_safe_flag_values is
            // structurally valued.
            //
            // `allowed` empty = explicit-unrestricted: contributor
            // vetted that any bare-literal value preserves shell-init
            // output. Non-empty = value must appear in the allowlist.
            let value: &str = if let Some(v) = eq_value {
                v
            } else if let Some(next) = remaining.get(i + 1) {
                let v = next.as_str();
                i += 1;
                v
            } else {
                return false;
            };
            // Empty value is denied even under the explicit-
            // unrestricted (`= []`) posture: `--flag=` and an empty
            // following token never represent a meaningful tool
            // argument. The bare-literal alphabet check is per-char
            // and vacuously passes empty strings, so the walker
            // has to reject explicitly.
            if value.is_empty() {
                return false;
            }
            if !allowed.is_empty() && !allowed.iter().any(|av| av == value) {
                return false;
            }
        }
        i += 1;
    }
    if !eval_safe_required_flags.is_empty() && !seen_required {
        return false;
    }
    true
}

pub fn toml_command_docs() -> Vec<crate::docs::CommandDoc> {
    TOML_REGISTRY
        .iter()
        .filter(|(key, spec)| *key == &spec.name)
        .map(|(_, spec)| spec.to_command_doc())
        .collect()
}

#[cfg(test)]
mod tests;
