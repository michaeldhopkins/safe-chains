//! `fd` is a read-only file finder, EXCEPT `-x/--exec` and `-X/--exec-batch`, which run a command
//! per result (or once for all results) — the same arbitrary-command entry point as `find -exec`.
//! So this handler delegates the inner command to the classifier, bound to each search path, and
//! routes the non-exec case to the TOML flag grammar. All flag data lives in
//! `commands/search/fd.toml`; this file is dispatch logic only.
use crate::parse::Token;
use crate::registry;
use crate::verdict::{SafetyLevel, Verdict};

const EXEC_FLAGS: &[&str] = &["-x", "--exec", "-X", "--exec-batch"];

pub fn check_fd(tokens: &[Token]) -> Verdict {
    let Some(xi) = tokens.iter().position(|t| EXEC_FLAGS.contains(&t.as_str())) else {
        // No exec: a plain search — validate flags against the TOML grammar.
        return registry::try_fallback_grammar("fd", tokens).unwrap_or(Verdict::Denied);
    };
    // The pre-exec portion is an ordinary fd search; its flags must still be valid (an unknown
    // flag denies) — reuse the same grammar on just that slice.
    match registry::try_fallback_grammar("fd", &tokens[..xi]) {
        Some(Verdict::Allowed(_)) => {}
        _ => return Verdict::Denied,
    }
    // The exec command runs to a `;`/`+` terminator or the end of the line.
    let cmd_start = xi + 1;
    let cmd_end = tokens[cmd_start..]
        .iter()
        .position(|t| *t == ";" || *t == "+")
        .map(|p| cmd_start + p)
        .unwrap_or(tokens.len());
    if cmd_start >= cmd_end {
        return Verdict::Denied;
    }
    // `{}` is substituted with each matched path, which lives UNDER fd's search path(s). Bind it to
    // a path under each — deny-absorbing, so a system/home search denies. Every bare pre-exec operand
    // is a candidate base (the pattern is benign worktree, `--search-path`/`--base-directory` VALUES
    // are real bases), plus fd's default `.`.
    let mut bases: Vec<&str> = tokens[1..xi]
        .iter()
        .map(Token::as_str)
        .filter(|s| !s.starts_with('-'))
        .collect();
    bases.push(".");
    // With no `{}` placeholder, fd APPENDS each matched path as a trailing argument (`fd -X cat`
    // runs `cat <matches>`), so the path must still be bound in that form or a `/etc` search leaks.
    let has_placeholder =
        tokens[cmd_start..cmd_end].iter().any(|t| t.as_str().contains('{'));
    let mut level = SafetyLevel::Inert;
    for base in &bases {
        let bound = format!("{}/f", base.trim_end_matches('/'));
        let mut words: Vec<String> = tokens[cmd_start..cmd_end]
            .iter()
            .map(|t| bind_placeholders(t.as_str(), &bound))
            .collect();
        if !has_placeholder {
            words.push(bound.clone());
        }
        match crate::command_verdict(&shell_words::join(&words)) {
            Verdict::Denied => return Verdict::Denied,
            Verdict::Allowed(l) => level = level.max(l),
        }
    }
    Verdict::Allowed(level)
}

/// fd's placeholders (`{}` full path, `{.}` no-ext, `{/}` basename, `{//}` parent, `{/.}` basename
/// no-ext) all name the matched file — bind each to the traversal-bound path. Longest tokens first
/// so `{//}` isn't partially eaten by `{}`.
fn bind_placeholders(tok: &str, bound: &str) -> String {
    tok.replace("{//}", bound)
        .replace("{/.}", bound)
        .replace("{/}", bound)
        .replace("{.}", bound)
        .replace("{}", bound)
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        // plain search (the non-exec fallback grammar)
        bare: "fd",
        pattern: "fd pattern",
        flags: "fd --type f --hidden pattern",
        pattern_and_path: "fd pattern src",
        ext_valued: "fd -e rs",
        // exec delegates to a safe inner command, bound under the worktree
        exec_echo: "fd -x echo {}",
        exec_wc: "fd pattern -x wc -l {}",
        exec_batch_cat_worktree: "fd -X cat",
        exec_rm_worktree: "fd -x rm {}", // worktree destroy is editor-level (like `rm ./f`)
        exec_placeholder_variants: "fd -x cat {//}/{/}",
    }

    denied! {
        // exec into a SYSTEM search path denies through the inner command's locus
        exec_system_read: "fd /etc -x cat {}",
        exec_batch_system: "fd /etc -X cat",
        exec_system_glued_placeholder: "fd -x rm /etc/{}",
        exec_search_path_flag_system: "fd --search-path /etc -X cat",
        // an unknown search flag before the exec still denies
        unknown_flag: "fd --evil",
        unknown_flag_before_exec: "fd --evil -x echo {}",
        // a dangling exec with no command
        empty_exec: "fd -x",
    }

    use proptest::prelude::*;
    proptest! {
        /// fd's `-x`/`-X` is a delegation flag exactly like `find -exec`: the finder's verdict must
        /// FOLLOW the inner command bound to the search path. A read/write inner command over a SYSTEM
        /// base must deny; the same over the worktree must allow. Guards the class the way find's own
        /// exec tests do — a regression that stopped delegating (allowing everything, or denying
        /// everything) fails here.
        #[test]
        fn fd_exec_follows_the_inner_command_locus(
            inner in prop::sample::select(vec!["cat", "od", "base64", "wc -l"]),
        ) {
            // System base → the inner read of a system path denies.
            let sys = format!("fd /etc -x {} {{}}", inner);
            prop_assert!(!crate::is_safe_command(&sys), "system search must deny: {}", sys);
            // Worktree base → the same read allows.
            let wt = format!("fd src -x {} {{}}", inner);
            prop_assert!(crate::is_safe_command(&wt), "worktree search must allow: {}", wt);
        }

        /// A batch exec with NO placeholder (`fd <base> -X <cmd>`) appends the match as a trailing arg;
        /// it must gate the base identically to the placeholder form — never a silent leak.
        #[test]
        fn fd_batch_without_placeholder_gates_the_base(
            reader in prop::sample::select(vec!["cat", "od", "base64"]),
        ) {
            let sys = format!("fd /etc -X {}", reader);
            prop_assert!(!crate::is_safe_command(&sys), "batch system read must deny: {}", sys);
        }
    }
}
