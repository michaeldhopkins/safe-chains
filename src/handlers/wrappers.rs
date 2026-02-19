use std::collections::HashSet;
use std::sync::LazyLock;

static TIMEOUT_FLAGS_WITH_ARG: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["-s", "--signal", "-k", "--kill-after"]));

const SELF_TEST: &str = "test_safe_chains.py";

pub fn is_safe_env(tokens: &[String], is_safe: &dyn Fn(&str) -> bool) -> bool {
    if tokens.len() == 1 {
        return true;
    }
    let mut i = 1;
    while i < tokens.len() && tokens[i].starts_with('-') {
        if tokens[i] == "-i" || tokens[i] == "--ignore-environment" {
            i += 1;
        } else if tokens[i] == "-u" || tokens[i] == "--unset" {
            i += 2;
        } else {
            i += 1;
        }
    }
    while i < tokens.len() && !tokens[i].starts_with('-') && tokens[i].contains('=') {
        i += 1;
    }
    if i >= tokens.len() {
        return true;
    }
    let inner = tokens[i..].join(" ");
    is_safe(&inner)
}

pub fn is_safe_python(tokens: &[String]) -> bool {
    if tokens.len() != 2 {
        return false;
    }
    let script = tokens[1].rsplit('/').next().unwrap_or(&tokens[1]);
    script == SELF_TEST
}

pub fn is_safe_timeout(tokens: &[String], is_safe: &dyn Fn(&str) -> bool) -> bool {
    let mut i = 1;
    while i < tokens.len() && tokens[i].starts_with('-') {
        if TIMEOUT_FLAGS_WITH_ARG.contains(tokens[i].as_str()) {
            i += 2;
        } else {
            i += 1;
        }
    }
    i += 1;
    if i >= tokens.len() {
        return false;
    }
    let inner = tokens[i..].join(" ");
    is_safe(&inner)
}

pub fn is_safe_time(tokens: &[String], is_safe: &dyn Fn(&str) -> bool) -> bool {
    let mut i = 1;
    if i < tokens.len() && tokens[i] == "-p" {
        i += 1;
    }
    if i >= tokens.len() {
        return false;
    }
    let inner = tokens[i..].join(" ");
    is_safe(&inner)
}

pub fn is_safe_nice(tokens: &[String], is_safe: &dyn Fn(&str) -> bool) -> bool {
    let mut i = 1;
    while i < tokens.len() && tokens[i].starts_with('-') {
        if tokens[i] == "-n" || tokens[i] == "--adjustment" {
            i += 2;
        } else {
            i += 1;
        }
    }
    if i >= tokens.len() {
        return false;
    }
    let inner = tokens[i..].join(" ");
    is_safe(&inner)
}

#[cfg(test)]
mod tests {
    use crate::is_safe;

    fn check(cmd: &str) -> bool {
        is_safe(cmd)
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
    fn python_self_test() {
        assert!(check("python3 test_safe_chains.py"));
    }

    #[test]
    fn python_self_test_path() {
        assert!(check("python3 /some/path/test_safe_chains.py"));
    }

    #[test]
    fn python_self_test_python2() {
        assert!(check("python test_safe_chains.py"));
    }

    #[test]
    fn python_with_args_denied() {
        assert!(!check("python3 test_safe_chains.py --verbose"));
    }

    #[test]
    fn python_script_denied() {
        assert!(!check("python3 script.py"));
    }

    #[test]
    fn python_manage_denied() {
        assert!(!check("python3 manage.py"));
    }

    #[test]
    fn python_c_denied() {
        assert!(!check("python3 -c 'import os'"));
    }

    #[test]
    fn bare_python_denied() {
        assert!(!check("python3"));
    }

    #[test]
    fn python_other_test_denied() {
        assert!(!check("python3 test_other.py"));
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
}
