//! `jj -R`/`--repository` names the repository a subcommand acts on. Every jj command snapshots
//! the working copy first, which writes into that repository's store, so the value is a write
//! target. `--ignore-working-copy` skips the snapshot, and a subcommand that only observes then
//! writes nothing, so for those the value is only read. Without -R jj acts on the repository the
//! working directory sits in, which after `cd DIR` is DIR, so the working directory is judged with
//! the same role, even when -R is given, which fails closed. The one exception is a session whose
//! root is itself a trust root directory: the session works there. A repository write
//! reaches every file beneath it, a trust file among them (`jj restore` rewrites one), so no grant
//! opens a write to a repository at a trust root such as `~/.claude`.

use crate::parse::Token;
use crate::pathgate::{Role, RoleSpec, Shape, walk};
use std::collections::HashMap;

const STANDALONE_GLOBALS: &[&str] = &["--debug", "--ignore-immutable", "--ignore-working-copy", "--no-pager", "--quiet", "--verbose"];

const VALUED_GLOBALS: &[&str] = &["--at-op", "--at-operation", "--color", "--repository", "-R"];

const OBSERVING_SUBS: &[&[&str]] = &[
    &["cat"],
    &["diff"],
    &["help"],
    &["log"],
    &["root"],
    &["show"],
    &["st"],
    &["status"],
    &["version"],
    &["bookmark", "list"],
    &["config", "get"],
    &["config", "list"],
    &["file", "list"],
    &["file", "show"],
    &["git", "remote", "list"],
    &["op", "log"],
    &["tag", "list"],
    &["workspace", "list"],
    &["workspace", "root"],
];

pub(super) fn jj_repository(tokens: &[Token]) -> bool {
    let role = if reads_without_snapshot(tokens) { Role::Read } else { Role::Write };
    let flags: HashMap<String, Role> = [("-R", role), ("--repository", role), ("--destination", Role::Ignore)]
        .into_iter()
        .map(|(f, r)| (f.to_string(), r))
        .collect();
    let spec = RoleSpec { positional: Role::Ignore, shape: Shape::Plain, flags, handler: None, write_when: Vec::new(), when: Vec::new() };
    let moved = moved_from_root();
    walk(&spec, tokens)
        || (judges_here(moved) && crate::pathgate::judge(role, ".") == crate::verdict::Verdict::Denied)
        || (role == Role::Write && writes_a_trust_root(tokens, moved))
}

fn judges_here(moved: bool) -> bool {
    moved || crate::engine::resolve::frozen_write_kind(".") != Some(crate::engine::resolve::FrozenWrite::TrustRootDir)
}

fn moved_from_root() -> bool {
    let cwd = crate::pathctx::cwd();
    let trim = |p: &Option<String>| p.as_deref().map(|p| p.trim_end_matches('/').to_string());
    cwd.is_some() && trim(&cwd) != trim(&crate::pathctx::root())
}

fn writes_a_trust_root(tokens: &[Token], moved: bool) -> bool {
    repositories(tokens, moved)
        .iter()
        .any(|path| crate::engine::resolve::frozen_write_kind(trim_slashes(path)).is_some())
}

fn trim_slashes(path: &str) -> &str {
    match path.trim_end_matches('/') {
        "" if path.starts_with('/') => "/",
        trimmed => trimmed,
    }
}

fn repositories(tokens: &[Token], moved: bool) -> Vec<String> {
    let words: Vec<&str> = tokens.iter().skip(1).map(Token::as_str).take_while(|t| *t != "--").collect();
    let mut found = if moved { vec![".".to_string()] } else { Vec::new() };
    let mut i = 0;
    while i < words.len() {
        let word = words[i];
        i += 1;
        if word == "-R" || word == "--repository" {
            found.extend(words.get(i).map(|v| (*v).to_string()));
            i += 1;
        } else if let Some(value) = word.strip_prefix("--repository=") {
            found.push(value.to_string());
        } else if let Some(value) = word.strip_prefix("-R").filter(|v| !v.is_empty()) {
            found.push(value.to_string());
        } else if VALUED_GLOBALS.contains(&word) {
            i += 1;
        }
    }
    found
}

fn reads_without_snapshot(tokens: &[Token]) -> bool {
    let words: Vec<&str> = tokens.iter().skip(1).map(Token::as_str).take_while(|t| *t != "--").collect();
    words.contains(&"--ignore-working-copy") && names_an_observing_sub(&words)
}

fn names_an_observing_sub(words: &[&str]) -> bool {
    let mut path: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < words.len() {
        let word = words[i];
        i += 1;
        if word.starts_with('-') {
            if STANDALONE_GLOBALS.contains(&word) {
                continue;
            }
            if VALUED_GLOBALS.contains(&word) {
                i += 1;
                continue;
            }
            let glued_long = word.split_once('=').is_some_and(|(head, _)| VALUED_GLOBALS.contains(&head));
            if glued_long || (word.starts_with("-R") && word.len() > 2) {
                continue;
            }
            return false;
        }
        path.push(word);
        if OBSERVING_SUBS.contains(&path.as_slice()) {
            return true;
        }
        if !OBSERVING_SUBS.iter().any(|sub| sub.starts_with(&path)) {
            return false;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn toks(words: &[&str]) -> Vec<Token> {
        words.iter().map(|s| Token::from_test(s)).collect()
    }

    fn vocabulary() -> impl Strategy<Value = Vec<&'static str>> {
        let words = prop::sample::select(vec![
            "--ignore-working-copy", "--quiet", "--no-pager", "-R", "--repository", "--color", "always", "-r", "@", "--", "-Rrepo",
            "--repository=repo", "repo", "log", "diff", "status", "bookmark", "list", "set", "main", "config", "get", "op", "git",
            "remote", "workspace", "root", "new", "describe", "-m", "x", "file", "show", "--bogus",
        ]);
        prop::collection::vec(words, 0..8)
    }

    proptest! {
        #[test]
        fn the_repository_is_read_only_with_the_flag_and_an_observing_sub(rest in vocabulary()) {
            let mut words = vec!["jj"];
            words.extend(rest.iter().copied());
            let tokens = toks(&words);
            let before_end: Vec<&str> = rest.iter().copied().take_while(|w| *w != "--").collect();
            if reads_without_snapshot(&tokens) {
                prop_assert!(before_end.contains(&"--ignore-working-copy"));
                let observed = OBSERVING_SUBS.iter().any(|sub| sub.iter().all(|w| before_end.contains(w)));
                prop_assert!(observed, "{words:?} read as an observing sub");
                let without: Vec<&str> = words.iter().copied().filter(|w| *w != "--ignore-working-copy").collect();
                prop_assert!(!reads_without_snapshot(&toks(&without)));
            }
        }
    }

    #[test]
    fn a_mutating_sub_keeps_the_write_role_with_the_flag() {
        for words in [
            &["jj", "--ignore-working-copy", "-R", "r", "bookmark", "set", "main"][..],
            &["jj", "--ignore-working-copy", "-R", "r", "new"],
            &["jj", "-R", "r", "--color", "log", "new", "--ignore-working-copy"],
            &["jj", "--ignore-working-copy", "-R", "r", "workspace", "update-stale"],
            &["jj", "--ignore-working-copy", "--bogus", "log"],
            &["jj", "log", "--", "--ignore-working-copy"],
        ] {
            assert!(!reads_without_snapshot(&toks(words)), "{words:?}");
        }
        for words in [
            &["jj", "--ignore-working-copy", "-R", "r", "log"][..],
            &["jj", "-Rr", "bookmark", "--ignore-working-copy", "list"],
            &["jj", "--repository=r", "git", "remote", "list", "--ignore-working-copy"],
            &["jj", "--color", "never", "op", "log", "--ignore-working-copy"],
        ] {
            assert!(reads_without_snapshot(&toks(words)), "{words:?}");
        }
    }
}
