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

static TOML_REGISTRY: LazyLock<HashMap<String, CommandSpec>> = LazyLock::new(|| {
    let mut all = Vec::new();
    all.extend(load_toml(include_str!("../../commands/ai.toml")));
    all.extend(load_toml(include_str!("../../commands/android.toml")));
    all.extend(load_toml(include_str!("../../commands/binary.toml")));
    all.extend(load_toml(include_str!("../../commands/builtins.toml")));
    all.extend(load_toml(include_str!("../../commands/containers.toml")));
    all.extend(load_toml(include_str!("../../commands/data.toml")));
    all.extend(load_toml(include_str!("../../commands/dotnet.toml")));
    all.extend(load_toml(include_str!("../../commands/fs.toml")));
    all.extend(load_toml(include_str!("../../commands/fuzzy.toml")));
    all.extend(load_toml(include_str!("../../commands/go.toml")));
    all.extend(load_toml(include_str!("../../commands/hash.toml")));
    all.extend(load_toml(include_str!("../../commands/jvm.toml")));
    all.extend(load_toml(include_str!("../../commands/magick.toml")));
    all.extend(load_toml(include_str!("../../commands/net.toml")));
    all.extend(load_toml(include_str!("../../commands/node.toml")));
    all.extend(load_toml(include_str!("../../commands/php.toml")));
    all.extend(load_toml(include_str!("../../commands/python.toml")));
    all.extend(load_toml(include_str!("../../commands/ruby.toml")));
    all.extend(load_toml(include_str!("../../commands/rust.toml")));
    all.extend(load_toml(include_str!("../../commands/search.toml")));
    all.extend(load_toml(include_str!("../../commands/swift.toml")));
    all.extend(load_toml(include_str!("../../commands/sysinfo.toml")));
    all.extend(load_toml(include_str!("../../commands/system.toml")));
    all.extend(load_toml(include_str!("../../commands/text.toml")));
    all.extend(load_toml(include_str!("../../commands/tools.toml")));
    all.extend(load_toml(include_str!("../../commands/wrappers.toml")));
    all.extend(load_toml(include_str!("../../commands/xcode.toml")));
    build_registry(all)
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
        .values()
        .map(|spec| spec.to_command_doc())
        .collect()
}

#[cfg(test)]
mod tests;
