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
