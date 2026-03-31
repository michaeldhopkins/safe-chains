mod build;
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

type HandlerFn = fn(&[Token]) -> Verdict;

static CMD_HANDLERS: LazyLock<HashMap<&'static str, HandlerFn>> =
    LazyLock::new(crate::handlers::custom_cmd_handlers);

static SUB_HANDLERS: LazyLock<HashMap<&'static str, HandlerFn>> =
    LazyLock::new(crate::handlers::custom_sub_handlers);

macro_rules! load_commands {
    ($($file:literal),* $(,)?) => {{
        let mut all = Vec::new();
        $(all.extend(load_toml(include_str!(concat!("../../commands/", $file, ".toml"))));)*
        build_registry(all)
    }};
}

static TOML_REGISTRY: LazyLock<HashMap<String, CommandSpec>> = LazyLock::new(|| {
    load_commands![
        "ai", "android", "ansible", "binary", "builtins", "cloud", "containers", "data",
        "dotnet", "forges", "fs", "fuzzy", "go", "hash", "jvm", "magick", "net",
        "node", "php", "python", "r", "ruby", "rust", "search", "swift",
        "sysinfo", "system", "text", "tools", "vcs", "wrappers", "xcode",
    ]
});

pub fn toml_dispatch(tokens: &[Token]) -> Option<Verdict> {
    let cmd = tokens[0].command_name();
    TOML_REGISTRY.get(cmd).map(|spec| dispatch_spec(tokens, spec))
}

pub fn toml_command_names() -> Vec<&'static str> {
    TOML_REGISTRY
        .keys()
        .map(|k| k.as_str())
        .collect()
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
