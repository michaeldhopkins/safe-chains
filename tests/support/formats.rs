//! Every hook format in the registry, paired with its target's name.
//!
//! A target can answer in more than one format (Codex answers `PreToolUse` and
//! `PermissionRequest`), so a contract guard that walked `hook_format()` alone would leave the
//! second one unchecked. The targets are leaked so each format can be borrowed for `'static`;
//! this is test code and the registry is a handful of unit structs.

use safe_chains::targets::HookFormat;

pub fn every_hook_format() -> Vec<(&'static str, &'static dyn HookFormat)> {
    safe_chains::targets::registry()
        .into_iter()
        .flat_map(|target| {
            let target: &'static dyn safe_chains::targets::Target = Box::leak(target);
            target.hook_formats().into_iter().map(move |format| (target.name(), format))
        })
        .collect()
}
