//! What a subcommand writes in the folder it runs in without naming a path (`writes_cwd`), and the
//! subs that run the folder's own code (`executor = "project"`). Read by the unknown-folder mode
//! (`pathctx::folder`), which refuses a write it cannot account for.

use crate::parse::Token;
use crate::pathctx::anchor::Anchor;

use super::types::DispatchKind;
use super::{CUSTOM_REGISTRY, TOML_REGISTRY, canonical_name};

/// Lower a sub's `writes_cwd` / `output_dirs` / `executor` into its declaration, refusing a shape
/// that says less than it seems to.
pub(super) fn lower(
    parent: &str,
    name: &str,
    writes_cwd: Option<&str>,
    output_dirs: &[String],
    executor: Option<&str>,
) -> Result<Option<Anchor>, String> {
    let at = format!("command '{parent}' sub `{name}`");
    if let Some(dir) = output_dirs.iter().find(|d| !is_owned_subtree(d)) {
        return Err(format!("{at}: output_dirs entry `{dir}` must be a relative directory below the folder"));
    }
    let anchor = match (writes_cwd, executor) {
        (Some(_), Some("project")) => {
            return Err(format!("{at}: `executor = \"project\"` already says it runs the folder's code; drop `writes_cwd`"));
        }
        (Some("output"), _) if output_dirs.is_empty() => {
            return Err(format!("{at}: `writes_cwd = \"output\"` needs `output_dirs`, the subtrees the tool owns"));
        }
        (Some("output"), _) => Anchor::ImplicitOutput,
        (Some("source"), _) if !output_dirs.is_empty() => {
            return Err(format!("{at}: `output_dirs` belongs with `writes_cwd = \"output\"`, not `source`"));
        }
        (Some("source"), _) => Anchor::ImplicitSource,
        (Some("none"), _) if !output_dirs.is_empty() => {
            return Err(format!("{at}: `output_dirs` belongs with `writes_cwd = \"output\"`, not `none`"));
        }
        (Some("none"), _) => Anchor::Free,
        (Some("code"), _) if !output_dirs.is_empty() => {
            return Err(format!("{at}: `output_dirs` belongs with `writes_cwd = \"output\"`, not `code`"));
        }
        (Some("code"), _) => Anchor::RunsFolderCode,
        (Some("named"), _) if !output_dirs.is_empty() => {
            return Err(format!("{at}: `output_dirs` belongs with `writes_cwd = \"output\"`, not `named`"));
        }
        (Some("named"), _) => Anchor::NamesItsWrites,
        (Some(other), _) => return Err(format!("{at}: unknown writes_cwd `{other}` (known: output, source, none, code, named)")),
        (None, _) if !output_dirs.is_empty() => {
            return Err(format!("{at}: `output_dirs` without `writes_cwd = \"output\"`"));
        }
        (None, Some("project")) => Anchor::RunsFolderCode,
        (None, _) => return Ok(None),
    };
    Ok(Some(anchor))
}

fn is_owned_subtree(dir: &str) -> bool {
    !dir.is_empty() && !dir.starts_with('/') && !dir.starts_with('~') && dir.split('/').all(|s| !s.is_empty() && s != "." && s != "..")
}

/// The declared write of the deepest sub `tokens` reach, when that sub declares one. A sub that
/// declares nothing does not inherit its parent's declaration: `git stash` writing source says
/// nothing researched about `git stash drop`. `None` leaves the write unaccounted for.
pub(crate) fn cwd_writes(tokens: &[Token]) -> Option<Anchor> {
    let canonical = canonical_name(tokens.first()?.command_name());
    let spec = CUSTOM_REGISTRY.get(canonical).or_else(|| TOML_REGISTRY.get(canonical))?;
    let mut rest = &tokens[1..];
    if rest.first().is_some_and(|t| t.as_str().starts_with('+')) {
        rest = &rest[1..];
    }
    let mut kind = &spec.kind;
    let mut found = spec.writes_cwd;
    loop {
        let subs = match kind {
            DispatchKind::Branching { subs, .. } | DispatchKind::Custom { subs, .. } => subs,
            _ => return found,
        };
        let Some((arg, tail)) = rest.split_first() else { return found };
        let Some(sub) = subs.iter().find(|s| s.name == arg.as_str()) else {
            return found;
        };
        found = sub.writes_cwd;
        rest = tail;
        kind = &sub.kind;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lowered(writes: Option<&str>, dirs: &[&str], executor: Option<&str>) -> Result<Option<Anchor>, String> {
        let dirs: Vec<String> = dirs.iter().map(|d| (*d).to_string()).collect();
        lower("tool", "sub", writes, &dirs, executor)
    }

    #[test]
    fn each_declaration_lowers_to_its_anchor() {
        assert_eq!(lowered(Some("output"), &["target"], None), Ok(Some(Anchor::ImplicitOutput)));
        assert_eq!(lowered(Some("output"), &[".build", "out/gen"], None), Ok(Some(Anchor::ImplicitOutput)));
        assert_eq!(lowered(Some("source"), &[], None), Ok(Some(Anchor::ImplicitSource)));
        assert_eq!(lowered(Some("none"), &[], None), Ok(Some(Anchor::Free)));
        assert_eq!(lowered(Some("code"), &[], None), Ok(Some(Anchor::RunsFolderCode)));
        assert_eq!(lowered(None, &[], Some("project")), Ok(Some(Anchor::RunsFolderCode)));
        assert_eq!(lowered(None, &[], None), Ok(None));
        assert_eq!(lowered(None, &[], Some("file")), Ok(None));
    }

    proptest::proptest! {
        /// Lowering never panics, and anything it accepts is one of the five declarations with the
        /// shape that declaration needs: output dirs exactly when the write is output.
        #[test]
        fn only_a_well_formed_declaration_lowers(
            writes in proptest::option::of(proptest::prop_oneof!["output|source|none|code", ".{0,8}"]),
            dirs in proptest::collection::vec("[a-z./~]{0,6}", 0..3),
            executor in proptest::option::of(proptest::prop_oneof!["project|file", ".{0,4}"]),
        ) {
            let dirs_ref: Vec<&str> = dirs.iter().map(String::as_str).collect();
            if let Ok(Some(anchor)) = lowered(writes.as_deref(), &dirs_ref, executor.as_deref()) {
                proptest::prop_assert_eq!(anchor == Anchor::ImplicitOutput, !dirs.is_empty());
                proptest::prop_assert!(dirs.iter().all(|d| is_owned_subtree(d)));
                match writes.as_deref() {
                    Some("output") => proptest::prop_assert_eq!(anchor, Anchor::ImplicitOutput),
                    Some("source") => proptest::prop_assert_eq!(anchor, Anchor::ImplicitSource),
                    Some("none") => proptest::prop_assert_eq!(anchor, Anchor::Free),
                    Some("code") => proptest::prop_assert_eq!(anchor, Anchor::RunsFolderCode),
                    None => proptest::prop_assert_eq!((anchor, executor.as_deref()), (Anchor::RunsFolderCode, Some("project"))),
                    Some(other) => proptest::prop_assert!(false, "accepted `{}`", other),
                }
            }
        }
    }

    #[test]
    fn a_declaration_that_says_less_than_it_seems_is_refused() {
        for (writes, dirs, executor) in [
            (Some("output"), vec![], None),
            (Some("source"), vec!["target"], None),
            (Some("none"), vec!["target"], None),
            (None, vec!["target"], None),
            (Some("outputs"), vec!["target"], None),
            (Some("source"), vec![], Some("project")),
            (Some("output"), vec!["/tmp"], None),
            (Some("output"), vec!["../x"], None),
            (Some("output"), vec!["~/x"], None),
            (Some("output"), vec![""], None),
            (Some("output"), vec!["a//b"], None),
        ] {
            assert!(lowered(writes, &dirs, executor).is_err(), "{writes:?} {dirs:?} {executor:?}");
        }
    }
}
