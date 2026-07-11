//! CLI-gate exit-code contract. In gate mode (`safe-chains "<cmd>" [--level L]`), a MALFORMED
//! invocation must FAIL CLOSED — a non-zero exit — never exit 0 ("allowed"). Regression for the
//! typo'd-flag fail-open: a clap parse error used to fall through to hook mode, which read empty
//! stdin and exited 0.
use std::io::Write;
use std::process::{Command, Stdio};

/// Run the binary in claude-hook mode (bare, JSON on stdin) and return its stdout.
fn hook_stdout(payload: &str) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(payload.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The overreach nudge must NAME the working directory, so a user who forgot which directory they
/// launched the agent from can spot the mismatch (and the reached path, so they know what it hit).
#[test]
fn overreach_nudge_names_the_working_directory() {
    let payload = r#"{"tool_input":{"command":"cat /other/repo/x.rs"},"cwd":"/work/here"}"#;
    let out = hook_stdout(payload);
    assert!(out.contains("/work/here"), "nudge must NAME the working directory (mismatch cue): {out}");
    assert!(out.contains("/other/repo/x.rs"), "nudge must name the reached path: {out}");
}

fn exit_code(args: &[&str]) -> i32 {
    Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .code()
        .unwrap_or(-1)
}

#[test]
fn cli_gate_fails_closed_on_malformed_invocation() {
    // valid gate: a dangerous command at the inert threshold is REFUSED (1)
    assert_eq!(exit_code(&["rm -rf /", "--level", "inert"]), 1, "valid gate must refuse");
    // valid gate: a safe command is ALLOWED (0)
    assert_eq!(exit_code(&["echo hi", "--level", "inert"]), 0, "valid gate must allow a safe cmd");

    // the fail-open: a TYPO'd flag must NOT exit 0 (it used to, via the hook fallback).
    assert_ne!(exit_code(&["rm -rf /", "--levle", "inert"]), 0, "typo'd flag must FAIL CLOSED");
    // an unknown flag likewise fails closed.
    assert_ne!(exit_code(&["-v"]), 0, "unknown flag must fail closed");
    assert_ne!(exit_code(&["rm -rf /", "--nonsense"]), 0, "unknown long flag must fail closed");

    // --version / --help remain exit 0 (they are not gate decisions).
    assert_eq!(exit_code(&["--version"]), 0, "--version prints and exits 0");
}

/// The upper-band `--level` thresholds (`local-admin`/`network-admin`/`yolo`) classify per-level
/// via the engine, unlocking profiles that the default developer band denies — end to end through
/// the actual `main.rs` flag plumbing (canonical-name resolution + `upper_level_by_name`).
#[test]
fn upper_band_level_thresholds_gate_through_the_cli() {
    // git push origin — a network-admin operation.
    assert_eq!(exit_code(&["git push origin main", "--level", "developer"]), 1, "developer denies push");
    assert_eq!(exit_code(&["git push origin main", "--level", "network-admin"]), 0, "network-admin allows push");
    assert_eq!(exit_code(&["git push origin main", "--level", "yolo"]), 0, "yolo allows push");

    // even yolo refuses the catastrophe corner and an unmodeled command (allowlist-only).
    assert_eq!(exit_code(&["rm -rf /", "--level", "yolo"]), 1, "yolo denies rm -rf /");
    assert_eq!(exit_code(&["frobnicate --wombat", "--level", "yolo"]), 1, "yolo denies an unmodeled command");

    // a plain read is fine at an upper level; the lower band is unchanged.
    assert_eq!(exit_code(&["cat ./README.md", "--level", "network-admin"]), 0, "reads pass at network-admin");
    assert_eq!(exit_code(&["git push origin main", "--level", "reader"]), 1, "reader still denies push");
}
