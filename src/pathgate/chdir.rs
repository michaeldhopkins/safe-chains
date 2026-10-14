//! A global flag that moves the command into another directory before it reads anything
//! (`git -C DIR`). Every relative operand the command then names is relative to DIR, not to the
//! caller's cwd, so `git -C /etc diff shadow` reads `/etc/shadow` while its operand spells a
//! worktree file. Declared per command in `pathgates.toml`'s `[chdir]` table.
//!
//! The gate runs the command's ordinary path gates with the ambient cwd moved to DIR, the way
//! intra-line `cd` tracking does (`pathctx::enter_cwd`), so every role — flag values, positionals,
//! sub-scoped gates and handlers — resolves against the directory the tool will really be in. DIR
//! itself is gated as a READ first: git renders the content of the repository it is pointed at
//! (`git -C ~/.ssh log -p`), whatever the operands say. A read, not a tree: `git -C /other/repo
//! status` stays as approvable as `cd /other/repo && git status`.

use std::collections::HashSet;

use serde::Deserialize;

use super::{Role, gate};
use crate::parse::Token;

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(crate) struct Chdir {
    /// The directory-changing global flag.
    flag: String,
    /// The command's OTHER global flags that take a value, so the scan steps over their values
    /// instead of mistaking one for the subcommand and stopping early.
    #[serde(default)]
    valued: HashSet<String>,
}

/// Every directory the global options name, in order. Stops at the first bare token: that is the
/// subcommand, and a `-C` after it belongs to the subcommand (`git grep -C 3`).
fn dirs<'a>(spec: &Chdir, tokens: &'a [Token]) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        if t == spec.flag {
            out.extend(tokens.get(i + 1).map(Token::as_str));
            i += 2;
        } else if let Some(v) = t.strip_prefix(spec.flag.as_str()).and_then(|r| r.strip_prefix('=')) {
            out.push(v);
            i += 1;
        } else if spec.valued.contains(t) {
            i += 2;
        } else if t.starts_with('-') {
            i += 1;
        } else {
            break;
        }
    }
    out
}

/// The cwd after moving into `dir` from `cur`. A relative move from an unknown cwd stays unknown
/// (relative paths keep their worktree reading, as after a relative `cd`), except one that climbs:
/// nothing can say where `..` lands from nowhere, so it leaves the cwd unresolved.
fn moved(cur: Option<String>, dir: &str) -> Option<String> {
    crate::pathctx::join_cwd(cur.as_deref(), dir)
        .or_else(|| if dir.split('/').any(|seg| seg == "..") { Some(crate::pathctx::UNRESOLVED_CWD.to_string()) } else { cur })
}

/// Whether the command must be refused: a named directory is a sensitive tree, or `rest` (the
/// command's own gates) denies once judged from the directory the command will really run in.
pub(super) fn should_deny(spec: Option<&Chdir>, tokens: &[Token], rest: impl FnOnce() -> bool) -> bool {
    let Some(spec) = spec else { return rest() };
    let named = dirs(spec, tokens);
    if named.is_empty() {
        return rest();
    }
    let mut cwd = crate::pathctx::cwd();
    for dir in named.into_iter().filter(|d| !d.is_empty()).map(spelled) {
        let _here = crate::pathctx::enter_cwd(cwd.clone());
        if gate(Role::Read, &dir) {
            return true;
        }
        cwd = moved(cwd.clone(), &dir);
    }
    let _there = crate::pathctx::enter_cwd(cwd);
    rest()
}

/// The two variables a shell always sets to a directory, spelled the way the gate already reads:
/// `git -C "$PWD"` stays where it is and `git -C "$HOME/x"` is `~/x`. Any other variable stays
/// unresolved, and the read gate refuses it.
fn spelled(dir: &str) -> String {
    for (var, as_path) in [("$PWD", "."), ("${PWD}", "."), ("$HOME", "~"), ("${HOME}", "~")] {
        if let Some(rest) = dir.strip_prefix(var)
            && (rest.is_empty() || rest.starts_with('/'))
        {
            return format!("{as_path}{rest}");
        }
    }
    dir.to_string()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{Chdir, dirs};
    use crate::parse::Token;
    use crate::pathctx::PathCtx;

    const ROOT: &str = "/work/projects/app";

    fn toks(words: &[&str]) -> Vec<Token> {
        words.iter().map(|w| Token::from_raw((*w).to_string())).collect()
    }

    fn git() -> Chdir {
        Chdir { flag: "-C".into(), valued: ["-c".to_string()].into() }
    }

    fn denied(cmd: &str) -> bool {
        let ctx = PathCtx { cwd: Some(ROOT.into()), root: Some(ROOT.into()), session_id: None };
        !crate::command_verdict_in(cmd, ctx).is_allowed()
    }

    #[test]
    fn the_scan_collects_every_global_directory_and_stops_at_the_subcommand() {
        let t = toks(&["git", "-c", "-C", "-C", "a", "-C=b", "grep", "-C", "3", "x"]);
        assert_eq!(dirs(&git(), &t), vec!["a", "b"]);
        assert!(dirs(&git(), &toks(&["git", "grep", "-C", "/etc", "x"])).is_empty());
    }

    #[test]
    fn a_relative_operand_is_judged_where_the_command_moved_to() {
        for cmd in [
            "git -C /etc diff shadow", "git -C /etc diff --no-index shadow /dev/null", "git -C / diff etc/shadow",
            "git -C /etc -C . diff shadow", "git -C / -C etc diff shadow", "git -C ~ diff --no-index .ssh/id_rsa /dev/null",
            "git -C /etc diff -O shadow", "git -C /etc grep -f shadow x", "git -C sub diff ../../../../etc/shadow",
        ] {
            assert!(denied(cmd), "{cmd}");
        }
    }

    #[test]
    fn the_directory_itself_is_gated_as_a_read() {
        for cmd in ["git -C ~/.ssh log", "git -C ~/.aws status", "git -C $DIR log -p", "git -C ~/.gnupg log -p"] {
            assert!(denied(cmd), "{cmd}");
        }
        for cmd in ["git -C $HOME/.ssh log -p", "git -C ${HOME}/.aws status", "git -C $PWD/../../../etc diff shadow"] {
            assert!(denied(cmd), "{cmd}");
        }
        for cmd in ["git -C /some/repo status", "git -C ~/projects/other log -p", "git -C \"$PWD\" log --oneline", "git -C $HOME status"] {
            assert!(!denied(cmd), "{cmd}");
        }
    }

    #[test]
    fn ordinary_moves_inside_the_workspace_still_approve() {
        for cmd in ["git -C sub diff src/x.rs", "git -C . log --oneline", "git -C sub/dir status", "git -C sub grep -n pat"] {
            assert!(!denied(cmd), "{cmd}");
        }
    }

    #[test]
    fn a_climb_from_an_unknown_cwd_is_left_unresolved() {
        assert_eq!(super::moved(None, "sub"), None);
        assert_eq!(super::moved(None, "../x").as_deref(), Some(crate::pathctx::UNRESOLVED_CWD));
        assert_eq!(super::moved(None, "/etc").as_deref(), Some("/etc"));
        assert_eq!(super::moved(Some(ROOT.into()), "sub").as_deref(), Some("/work/projects/app/sub"));
    }

    proptest! {
        /// Moving into a directory and naming a file there is the same read as naming the joined
        /// path from where we started: a `-C` can never launder an operand into the worktree.
        #[test]
        fn a_move_and_an_operand_judge_as_their_join(
            dir in prop::sample::select(vec!["/etc", "/", "~", "~/.ssh", "sub", "..", "../..", "/tmp", "/work/projects"]),
            file in prop::sample::select(vec!["shadow", "etc/shadow", ".ssh/id_rsa", "id_rsa", "x.rs", "../etc/passwd", "hosts"]),
        ) {
            let moved = format!("git -C {dir} diff --no-index {file} /dev/null");
            let joined = format!("git diff --no-index {dir}/{file} /dev/null");
            prop_assert!(denied(&moved) || !denied(&joined), "{moved} approved while {joined} denies");
        }
    }
}
