//! Where a statement-level `cd` takes the shell, for cwd tracking.
//!
//! bash looks a relative name up in CDPATH before the working directory, unless it starts with `/`,
//! `.` or `..`, so once CDPATH may be set `cd tools` can land anywhere a CDPATH entry holds a `tools`.
//! Such a target is left unresolved, and the caller then treats the working directory as unknown.
//! CDPATH counts as set when the command line sets it, or when the environment safe-chains itself
//! runs in holds it, which is often the shell the agent runs commands in. zsh also reads the
//! lowercase array `cdpath`, tied to it.

use super::{Cmd, Pipeline};

/// The target of a statement-level `cd DIR` (a single simple command named `cd`). `None` for
/// anything else, for `cd` with no plain positional (bare `cd`, `cd -`), and for a target CDPATH
/// may redirect.
pub(super) fn cd_target(pipeline: &Pipeline) -> Option<String> {
    let [Cmd::Simple(s)] = pipeline.commands.as_slice() else {
        return None;
    };
    if s.words.first()?.eval() != "cd" {
        return None;
    }
    let prefixed = s.env.iter().any(|(name, _)| name == "CDPATH" || name == "cdpath");
    let target = s.words.iter().skip(1).map(|w| w.eval()).find(|a| !a.starts_with('-'))?;
    (!searches_cdpath(&target, prefixed || cdpath_in_scope())).then_some(target)
}

fn cdpath_in_scope() -> bool {
    crate::pathctx::expand_vars("$CDPATH", false) != "$CDPATH"
        || crate::pathctx::expand_vars("$cdpath", false) != "$cdpath"
        || std::env::var_os("CDPATH").is_some_and(|v| !v.is_empty())
}

fn searches_cdpath(target: &str, cdpath_set: bool) -> bool {
    let anchored =
        target.starts_with(['/', '~']) || target == "." || target == ".." || target.starts_with("./") || target.starts_with("../");
    cdpath_set && !anchored
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn cdpath_redirects_only_an_unanchored_name(name in "[a-z][a-z0-9_.-]{0,8}", prefix in prop::sample::select(vec!["", "/", "~/", "./", "../"])) {
            let target = format!("{prefix}{name}");
            prop_assert!(!searches_cdpath(&target, false), "{target} without CDPATH");
            prop_assert_eq!(searches_cdpath(&target, true), prefix.is_empty(), "{}", target);
        }
    }

    #[test]
    fn the_dot_directories_never_search_cdpath() {
        for target in [".", "..", "./", "../x"] {
            assert!(!searches_cdpath(target, true), "{target}");
        }
        assert!(searches_cdpath("$X", true), "an unexpanded target fails closed");
    }
}
