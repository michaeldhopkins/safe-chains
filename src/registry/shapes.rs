//! A sub's `positional_shape` and `executor` names, read from TOML, and the check a shaped flag
//! policy applies to its first positional.

use super::{ExecutorKind, OwnedPolicy, TomlSub};
use crate::parse::Token;
use crate::policy::PositionalShape;

impl ExecutorKind {
    pub(in crate::registry) fn from_name(name: &str) -> Option<Self> {
        match name {
            "file" => Some(Self::File),
            "project" => Some(Self::Project),
            _ => None,
        }
    }
}

/// The sub's `positional_shape`, refused when the name is unknown or when the sub's dispatch
/// (`write_flags`, `guard`, `first_arg`, `require_any`) would never read it.
pub(in crate::registry) fn sub_shape(parent: &str, toml: &TomlSub) -> Result<Option<PositionalShape>, String> {
    let Some(name) = toml.positional_shape.as_deref() else { return Ok(None) };
    let Some(shape) = PositionalShape::from_name(name) else {
        return Err(format!("command '{parent}' sub `{}`: unknown positional_shape `{name}`", toml.name));
    };
    let ignored = !toml.write_flags.is_empty() || toml.guard.is_some() || !toml.first_arg.is_empty() || !toml.require_any.is_empty();
    if toml.executor.is_none() && ignored {
        return Err(format!(
            "command '{parent}' sub `{}`: positional_shape applies only to a plain flag policy or an executor, not with write_flags, guard, first_arg or require_any",
            toml.name
        ));
    }
    Ok(Some(shape))
}

/// Whether the first positional fails the shape; a policy with no shape, or a line with no
/// positional, never does.
pub(in crate::registry) fn misshapen(shape: Option<PositionalShape>, tokens: &[Token], policy: &OwnedPolicy) -> bool {
    shape.is_some_and(|s| super::super::policy::first_positional(tokens, policy).is_some_and(|first| !s.matches(first)))
}

#[cfg(test)]
mod tests {
    proptest::proptest! {
        /// A sub shaped `local-path` refuses a repository address as its first positional, after any
        /// flags, and keeps a plain local folder.
        #[test]
        fn a_shaped_sub_refuses_a_repository_address_and_keeps_a_local_folder(
            parts in proptest::collection::vec(proptest::string::string_regex("[a-z][a-z0-9_-]{0,7}").unwrap(), 1..4),
            flags in proptest::sample::select(vec!["", "-o ./out.yaml ", "--output=./out.yaml ", "--load-restrictor LoadRestrictionsRootOnly "]),
            form in proptest::sample::select(vec!["github.com/{p}", "https://gitlab.com/{p}", "git@github.com:{p}", "gh:{p}", "{p}.git", "example.org/{p}?ref=v1"]),
        ) {
            let path = parts.join("/");
            let remote = format!("kustomize build {flags}{}", form.replace("{p}", &path));
            proptest::prop_assert!(!crate::is_safe_command(&remote), "{}", remote);
            let local = format!("kustomize build {flags}./{path}");
            proptest::prop_assert!(crate::is_safe_command(&local), "{}", local);
        }
    }
}
