use crate::registry::types::{FlagProvenance, TomlSubFlag, TomlWrapper};

/// A command's top-level classifying flags (`[[command.flag]]`), with the flags whose next word is
/// their value anywhere in the command (`flag_valued`, `valued` and the wrapper's `valued`), so an
/// unknown word after one is not taken for a classifying flag.
#[derive(Debug, Clone, Default)]
pub(crate) struct ArchetypeFlags {
    pub flags: Vec<FlagProvenance>,
    pub valued: Vec<String>,
}

impl ArchetypeFlags {
    pub(crate) fn iter(&self) -> std::slice::Iter<'_, FlagProvenance> {
        self.flags.iter()
    }

    pub(crate) fn may_hide_a_flag(&self, tokens: &[crate::parse::Token]) -> bool {
        crate::cst::opaque::may_be_any_flag(tokens, &self.valued)
    }
}

pub(super) fn lower(
    cmd: &str,
    flags: Vec<TomlSubFlag>,
    flag_valued: &[String],
    valued: &[String],
    wrapper: Option<&TomlWrapper>,
) -> Result<ArchetypeFlags, String> {
    let valued = flag_valued.iter().chain(valued).chain(wrapper.iter().flat_map(|w| &w.valued)).cloned().collect();
    let flags = build_command_archetype_flags(cmd, flags)?;
    Ok(ArchetypeFlags { flags, valued })
}

/// Validate and lower a top-level command's `[[command.flag]]` classifying flags — the flat-command
/// analog of a profiled sub's escalating flags. Each must name a known archetype (or `unclassified`)
/// and cite `fact`/`source`; `when_absent`/`value_prefix` are mutually exclusive. Mirrors the per-sub
/// check in `assert_sub_provenance`.
fn build_command_archetype_flags(cmd: &str, flags: Vec<TomlSubFlag>) -> Result<Vec<FlagProvenance>, String> {
    let cited = |o: &Option<String>| o.as_deref().is_some_and(|s| !s.trim().is_empty());
    let judged = |o: &Option<String>| o.as_deref().is_none_or(|s| !s.trim().is_empty());
    flags
        .into_iter()
        .map(|f| {
            ensure!(
                f.classifies == "unclassified" || crate::engine::archetype::archetype(&f.classifies).is_some(),
                "command `{cmd}` flag `{}`: classifies `{}` is not a known archetype",
                f.name,
                f.classifies,
            );
            ensure!(cited(&f.fact), "command `{cmd}` flag `{}`: requires a `fact`", f.name);
            ensure!(cited(&f.source), "command `{cmd}` flag `{}`: requires a `source`", f.name);
            ensure!(judged(&f.judgment), "command `{cmd}` flag `{}`: `judgment`, if given, must not be blank", f.name);
            ensure!(
                !(f.when_absent == Some(true) && f.value_prefix.is_some()),
                "command `{cmd}` flag `{}`: `when_absent` and `value_prefix` are mutually exclusive",
                f.name,
            );
            Ok(FlagProvenance {
                name: f.name,
                classifies: f.classifies,
                value_prefix: f.value_prefix,
                when_absent: f.when_absent.unwrap_or(false),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn valued_joins_the_flag_and_command_lists(
            flag_valued in proptest::collection::vec("-{1,2}[a-z]{1,8}", 0..4),
            valued in proptest::collection::vec("-{1,2}[a-z]{1,8}", 0..4),
        ) {
            let lowered = super::lower("x", Vec::new(), &flag_valued, &valued, None).expect("lowers");
            let joined: Vec<String> = flag_valued.iter().chain(&valued).cloned().collect();
            prop_assert_eq!(lowered.valued, joined);
            prop_assert!(lowered.flags.is_empty());
        }
    }
}
