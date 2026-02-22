use safe_chains::is_safe_command;
use safe_chains::parse::{CommandLine, Segment};

fn check(cmd: &str) -> bool {
    is_safe_command(cmd)
}

fn seg(s: &str) -> Segment {
    let segs = CommandLine::new(s).segments();
    assert_eq!(segs.len(), 1, "expected single segment: {s}");
    segs.into_iter().next().unwrap()
}

// ── strip_env_prefix with quoted values ──

#[test]
fn env_prefix_quoted_value_simple() {
    let s = seg("FOO='bar baz' ls");
    let stripped = s.strip_env_prefix();
    assert_eq!(stripped.as_str(), "ls");
}

#[test]
fn env_prefix_quoted_value_with_equals() {
    let s = seg("FOO='a=b' ls");
    let stripped = s.strip_env_prefix();
    assert_eq!(stripped.as_str(), "ls");
}

#[test]
fn env_prefix_double_quoted() {
    let s = seg("FOO=\"bar baz\" ls");
    let stripped = s.strip_env_prefix();
    assert_eq!(stripped.as_str(), "ls");
}

#[test]
fn env_prefix_quoted_safe_command() {
    assert!(check("FOO='bar baz' ls -la"));
}

#[test]
fn env_prefix_quoted_unsafe_denied() {
    assert!(!check("FOO='bar baz' rm -rf /"));
}

// ── /dev/null redirect tokens leaking to handlers ──

#[test]
fn awk_dev_null_redirect() {
    // awk handler checks for ">" in all tokens — /dev/null redirect leaks through
    assert!(check("awk '{print $1}' file.txt > /dev/null"));
}

#[test]
fn sed_dev_null_redirect() {
    assert!(check("sed 's/foo/bar/' file.txt > /dev/null"));
}

#[test]
fn grep_dev_null_redirect() {
    assert!(check("grep pattern file > /dev/null"));
}

#[test]
fn git_log_dev_null_redirect() {
    assert!(check("git log > /dev/null"));
}

#[test]
fn echo_dev_null_redirect() {
    assert!(check("echo hello > /dev/null"));
}

#[test]
fn echo_stderr_dev_null() {
    assert!(check("echo hello 2> /dev/null"));
}

// ── multi-digit fd redirects ──

#[test]
fn multi_digit_fd_redirect() {
    assert!(check("ls 10>&1"));
    assert!(check("cargo clippy 255>&2"));
}

#[test]
fn multi_digit_dev_null_redirect() {
    assert!(check("echo hello 10>/dev/null"));
    assert!(check("ls 255>/dev/null"));
}

// ── numeric args not swallowed by redirect filter ──

#[test]
fn numeric_arg_not_swallowed_by_redirect_filter() {
    assert!(check("head -n 42 /dev/null"));
    assert!(check("head -42 /dev/null"));
    assert!(check("tail -100 /dev/null"));
}

// ── edge cases ──

#[test]
fn command_with_path_traversal() {
    assert!(!check("/usr/bin/../../../etc/shadow"));
}

#[test]
fn command_with_simple_path() {
    assert!(check("/usr/bin/ls -la"));
}

#[test]
fn has_flag_unicode_no_panic() {
    let _result = check("sed -é 's/foo/bar/'");
}

#[test]
fn heredoc_blocked() {
    assert!(!check("cat << EOF"));
}
