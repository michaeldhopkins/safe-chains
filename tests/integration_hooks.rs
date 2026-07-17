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
    match child.stdin.as_mut().unwrap().write_all(stdin_payload.as_bytes()) {
        Ok(()) => {}
        // Short-circuit error paths (e.g. unknown subcommand) exit before
        // reading stdin, which races with our write and surfaces as
        // BrokenPipe. The test is asserting on stdout/exit code, not on
        // a successful stdin handshake — tolerate the race.
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
        Err(e) => panic!("write to safe-chains stdin failed: {e}"),
    }
    let out = child.wait_with_output().expect("wait");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// Run the claude hook with a temp `$HOME` carrying `level = "<level>"` in the user config, and
/// `cwd` set to that home so a relative `./f` classifies as a worktree path. Returns (stdout, exit).
fn hook_at_level(level: &str, command: &str) -> (String, i32) {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let home = std::env::temp_dir()
        .join(format!("sc-lvl-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(home.join(".config")).unwrap();
    std::fs::write(home.join(".config/safe-chains.toml"), format!("level = \"{level}\"\n")).unwrap();
    let payload =
        format!(r#"{{"tool_input": {{"command": "{command}"}}, "cwd": "{}"}}"#, home.display());
    let mut child = Command::new(binary())
        .args(["hook", "claude"])
        .env("HOME", &home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let _ = child.stdin.as_mut().unwrap().write_all(payload.as_bytes());
    let out = child.wait_with_output().expect("wait");
    let _ = std::fs::remove_dir_all(&home);
    (String::from_utf8_lossy(&out.stdout).into_owned(), out.status.code().unwrap_or(-1))
}

fn decides_allow(stdout: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(stdout.trim())
        .ok()
        .and_then(|v| {
            v.pointer("/hookSpecificOutput/permissionDecision")
                .and_then(|d| d.as_str())
                .map(|s| s == "allow")
        })
        .unwrap_or(false)
}

#[test]
fn configured_level_network_admin_raises_the_ceiling() {
    // git push gates at the default band; network-admin (from the write-protected config) approves it.
    let (stdout, code) = hook_at_level("network-admin", "git push origin main");
    assert_eq!(code, 0);
    assert!(decides_allow(&stdout), "network-admin approves git push: {stdout}");
}

#[test]
fn configured_level_reader_lowers_the_ceiling_and_gates_writes() {
    // A read approves; a WRITE gates — even though the built-in coverage fallback would otherwise
    // re-admit it. This is the lowering fix: the fallback is held under the same `reader` ceiling.
    assert!(decides_allow(&hook_at_level("reader", "cat ./notes.txt").0), "reader approves a read");
    let (stdout, code) = hook_at_level("reader", "echo hi > ./notes.txt");
    assert_eq!(code, 0);
    assert!(!decides_allow(&stdout), "reader gates a worktree write: {stdout}");
}

#[test]
fn configured_level_editor_gates_destroys_but_not_writes() {
    // editor ≠ developer: create/mutate a worktree file approves, but a DESTROY gates — a distinction
    // the 3-band projection flattens, honored through the hook via the engine level in BOTH the primary
    // verdict and the coverage fallback. (developer, the default, allows the destroy — see the write/rm
    // asymmetry against the reader/default tests above.)
    assert!(decides_allow(&hook_at_level("editor", "echo hi > ./notes.txt").0), "editor writes a worktree file");
    let (stdout, code) = hook_at_level("editor", "rm ./notes.txt");
    assert_eq!(code, 0);
    assert!(!decides_allow(&stdout), "editor gates a worktree destroy: {stdout}");
    // the same destroy approves at the default developer band.
    assert!(decides_allow(&hook_at_level("developer", "rm ./notes.txt").0), "developer allows the destroy");
}

#[test]
fn configured_level_unknown_name_falls_back_to_the_default_band() {
    // A garbled level must not open OR over-tighten — it fails safe to developer: a worktree write
    // approves (default), git push still gates.
    assert!(decides_allow(&hook_at_level("banana", "echo hi > ./notes.txt").0), "unknown → default allows a write");
    assert!(!decides_allow(&hook_at_level("banana", "git push origin main").0), "unknown → default still gates git push");
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
    // an unsafe command with no path operand → empty stdout + exit 0 means "no opinion",
    // Claude falls back to its own rules. That's the contract, not an error. (A command that
    // reaches OUTSIDE the workspace instead gets a nudge — see the test below.)
    let payload = r#"{"tool_input": {"command": "git push"}}"#;
    let (stdout, _stderr, code) = run_hook(&[], payload);
    assert_eq!(code, 0);
    assert_eq!(stdout, "");
}

#[test]
fn claude_nudges_when_a_command_reaches_outside_the_workspace() {
    let payload = r#"{"tool_input": {"command": "cat /etc/hosts"}, "cwd": "/Users/me/proj"}"#;
    let (stdout, _stderr, code) = run_hook(&[], payload);
    assert_eq!(code, 0);
    // reaches outside → an additionalContext nudge, but NO permission decision (Claude still decides)
    assert!(stdout.contains("additionalContext"), "expected a nudge: {stdout}");
    assert!(!stdout.contains("permissionDecision"), "the nudge must not decide: {stdout}");
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
fn codex_hook_safe_command_emits_nothing() {
    // Codex has no `grant` (`allow` is unsupported on v0.144.3), so a SAFE command emits nothing
    // and Codex runs it through its own flow. See docs/design/harness-capability-model.md.
    let payload = r#"{"tool_name": "Bash", "tool_input": {"command": "ls -la"}}"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "codex"], payload);
    assert_eq!(code, 0);
    assert_eq!(stdout, "");
}

#[test]
fn codex_hook_denies_gated_command() {
    // Codex has no interactive approval and its sandbox permits broad reads, so a GATED command is
    // vetoed by the hook (with a `deny` decision), not left silent.
    let payload = r#"{"tool_name": "Bash", "tool_input": {"command": "rm -rf /etc"}}"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "codex"], payload);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(
        v.pointer("/hookSpecificOutput/permissionDecision").and_then(|d| d.as_str()),
        Some("deny"),
    );
    assert_eq!(
        v.pointer("/hookSpecificOutput/hookEventName").and_then(|d| d.as_str()),
        Some("PreToolUse"),
    );
}

#[test]
fn codex_hook_handles_optional_cwd() {
    // A safe command with cwd present: parses fine and emits nothing (safe → run).
    let payload = r#"{"tool_input": {"command": "git status"}, "cwd": "/Users/me/project"}"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "codex"], payload);
    assert_eq!(code, 0);
    assert_eq!(stdout, "");
}

#[test]
fn codex_hook_invalid_json_exits_zero_silently() {
    let (stdout, _stderr, code) = run_hook(&["hook", "codex"], "{");
    assert_eq!(code, 0);
    assert_eq!(stdout, "");
}

#[test]
fn codex_hook_gated_overreach_reason_names_the_path() {
    // A gated command that reaches OUTSIDE the workspace: the deny reason must name the path and say
    // it's outside the working directory (not the generic "not on the allowlist" text). A Deny
    // harness exits the gated match early, so this is the only place that specific reason reaches it.
    let payload = r#"{"tool_input": {"command": "cat /etc/hosts"}, "cwd": "/tmp/proj"}"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "codex"], payload);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let reason = v
        .pointer("/hookSpecificOutput/permissionDecisionReason")
        .and_then(|d| d.as_str())
        .unwrap_or_default();
    assert!(reason.contains("/etc/hosts"), "reason should name the path: {reason}");
    assert!(reason.contains("outside the working directory"), "reason: {reason}");
}

#[test]
fn codex_hook_gated_non_overreach_uses_generic_reason() {
    // Gated but NOT an overreach (an unknown command inside the workspace): the deny reason falls
    // back to the generic "not on the allowlist" text, not the path-specific one.
    let payload = r#"{"tool_input": {"command": "frobnicate --wizz"}, "cwd": "/tmp/proj"}"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "codex"], payload);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let reason = v
        .pointer("/hookSpecificOutput/permissionDecisionReason")
        .and_then(|d| d.as_str())
        .unwrap_or_default();
    assert!(reason.contains("not on the allowlist"), "reason: {reason}");
    assert!(!reason.contains("outside the working directory"), "reason: {reason}");
}

// --- Antigravity (`agy`) hook subcommand ---

#[test]
fn antigravity_hook_safe_command_allows() {
    let payload = r#"{"toolCall":{"name":"run_command","args":{"CommandLine":"git status"}},"workspacePaths":["/tmp/proj"]}"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "antigravity"], payload);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v.get("decision").and_then(|d| d.as_str()), Some("allow"));
}

#[test]
fn antigravity_hook_gated_command_force_asks() {
    // Gated, non-overreach: escalate with `force_ask` (bypasses agy's Always-Allow cache) and the
    // generic "stops flagging it" reason.
    let payload = r#"{"toolCall":{"name":"run_command","args":{"CommandLine":"frobnicate --wizz"}},"workspacePaths":["/tmp/proj"]}"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "antigravity"], payload);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v.get("decision").and_then(|d| d.as_str()), Some("force_ask"));
    let reason = v.get("reason").and_then(|d| d.as_str()).unwrap_or_default();
    assert!(reason.contains("stops flagging it"), "reason: {reason}");
}

#[test]
fn antigravity_hook_gated_overreach_force_asks_with_path() {
    let payload = r#"{"toolCall":{"name":"run_command","args":{"CommandLine":"cat /etc/hosts"}},"workspacePaths":["/tmp/proj"]}"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "antigravity"], payload);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v.get("decision").and_then(|d| d.as_str()), Some("force_ask"));
    let reason = v.get("reason").and_then(|d| d.as_str()).unwrap_or_default();
    assert!(reason.contains("/etc/hosts"), "reason should name the path: {reason}");
    assert!(reason.contains("outside the working directory"), "reason: {reason}");
}

#[test]
fn antigravity_hook_credential_reach_names_credential_store() {
    let payload = r#"{"toolCall":{"name":"run_command","args":{"CommandLine":"cat ~/.ssh/id_rsa"}},"workspacePaths":["/tmp/proj"]}"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "antigravity"], payload);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v.get("decision").and_then(|d| d.as_str()), Some("force_ask"));
    let reason = v.get("reason").and_then(|d| d.as_str()).unwrap_or_default();
    assert!(reason.contains("credential store"), "reason: {reason}");
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
    // cursor-agent ignores hook `allow` but HONORS `deny` (verified live), so Cursor is a Deny
    // harness: a gated command is VETOED with `permission:"deny"` + our reason, not abstained.
    let payload = r#"{"command": "rm -rf /etc", "cwd": "/x"}"#;
    let (stdout, _stderr, code) = run_hook(&["hook", "cursor"], payload);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v.get("permission").and_then(|s| s.as_str()), Some("deny"));
    assert!(v.get("user_message").and_then(|s| s.as_str()).is_some_and(|m| m.contains("safe-chains")));
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

/// Cross-target hook contract: for EVERY target with a runtime hook, the abstain path must be
/// fail-safe and the nudge/context path must never carry a permission decision. safe-chains never
/// denies — it emits empty output and lets the harness prompt — so any target that (a) emits an
/// approval on `Denied`, or (b) leaks a decision field through `render_context` (which would
/// override the user's own allowlist), is a silent fail-open. Only Claude asserted these before;
/// this pins them for all targets so a new one can't regress uncaught (the qwen/droid
/// `render_context` overrides had no such test).
#[test]
fn every_target_hook_contract_is_fail_safe() {
    use safe_chains::Verdict;
    let mut failures = Vec::new();
    for target in safe_chains::targets::registry() {
        let Some(fmt) = target.hook_format() else {
            continue;
        };
        let name = target.name();
        let denied = fmt.render_response(Verdict::Denied);
        if !denied.stdout.trim().is_empty() {
            failures.push(format!("{name}: render_response(Denied) emitted `{}`", denied.stdout));
        }
        let ctx = fmt.render_context("reaches /etc/x outside your workspace");
        for key in ["\"permissionDecision\"", "\"decision\"", "\"permission\""] {
            if ctx.stdout.contains(key) {
                failures.push(format!("{name}: render_context leaked {key} -> `{}`", ctx.stdout));
            }
        }
        if fmt.parse_input("this is not json {[").is_ok() {
            failures.push(format!("{name}: parse_input accepted garbage as a command"));
        }
        // Capability ↔ emission consistency: a target's gated policy must match what it emits, and
        // the unused render_* paths must stay empty (a stray call can't fail open).
        use safe_chains::targets::GatedPolicy;
        match fmt.gated_policy() {
            GatedPolicy::Deny => {
                if !fmt.render_deny("blocked").stdout.contains("\"deny\"") {
                    failures.push(format!("{name}: Deny policy but render_deny has no deny decision"));
                }
                if !fmt.render_ask("x").stdout.trim().is_empty() {
                    failures.push(format!("{name}: Deny policy but render_ask is non-empty"));
                }
            }
            GatedPolicy::Ask => {
                // An Ask target escalates to a human prompt: `ask` or `force_ask` (agy uses the
                // latter to bypass its Always-Allow cache — see targets/agy.rs).
                let ask = fmt.render_ask("confirm").stdout;
                if !ask.contains("\"ask\"") && !ask.contains("\"force_ask\"") {
                    failures.push(format!("{name}: Ask policy but render_ask has no ask decision"));
                }
                if !fmt.render_deny("x").stdout.trim().is_empty() {
                    failures.push(format!("{name}: Ask policy but render_deny is non-empty"));
                }
            }
            GatedPolicy::Defer => {
                if !fmt.render_deny("x").stdout.trim().is_empty() || !fmt.render_ask("x").stdout.trim().is_empty() {
                    failures.push(format!("{name}: Defer policy but a render_deny/ask emitted output"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "target hook contract violations:\n{}", failures.join("\n"));
}
