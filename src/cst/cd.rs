//! Where a statement-level `cd` takes the shell, for cwd tracking.
//!
//! bash looks a relative name up in CDPATH before the working directory, unless it starts with `/`,
//! `.` or `..`, so once CDPATH may be set `cd tools` can land anywhere a CDPATH entry holds a `tools`.
//! Such a target is left unresolved, and the caller then treats the working directory as unknown.
//! CDPATH counts as set when the command line sets it, or when the environment safe-chains itself
//! runs in holds it, which is often the shell the agent runs commands in. zsh also reads the
//! lowercase array `cdpath`, tied to it.

use super::{Cmd, Pipeline, Word, WordPart};

fn plain_literal(word: &Word) -> Option<String> {
    if word.0.is_empty() {
        return None;
    }
    let mut out = String::new();
    for part in &word.0 {
        let WordPart::Lit(s) = part else {
            return None;
        };
        out.push_str(s);
    }
    if out.contains(['$', '*', '?', '[', ']', '{', '}', '`', '\\']) || out.contains("__SAFE_CHAINS_") {
        return None;
    }
    (word.expand().as_slice() == [out.as_str()]).then_some(out)
}

pub(super) fn shadows_cd_or_time(name: &str) -> bool {
    matches!(name, "cd" | "time" | "pushd" | "popd")
}

pub(super) fn cd_target(pipeline: &Pipeline) -> Option<(String, bool)> {
    let [Cmd::Simple(s)] = pipeline.commands.as_slice() else {
        return None;
    };
    if !s.env.is_empty() || super::functions::names_a_function("cd") || super::functions::ever_defined("cd") {
        return None;
    }
    let words: Vec<String> = s.words.iter().map(plain_literal).collect::<Option<_>>()?;
    let mut words = words.into_iter().peekable();
    if words.peek().is_some_and(|word| word == "time") {
        if super::functions::names_a_function("time") || super::functions::ever_defined("time") {
            return None;
        }
        words.next();
        while let Some(flag) = words.peek() {
            match flag.as_str() {
                "-p" => {
                    words.next();
                }
                "--" => {
                    words.next();
                    break;
                }
                _ => break,
            }
        }
    }
    if words.next().as_deref() != Some("cd") {
        return None;
    }
    let mut physical = false;
    let mut args = words;
    let mut target = None;
    for arg in &mut args {
        if arg == "--" {
            target = args.next();
            break;
        }
        if let Some(flags) = arg.strip_prefix('-') {
            if flags.is_empty() || !flags.chars().all(|c| matches!(c, 'L' | 'P' | 'e' | '@')) {
                return None;
            }
            for c in flags.chars() {
                match c {
                    'L' => physical = false,
                    'P' => physical = true,
                    _ => {}
                }
            }
            continue;
        }
        target = Some(arg);
        break;
    }
    if args.next().is_some() {
        return None;
    }
    let target = target?;
    if target == "-" || (target.starts_with('~') && target != "~" && !target.starts_with("~/")) {
        return None;
    }
    (!searches_cdpath(&target, cdpath_in_scope())).then_some((target, physical))
}

pub(super) fn moves_cwd(simple: &super::SimpleCmd) -> bool {
    let mut idx = 0;
    while let Some(word) = simple.words.get(idx) {
        let mut expanded = word.expand();
        let evaluated = word.eval();
        if !expanded.contains(&evaluated) {
            expanded.push(evaluated);
        }
        if expanded.iter().any(|w| {
            let base = w.rsplit('/').next().unwrap_or(w);
            matches!(base, "cd" | "pushd" | "popd")
                || (base == "time" && (super::functions::names_a_function("time") || super::functions::ever_defined("time")))
                || w.contains(['$', '*', '?'])
                || (w.contains('[') && w.contains(']'))
                || w.contains("__SAFE_CHAINS_")
        }) {
            return true;
        }
        if expanded.iter().any(|w| matches!(w.rsplit('/').next().unwrap_or(w), "alias" | "hash")) {
            return simple.words[idx + 1..].iter().any(|arg| {
                let v = arg.eval();
                v.contains('=') || v.contains("cd") || v.contains("time") || v.contains("pushd") || v.contains("popd")
            });
        }
        let is_wrapper = expanded
            .iter()
            .any(|w| matches!(w.rsplit('/').next().unwrap_or(w), "time" | "command" | "builtin" | "env"));
        if !is_wrapper {
            return false;
        }
        idx += 1;
        while let Some(next) = simple.words.get(idx) {
            let text = next.eval();
            if text == "--" {
                idx += 1;
                break;
            }
            if text.starts_with('-') || text.contains('=') {
                idx += 1;
            } else {
                break;
            }
        }
    }
    false
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
