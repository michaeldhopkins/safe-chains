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

pub fn toml_command_names() -> Vec<&'static str> {
    TOML_REGISTRY
        .keys()
        .map(|k| k.as_str())
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

pub fn toml_command_docs() -> Vec<crate::docs::CommandDoc> {
    TOML_REGISTRY
        .iter()
        .filter(|(key, spec)| *key == &spec.name)
        .map(|(_, spec)| spec.to_command_doc())
        .collect()
}

#[cfg(test)]
mod tests;
