//! End-to-end subprocess tests for the runtime hook surface.
//!
//! Each test spawns the compiled `safe-chains` binary, pipes a vendor-shaped
//! JSON envelope into stdin, and asserts on the binary's stdout/exit code.
//! This is what every hook integration ultimately exercises in production:
//! the agent CLI dumps JSON, we run, we respond. Catching regressions here
//! catches them before they reach a real Claude Code / Codex session.

#![allow(clippy::unwrap_used)]

use std::io::Write;
use std::process::{Command, Stdio};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_safe-chains")
}

fn run_hook(args: &[&str], stdin_payload: &str) -> (String, String, i32) {
    let mut child = Command::new(binary())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn safe-chains");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin_payload.as_bytes())
        .unwrap();
    let out = child.wait_with_output().expect("wait");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

// ---------------------------------------------------------------
// Claude Code format (default for bare invocation + `hook claude`)
// ---------------------------------------------------------------

#[test]
fn claude_default_invocation_allows_safe_command() {
    let payload = r#"{"tool_input": {"command": "ls -la"}}"#;
    let (stdout, _stderr, code) = run_hook(&[], payload);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(
        v.pointer("/hookSpecificOutput/permissionDecision")
            .and_then(|d| d.as_str()),
        Some("allow"),
    );
}

#[test]
fn claude_default_invocation_denies_unsafe_command() {
    let payload = r#"{"tool_input": {"command": "rm -rf /"}}"#;
    let (stdout, _stderr, code) = run_hook(&[], payload);
    // Hook protocol: empty stdout + exit 0 means "no opinion" → Claude
    // Code falls back to its own permission rules. That's the contract,
    // not an error.
    assert_eq!(code, 0);
    assert_eq!(stdout, "");
}

#[test]
fn claude_explicit_subcommand_allows_safe_command() {
    let payload = r#"{"tool_input": {"command": "git status"}}"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "claude"], payload);
    assert_eq!(code, 0);
    assert!(stdout.contains("\"permissionDecision\":\"allow\""));
}

#[test]
fn claude_invalid_json_exits_zero_silently() {
    let (stdout, _stderr, code) = run_hook(&[], "not json at all");
    assert_eq!(code, 0);
    assert_eq!(stdout, "");
}

#[test]
fn claude_safewrite_carries_appropriate_reason() {
    let payload = r#"{"tool_input": {"command": "cargo build"}}"#;
    let (stdout, _stderr, code) = run_hook(&[], payload);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).expect(&stdout);
    let reason = v
        .pointer("/hookSpecificOutput/permissionDecisionReason")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    assert!(
        reason.contains("safe utilities"),
        "reason was: {reason}"
    );
}

// ---------------------------------------------------------------
// Codex format
// ---------------------------------------------------------------

#[test]
fn codex_hook_allows_safe_command() {
    let payload = r#"{"tool_name": "Bash", "tool_input": {"command": "ls -la"}}"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "codex"], payload);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(
        v.pointer("/hookSpecificOutput/permissionDecision")
            .and_then(|d| d.as_str()),
        Some("allow"),
    );
    assert_eq!(
        v.pointer("/hookSpecificOutput/hookEventName")
            .and_then(|d| d.as_str()),
        Some("PreToolUse"),
    );
}

#[test]
fn codex_hook_denies_unsafe_command() {
    let payload = r#"{"tool_name": "Bash", "tool_input": {"command": "rm -rf /etc"}}"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "codex"], payload);
    assert_eq!(code, 0);
    assert_eq!(stdout, "");
}

#[test]
fn codex_hook_handles_optional_cwd() {
    let payload = r#"{"tool_input": {"command": "git status"}, "cwd": "/Users/me/project"}"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "codex"], payload);
    assert_eq!(code, 0);
    assert!(stdout.contains("\"permissionDecision\":\"allow\""));
}

#[test]
fn codex_hook_invalid_json_exits_zero_silently() {
    let (stdout, _stderr, code) = run_hook(&["hook", "codex"], "{");
    assert_eq!(code, 0);
    assert_eq!(stdout, "");
}

// ---------------------------------------------------------------
// Tooling errors
// ---------------------------------------------------------------

#[test]
fn unknown_tool_in_hook_subcommand_errors() {
    let (_stdout, stderr, code) = run_hook(&["hook", "made-up-tool"], "{}");
    assert_eq!(code, 1);
    assert!(
        stderr.contains("Unknown tool"),
        "stderr was: {stderr}"
    );
}

#[test]
fn list_tools_includes_every_supported_target() {
    let out = Command::new(binary())
        .arg("--list-tools")
        .output()
        .expect("run");
    let s = String::from_utf8_lossy(&out.stdout);
    for tool in ["claude", "codex", "cursor", "gemini", "copilot", "qwen", "droid", "opencode"] {
        assert!(s.contains(tool), "list-tools missing `{tool}`: {s}");
    }
}

// ---------------------------------------------------------------
// Cursor format — top-level `command`, `permission` key
// ---------------------------------------------------------------

#[test]
fn cursor_hook_allows_safe_command() {
    // Verbatim shape from cursor.com/docs/hooks for
    // `beforeShellExecution`.
    let payload = r#"{
        "conversation_id": "abc-123",
        "generation_id": "gen-456",
        "model": "claude-sonnet-4-5",
        "hook_event_name": "beforeShellExecution",
        "cursor_version": "2.0.43",
        "workspace_roots": ["/Users/me/project"],
        "user_email": null,
        "transcript_path": null,
        "command": "ls -la",
        "cwd": "/Users/me/project",
        "sandbox": false
    }"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "cursor"], payload);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(
        v.get("permission").and_then(|s| s.as_str()),
        Some("allow"),
    );
    assert!(v.get("permissionDecision").is_none(), "no Claude wrapper");
    assert!(v.get("decision").is_none(), "no Gemini wrapper");
}

#[test]
fn cursor_hook_denies_unsafe_command() {
    let payload = r#"{"command": "rm -rf /etc", "cwd": "/x"}"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "cursor"], payload);
    assert_eq!(code, 0);
    assert_eq!(stdout, "");
}

// ---------------------------------------------------------------
// Gemini format — `BeforeTool` event, `decision` key, no `ask`
// ---------------------------------------------------------------

#[test]
fn gemini_hook_allows_safe_command() {
    let payload = r#"{
        "session_id": "abc",
        "transcript_path": "/t",
        "cwd": "/p",
        "hook_event_name": "BeforeTool",
        "timestamp": "2026-05-06T12:00:00Z",
        "tool_name": "run_shell_command",
        "tool_input": {"command": "ls -la"}
    }"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "gemini"], payload);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v.get("decision").and_then(|s| s.as_str()), Some("allow"));
    assert!(v.get("permission").is_none(), "no Cursor wrapper");
    assert!(v.get("permissionDecision").is_none(), "no Claude wrapper");
}

#[test]
fn gemini_hook_skips_non_shell_tool() {
    // Gemini's matcher is regex on tool name. If a non-shell tool
    // somehow dispatches through, we shouldn't gate it.
    let payload = r#"{"tool_name": "list_files", "tool_input": {"command": "ignored"}}"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "gemini"], payload);
    assert_eq!(code, 0);
    assert_eq!(stdout, "");
}

// ---------------------------------------------------------------
// Qwen format — Claude-shaped envelope with `^Bash$` matcher
// ---------------------------------------------------------------

#[test]
fn qwen_hook_allows_safe_command() {
    let payload = r#"{
        "session_id": "abc",
        "transcript_path": "/t",
        "cwd": "/p",
        "hook_event_name": "PreToolUse",
        "timestamp": "2026-05-06T12:00:00Z",
        "permission_mode": "default",
        "tool_name": "Bash",
        "tool_input": {"command": "ls -la"},
        "tool_use_id": "tu_1"
    }"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "qwen"], payload);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(
        v.pointer("/hookSpecificOutput/permissionDecision")
            .and_then(|d| d.as_str()),
        Some("allow"),
    );
}

// ---------------------------------------------------------------
// Droid format — `Execute` matcher, Claude-shaped output
// ---------------------------------------------------------------

#[test]
fn droid_hook_allows_safe_command() {
    let payload = r#"{
        "session_id": "abc",
        "transcript_path": "/t",
        "cwd": "/p",
        "permission_mode": "off",
        "hook_event_name": "PreToolUse",
        "tool_name": "Execute",
        "tool_input": {"command": "ls -la"}
    }"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "droid"], payload);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(
        v.pointer("/hookSpecificOutput/permissionDecision")
            .and_then(|d| d.as_str()),
        Some("allow"),
    );
}

// ---------------------------------------------------------------
// Copilot format — flat output, double-encoded toolArgs, `bash` field
// ---------------------------------------------------------------

#[test]
fn copilot_hook_allows_safe_command_via_double_decode() {
    // `toolArgs` is a JSON-encoded *string*. Test asserts our parser
    // double-decodes correctly.
    let payload = r#"{
        "timestamp": 1704614600000,
        "cwd": "/path/to/project",
        "toolName": "bash",
        "toolArgs": "{\"command\":\"ls -la\",\"description\":\"list files\"}"
    }"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "copilot"], payload);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(
        v.get("permissionDecision").and_then(|s| s.as_str()),
        Some("allow"),
    );
    assert!(
        v.get("hookSpecificOutput").is_none(),
        "Copilot uses flat output — no wrapper",
    );
}

#[test]
fn copilot_hook_skips_non_bash_tools() {
    let payload = r#"{
        "timestamp": 1,
        "cwd": "/p",
        "toolName": "edit",
        "toolArgs": "{\"path\":\"x\"}"
    }"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "copilot"], payload);
    assert_eq!(code, 0);
    assert_eq!(stdout, "");
}

#[test]
fn copilot_hook_denies_unsafe_via_double_decode() {
    let payload = r#"{
        "timestamp": 1,
        "cwd": "/p",
        "toolName": "bash",
        "toolArgs": "{\"command\":\"rm -rf /etc\"}"
    }"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "copilot"], payload);
    assert_eq!(code, 0);
    assert_eq!(stdout, "");
}
