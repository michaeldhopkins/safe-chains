use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::{HookFormat, HookInput, HookResponse, InstallOutcome, ParseError, Target};
use crate::verdict::Verdict;

pub struct CodexTarget;

const HOOK_COMMAND: &str = "safe-chains hook codex";

/// The events `--setup` registers, both answered by `safe-chains hook codex`. `PreToolUse` vetoes
/// gated commands before they run; `PermissionRequest` approves safe reads when Codex would
/// otherwise prompt (a sandbox escalation, or an `untrusted` approval policy).
const EVENTS: [&str; 2] = ["PreToolUse", "PermissionRequest"];

impl Target for CodexTarget {
    fn name(&self) -> &'static str {
        "codex"
    }

    fn display_name(&self) -> &'static str {
        "Codex (OpenAI)"
    }

    #[cfg(test)]
    fn sample_envelope(&self, tool: &str, command: &str) -> Option<String> {
        Some(format!(r#"{{"tool_name":"{tool}","tool_input":{{"command":"{command}"}}}}"#))
    }

    fn detect_paths(&self, home: &Path) -> Vec<PathBuf> {
        vec![home.join(".codex")]
    }

    /// Adds whichever of `EVENTS` is missing, so a user who installed the `PreToolUse` hook
    /// before `PermissionRequest` support gets the second entry from a re-run of `--setup`.
    fn install(&self, home: &Path) -> Result<InstallOutcome, String> {
        let dir = home.join(".codex");
        if !dir.exists() {
            return Ok(InstallOutcome::Skipped { reason: format!("~/.codex not found at {} (Codex CLI not installed)", dir.display()) });
        }

        let path = dir.join("hooks.json");
        let mut settings = if path.exists() {
            let contents = std::fs::read_to_string(&path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
            serde_json::from_str(&contents).map_err(|e| format!("Could not parse {}: {e}", path.display()))?
        } else {
            Value::Object(Map::new())
        };

        let missing: Vec<&str> = EVENTS.into_iter().filter(|event| !has_safe_chains_hook(&settings, event)).collect();
        if missing.is_empty() {
            return Ok(InstallOutcome::AlreadyConfigured { path });
        }
        for event in missing {
            add_hook(&mut settings, event, HOOK_COMMAND).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        let output = serde_json::to_string_pretty(&settings).expect("serializing valid JSON");
        std::fs::write(&path, format!("{output}\n")).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
        Ok(InstallOutcome::Installed { path })
    }

    fn hook_format(&self) -> Option<&dyn HookFormat> {
        Some(&CodexHookFormat)
    }

    /// Codex names the event in `hook_event_name`, which it sets itself (the agent cannot reach
    /// it). Anything that is not a `PermissionRequest`, an envelope without the field included,
    /// keeps the `PreToolUse` answer this target has always given.
    fn hook_format_for(&self, stdin: &str) -> Option<&dyn HookFormat> {
        if event_name(stdin).as_deref() == Some("PermissionRequest") { Some(&CodexPermissionRequestFormat) } else { Some(&CodexHookFormat) }
    }

    fn hook_formats(&self) -> Vec<&dyn HookFormat> {
        vec![&CodexHookFormat, &CodexPermissionRequestFormat]
    }
}

#[derive(Deserialize)]
struct EventProbe {
    #[serde(default)]
    hook_event_name: Option<String>,
}

fn event_name(stdin: &str) -> Option<String> {
    serde_json::from_str::<EventProbe>(stdin).ok().and_then(|probe| probe.hook_event_name)
}

struct CodexHookFormat;

#[derive(Deserialize)]
struct ToolInput {
    command: String,
}

#[derive(Deserialize)]
struct CodexHookEnvelope {
    /// Optional so a harness that omits it still works; when present and naming another tool we
    /// abstain (see parse_input).
    #[serde(default)]
    tool_name: Option<String>,
    tool_input: ToolInput,
    #[serde(default)]
    cwd: Option<String>,
}

impl HookFormat for CodexHookFormat {
    fn parse_input(&self, stdin: &str) -> Result<HookInput, ParseError> {
        let envelope: CodexHookEnvelope = serde_json::from_str(stdin).map_err(|e| ParseError { message: e.to_string() })?;
        // Self-filter on the tool: the hook can be delivered for a non-shell call by a
        // hand-edited matcher, and deciding on one grants or vetoes a tool never analysed.
        if let Some(name) = &envelope.tool_name
            && name != "Bash"
        {
            return Err(ParseError { message: format!("not a shell tool: {name}") });
        }
        Ok(HookInput {
            command: envelope.tool_input.command,
            cwd: envelope.cwd,
            root: None, // codex sends cwd but no distinct project root (HARNESS-BEHAVIORS.md)
            // No scratchpad layout researched for this harness yet (see docs/design/agent-scratchpad.md).
            session_id: None,
        })
    }

    fn decision_pointer(&self) -> &'static str {
        "/hookSpecificOutput/permissionDecision" // mirrors Claude's nesting
    }

    fn render_response(&self, _verdict: Verdict) -> HookResponse {
        // SAFE command → emit nothing. A PreToolUse `permissionDecision:"allow"` is still rejected
        // as unsupported (v0.144.3 through v0.160.1), and Codex "continues on unsupported output"
        // anyway. Silence lets the safe command run through Codex's own flow; where that flow
        // would prompt, the PermissionRequest format below approves it.
        HookResponse { stdout: String::new(), exit_code: 0 }
    }

    // Codex has no human-review-on-silence (only sandbox-escape prompts) and no `ask`, but its
    // sandbox permits BROAD READS (`cat /etc/shadow` runs), so a gated command must be denied by
    // the hook. See docs/design/harness-capability-model.md. Verified against v0.144.3, 2026-07-13.
    fn gated_policy(&self) -> super::GatedPolicy {
        super::GatedPolicy::Deny
    }

    fn render_deny(&self, reason: &str) -> HookResponse {
        let body = json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": reason,
            }
        });
        HookResponse { stdout: serde_json::to_string(&body).unwrap_or_default(), exit_code: 0 }
    }
}

/// Codex's `PermissionRequest` event (v0.122.0 and later): it fires only when Codex is about to
/// ask for approval, and an `allow` answers that prompt. Anything short of a safe read emits
/// nothing, which leaves Codex's normal approval in place, so `gated_policy` keeps the `Defer`
/// default. Only `allow` is ever emitted: `updatedInput`, `updatedPermissions` and
/// `interrupt:true` make Codex fail the hook.
struct CodexPermissionRequestFormat;

/// A host-level network approval arrives as the command with a `network-access <host>`
/// description, and is classified like any other request: the classifier judges network commands
/// for Codex as it does for Claude Code.
#[derive(Deserialize)]
struct PermissionRequestToolInput {
    command: String,
}

#[derive(Deserialize)]
struct PermissionRequestEnvelope {
    hook_event_name: String,
    tool_name: String,
    tool_input: PermissionRequestToolInput,
    cwd: String,
}

impl HookFormat for CodexPermissionRequestFormat {
    fn parse_input(&self, stdin: &str) -> Result<HookInput, ParseError> {
        let envelope: PermissionRequestEnvelope = serde_json::from_str(stdin).map_err(|e| ParseError { message: e.to_string() })?;
        if envelope.hook_event_name != "PermissionRequest" {
            return Err(ParseError { message: format!("not a PermissionRequest: {}", envelope.hook_event_name) });
        }
        // Required here, not optional: this event also carries `apply_patch`, whose
        // `tool_input.command` is a patch, and MCP calls, `write_stdin` and `request_permissions`.
        if envelope.tool_name != "Bash" {
            return Err(ParseError { message: format!("not a shell tool: {}", envelope.tool_name) });
        }
        // The cwd is the workspace root here (see `cwd_is_the_commands`), so a root that is the
        // whole filesystem, or one that is not a path at all, would put every write inside it.
        if !bounds_a_workspace(&envelope.cwd) {
            return Err(ParseError { message: format!("no usable workspace in cwd: {}", envelope.cwd) });
        }
        Ok(HookInput { command: envelope.tool_input.command, cwd: Some(envelope.cwd), root: None, session_id: None })
    }

    /// Codex reports the turn's cwd, but `exec_command` runs in its own `workdir`, which this
    /// payload leaves out. An approval here runs outside the sandbox, so a relative path must not be
    /// trusted to land where the reported cwd says: the model could set `workdir` to `~` and have
    /// `echo x >> .zshrc` approved as a worktree write.
    fn cwd_is_the_commands(&self) -> bool {
        false
    }

    fn decision_pointer(&self) -> &'static str {
        "/hookSpecificOutput/decision/behavior"
    }

    fn render_response(&self, verdict: Verdict) -> HookResponse {
        if !verdict.is_allowed() {
            return HookResponse { stdout: String::new(), exit_code: 0 };
        }
        let body = json!({
            "hookSpecificOutput": {
                "hookEventName": "PermissionRequest",
                "decision": { "behavior": "allow" },
            }
        });
        HookResponse { stdout: serde_json::to_string(&body).unwrap_or_default(), exit_code: 0 }
    }
}

/// An absolute path that still names a directory below `/` once `.` and `..` are folded away, so
/// `/.`, `/Users/..` and `//` are refused like `/` itself.
fn bounds_a_workspace(cwd: &str) -> bool {
    if !cwd.starts_with('/') {
        return false;
    }
    let mut depth = 0usize;
    for part in cwd.split('/') {
        match part {
            "" | "." => {}
            ".." => depth = depth.saturating_sub(1),
            _ => depth += 1,
        }
    }
    depth > 0
}

fn hook_entry(binary: &str) -> Value {
    json!({
        "matcher": "Bash",
        "hooks": [{
            "type": "command",
            "command": binary,
        }]
    })
}

/// An entry counts only when it runs `safe-chains hook codex` on shell calls: one under an
/// `apply_patch` matcher, or running another target's hook, leaves `Bash` uncovered on that event.
fn has_safe_chains_hook(settings: &Value, event: &str) -> bool {
    settings
        .get("hooks")
        .and_then(|h| h.get(event))
        .and_then(|arr| arr.as_array())
        .is_some_and(|entries| {
            entries.iter().any(|entry| {
                matcher_covers_bash(entry.get("matcher"))
                    && entry.get("hooks").and_then(|h| h.as_array()).is_some_and(|hooks| {
                        hooks
                            .iter()
                            .any(|hook| hook.get("command").and_then(|c| c.as_str()).is_some_and(|cmd| cmd.contains(HOOK_COMMAND)))
                    })
            })
        })
}

fn matcher_covers_bash(matcher: Option<&Value>) -> bool {
    match matcher {
        None | Some(Value::Null) => true,
        Some(Value::String(m)) => m.is_empty() || m == "*" || m.contains("Bash"),
        Some(_) => false,
    }
}

/// Codex nests lifecycle events under a top-level `hooks` object (NOT Claude's flat `PreToolUse`
/// key) — a flat key makes Codex reject the whole file. See developers.openai.com/codex/hooks.
///
/// This used to REPLACE a wrong-typed `hooks` or `PreToolUse` value with an empty one, destroying
/// whatever the user had there without saying so. The shared helper refuses instead: an unreadable
/// value is usually a hand-edit or a schema we don't know, and rewriting config we did not
/// understand is not ours to do.
fn add_hook(settings: &mut Value, event: &str, binary: &str) -> Result<(), String> {
    super::append_hook_entry(settings, "hooks", event, hook_entry(binary))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verdict::SafetyLevel;

    fn target() -> CodexTarget {
        CodexTarget
    }

    fn installed(home: &Path) -> Value {
        let contents = std::fs::read_to_string(home.join(".codex/hooks.json")).unwrap();
        serde_json::from_str(&contents).unwrap()
    }

    fn permission_request(tool: &str, command: &str) -> String {
        json!({
            "session_id": "s",
            "turn_id": "t",
            "transcript_path": null,
            "cwd": "/w",
            "hook_event_name": "PermissionRequest",
            "model": "m",
            "permission_mode": "default",
            "tool_name": tool,
            "tool_input": { "command": command, "description": "needs to write outside the sandbox" },
        })
        .to_string()
    }

    #[test]
    fn install_no_codex_dir_skips() {
        let dir = tempfile::tempdir().unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::Skipped { .. }));
    }

    #[test]
    fn install_creates_hooks_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".codex")).unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::Installed { .. }));
        let settings = installed(dir.path());
        for event in EVENTS {
            assert!(has_safe_chains_hook(&settings, event), "{event} not installed");
        }
        // Codex nests events under a top-level `hooks` object; a flat top-level `PreToolUse`
        // (Claude's shape) makes Codex reject the entire file (`unknown field PreToolUse`).
        assert!(settings.get("PreToolUse").is_none(), "must not use Claude's flat PreToolUse key");
        assert!(settings.get("PermissionRequest").is_none(), "must not use a flat PermissionRequest key");
        assert_eq!(settings.as_object().map(|o| o.len()), Some(1), "only the `hooks` key: {settings}");
    }

    #[test]
    fn install_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".codex")).unwrap();
        target().install(dir.path()).unwrap();
        let first = installed(dir.path());
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::AlreadyConfigured { .. }));
        assert_eq!(installed(dir.path()), first, "a second run must not add duplicate entries");
    }

    #[test]
    fn install_adds_permission_request_beside_an_older_pre_tool_use_install() {
        let dir = tempfile::tempdir().unwrap();
        let codex_dir = dir.path().join(".codex");
        std::fs::create_dir(&codex_dir).unwrap();
        let older = json!({"hooks": {"PreToolUse": [hook_entry(HOOK_COMMAND)]}});
        std::fs::write(codex_dir.join("hooks.json"), older.to_string()).unwrap();

        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::Installed { .. }));
        let settings = installed(dir.path());
        assert!(has_safe_chains_hook(&settings, "PermissionRequest"));
        assert_eq!(settings["hooks"]["PreToolUse"], older["hooks"]["PreToolUse"], "the existing entry must be left exactly as it was");
    }

    #[test]
    fn install_uses_subcommand_invocation() {
        // The binary entry must be `safe-chains hook codex`, not just
        // `safe-chains`, so the runtime knows which envelope to emit.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".codex")).unwrap();
        target().install(dir.path()).unwrap();
        let settings = installed(dir.path());
        for event in EVENTS {
            assert_eq!(settings["hooks"][event][0]["hooks"][0]["command"], HOOK_COMMAND);
            assert_eq!(settings["hooks"][event][0]["matcher"], "Bash");
        }
    }

    #[test]
    fn install_preserves_existing_hooks() {
        let dir = tempfile::tempdir().unwrap();
        let codex_dir = dir.path().join(".codex");
        std::fs::create_dir(&codex_dir).unwrap();
        std::fs::write(
            codex_dir.join("hooks.json"),
            r#"{"hooks": {"PostToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "log-it"}]}], "PermissionRequest": [{"matcher": "apply_patch", "hooks": [{"type": "command", "command": "mine"}]}]}}"#,
        )
        .unwrap();
        target().install(dir.path()).unwrap();
        let settings = installed(dir.path());
        assert!(has_safe_chains_hook(&settings, "PreToolUse"));
        assert!(has_safe_chains_hook(&settings, "PermissionRequest"));
        assert!(settings["hooks"].get("PostToolUse").is_some(), "existing PostToolUse must be preserved");
        assert_eq!(settings["hooks"]["PermissionRequest"][0]["hooks"][0]["command"], "mine", "the user's own entry comes first, untouched");
    }

    #[test]
    fn parse_input_extracts_command() {
        let stdin = r#"{"tool_name": "Bash", "tool_input": {"command": "ls -la"}}"#;
        let parsed = CodexHookFormat.parse_input(stdin).unwrap();
        assert_eq!(parsed.command, "ls -la");
    }

    #[test]
    fn parse_input_with_optional_cwd() {
        let stdin = r#"{"tool_input": {"command": "pwd"}, "cwd": "/Users/me"}"#;
        let parsed = CodexHookFormat.parse_input(stdin).unwrap();
        assert_eq!(parsed.cwd.as_deref(), Some("/Users/me"));
    }

    #[test]
    fn parse_input_rejects_garbage() {
        assert!(CodexHookFormat.parse_input("not json").is_err());
        assert!(CodexHookFormat.parse_input("{}").is_err());
    }

    #[test]
    fn render_response_safe_emits_empty_body() {
        // A PreToolUse `permissionDecision:"allow"` is unsupported on Codex. A safe command emits
        // nothing (Codex continues → runs it); it must NOT emit an allow envelope.
        let r = CodexHookFormat.render_response(Verdict::Allowed(SafetyLevel::Inert));
        assert_eq!(r.stdout, "");
        let r = CodexHookFormat.render_response(Verdict::Denied);
        assert_eq!(r.stdout, "");
    }

    #[test]
    fn gated_command_is_denied_with_the_supported_shape() {
        // Codex handles a gated command by DENYING (no interactive approval, sandbox permits reads).
        assert_eq!(CodexHookFormat.gated_policy(), super::super::GatedPolicy::Deny);
        let r = CodexHookFormat.render_deny("blocked: not on the allowlist");
        let v: Value = serde_json::from_str(&r.stdout).unwrap();
        assert_eq!(v.pointer("/hookSpecificOutput/permissionDecision").and_then(|d| d.as_str()), Some("deny"));
        assert_eq!(v.pointer("/hookSpecificOutput/hookEventName").and_then(|d| d.as_str()), Some("PreToolUse"));
        assert_eq!(
            v.pointer("/hookSpecificOutput/permissionDecisionReason").and_then(|d| d.as_str()),
            Some("blocked: not on the allowlist"),
        );
        assert_eq!(r.exit_code, 0);
    }

    #[test]
    fn render_context_defaults_to_abstain() {
        // Codex's hook schema isn't verified for context injection, so it keeps
        // the safe default: emit nothing, leaving the normal flow untouched.
        let r = CodexHookFormat.render_context("anything");
        assert_eq!(r.stdout, "");
        assert_eq!(r.exit_code, 0);
    }

    #[test]
    fn routes_only_a_permission_request_to_the_permission_format() {
        let t = target();
        let pointer = |stdin: &str| t.hook_format_for(stdin).map(|f| f.decision_pointer());
        let permission = CodexPermissionRequestFormat.decision_pointer();
        let pre_tool_use = CodexHookFormat.decision_pointer();
        assert_eq!(pointer(&permission_request("Bash", "ls")), Some(permission));
        for other in [
            r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"ls"}}"#,
            r#"{"tool_name":"Bash","tool_input":{"command":"ls"}}"#,
            r#"{"hook_event_name":"permissionrequest","tool_name":"Bash","tool_input":{"command":"ls"}}"#,
            r#"{"hook_event_name":7,"tool_name":"Bash","tool_input":{"command":"ls"}}"#,
            "not json",
            "",
        ] {
            assert_eq!(pointer(other), Some(pre_tool_use), "{other} must keep the PreToolUse answer");
        }
    }

    #[test]
    fn permission_request_passes_shell_syntax_and_network_approvals_to_the_classifier() {
        for command in [
            "echo $(cat x)", "echo `cat x`", "diff <(ls) b", "curl https://x/$HOME", "read l < f", "cat f | xargs curl",
            "git status && ls", "ls; ls", "ls\nls", "cat *", "cat ~/x", "ls {a,b}", "git status", "bash -c 'ls -la'",
        ] {
            let parsed = CodexPermissionRequestFormat.parse_input(&permission_request("Bash", command)).unwrap();
            assert_eq!(parsed.command, command);
        }
        let mut network: Value = serde_json::from_str(&permission_request("Bash", "curl https://example.com")).unwrap();
        for description in [json!("network-access example.com"), json!(null), json!(7)] {
            network["tool_input"]["description"] = description.clone();
            let parsed = CodexPermissionRequestFormat.parse_input(&network.to_string());
            assert_eq!(parsed.map(|p| p.command).ok().as_deref(), Some("curl https://example.com"), "{description}");
        }
    }

    #[test]
    fn permission_request_parses_a_bash_envelope() {
        let parsed = CodexPermissionRequestFormat.parse_input(&permission_request("Bash", "git status")).unwrap();
        assert_eq!(parsed.command, "git status");
        assert_eq!(parsed.cwd.as_deref(), Some("/w"));
    }

    #[test]
    fn permission_request_abstains_on_every_other_tool() {
        for tool in ["apply_patch", "write_stdin", "request_permissions", "mcp__server__tool", "Edit", "bash"] {
            assert!(
                CodexPermissionRequestFormat.parse_input(&permission_request(tool, "ls")).is_err(),
                "{tool} must not be classified as a shell command"
            );
        }
        let no_tool = r#"{"hook_event_name":"PermissionRequest","tool_input":{"command":"ls"},"cwd":"/w"}"#;
        assert!(CodexPermissionRequestFormat.parse_input(no_tool).is_err(), "tool_name is required on this event");
        let pre_tool_use = r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"ls"},"cwd":"/w"}"#;
        assert!(CodexPermissionRequestFormat.parse_input(pre_tool_use).is_err());
        let argv = r#"{"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":["ls"]},"cwd":"/w"}"#;
        assert!(CodexPermissionRequestFormat.parse_input(argv).is_err(), "only a command string is classified");
    }

    #[test]
    fn permission_request_allows_exactly_the_documented_shape() {
        for level in [SafetyLevel::Inert, SafetyLevel::SafeRead, SafetyLevel::SafeWrite] {
            let r = CodexPermissionRequestFormat.render_response(Verdict::Allowed(level));
            assert_eq!(r.exit_code, 0);
            let v: Value = serde_json::from_str(&r.stdout).unwrap();
            assert_eq!(v, json!({"hookSpecificOutput": {"hookEventName": "PermissionRequest", "decision": {"behavior": "allow"}}}));
        }
    }

    #[test]
    fn permission_request_emits_nothing_short_of_a_safe_verdict() {
        assert_eq!(CodexPermissionRequestFormat.gated_policy(), super::super::GatedPolicy::Defer);
        for r in [
            CodexPermissionRequestFormat.render_response(Verdict::Denied),
            CodexPermissionRequestFormat.render_deny("x"),
            CodexPermissionRequestFormat.render_ask("x"),
            CodexPermissionRequestFormat.render_context("x"),
        ] {
            assert_eq!(r.stdout, "");
            assert_eq!(r.exit_code, 0);
        }
    }

    #[test]
    fn permission_request_needs_a_cwd_that_bounds_a_workspace() {
        let with_cwd = |cwd: Value| {
            json!({"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"ls"},"cwd":cwd}).to_string()
        };
        for unusable in [json!("/"), json!("//"), json!(""), json!("relative/dir"), json!(null), json!(7)] {
            assert!(CodexPermissionRequestFormat.parse_input(&with_cwd(unusable.clone())).is_err(), "cwd {unusable} must abstain");
        }
        let missing = r#"{"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"ls"}}"#;
        assert!(CodexPermissionRequestFormat.parse_input(missing).is_err());
        assert!(CodexPermissionRequestFormat.parse_input(&with_cwd(json!("/w"))).is_ok());
    }

    #[test]
    fn only_the_permission_request_cwd_is_not_the_commands() {
        assert!(CodexHookFormat.cwd_is_the_commands());
        assert!(!CodexPermissionRequestFormat.cwd_is_the_commands());
        let input = CodexPermissionRequestFormat.parse_input(&permission_request("Bash", "ls")).unwrap();
        let (cwd, root) = super::super::evaluation_dirs(&CodexPermissionRequestFormat, &input);
        assert_eq!(cwd.as_deref(), Some(super::super::UNKNOWN_WORKDIR));
        assert_eq!(root.as_deref(), Some("/w"), "the reported cwd stays the workspace");
    }

    #[test]
    fn install_ignores_an_entry_that_does_not_run_this_hook_on_shell_calls() {
        let dir = tempfile::tempdir().unwrap();
        let codex_dir = dir.path().join(".codex");
        std::fs::create_dir(&codex_dir).unwrap();
        let elsewhere = json!({"hooks": {
            "PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "safe-chains hook claude"}]}],
            "PermissionRequest": [{"matcher": "apply_patch", "hooks": [{"type": "command", "command": HOOK_COMMAND}]}],
        }});
        std::fs::write(codex_dir.join("hooks.json"), elsewhere.to_string()).unwrap();

        assert!(matches!(target().install(dir.path()).unwrap(), InstallOutcome::Installed { .. }));
        let settings = installed(dir.path());
        for event in EVENTS {
            let entries = settings["hooks"][event].as_array().unwrap();
            assert_eq!(entries.len(), 2, "{event}: ours is added beside the other entry");
            assert_eq!(entries[1], hook_entry(HOOK_COMMAND));
        }
    }

    #[test]
    fn matcher_coverage_of_shell_calls() {
        for covers in [None, Some(json!(null)), Some(json!("")), Some(json!("*")), Some(json!("Bash")), Some(json!("Bash|apply_patch"))] {
            assert!(matcher_covers_bash(covers.as_ref()), "{covers:?}");
        }
        for misses in [json!("apply_patch"), json!("mcp__x__y"), json!(["Bash"]), json!(1)] {
            assert!(!matcher_covers_bash(Some(&misses)), "{misses}");
        }
    }

    #[test]
    fn install_refuses_a_wrong_typed_permission_request_slot_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let codex_dir = dir.path().join(".codex");
        std::fs::create_dir(&codex_dir).unwrap();
        let path = codex_dir.join("hooks.json");
        std::fs::write(&path, r#"{"hooks": {"PermissionRequest": "corrupted"}}"#).unwrap();
        let before = std::fs::read(&path).unwrap();

        let Err(err) = target().install(dir.path()) else { panic!("a wrong-typed slot must be refused") };
        assert!(err.contains("hooks.PermissionRequest"), "the error names the slot: {err}");
        assert_eq!(std::fs::read(&path).unwrap(), before, "PreToolUse must not be written alone either");
    }

    #[test]
    fn only_a_read_is_granted_where_the_workdir_is_unknown() {
        use super::super::respond;
        let grants = |format: &dyn HookFormat, level| respond(format, "x", Verdict::Allowed(level)).is_some_and(|r| !r.stdout.is_empty());
        assert!(grants(&CodexPermissionRequestFormat, SafetyLevel::Inert));
        assert!(grants(&CodexPermissionRequestFormat, SafetyLevel::SafeRead));
        assert!(!grants(&CodexPermissionRequestFormat, SafetyLevel::SafeWrite), "a write could land anywhere");
        assert!(
            respond(&CodexHookFormat, "x", Verdict::Allowed(SafetyLevel::SafeWrite)).is_some(),
            "where the cwd is the command's, the ceiling does not apply"
        );
    }

    #[test]
    fn a_cwd_that_folds_to_the_filesystem_root_bounds_nothing() {
        for root in ["/", "//", "/.", "/./", "/Users/..", "/tmp/..", "/a/b/../..", "/../.."] {
            assert!(!bounds_a_workspace(root), "{root}");
        }
        for dir in ["/w", "/Users/me/proj", "/a/b/..", "/./w", "/../w"] {
            assert!(bounds_a_workspace(dir), "{dir}");
        }
    }

    proptest::proptest! {
        /// Folding `.` and `..` is the whole rule: a directory stays a workspace however many `.`
        /// segments it carries, and climbing back out of every segment it named reaches `/`.
        #[test]
        fn bounds_a_workspace_follows_the_folded_depth(
            names in proptest::collection::vec("[a-zA-Z0-9_-]{1,8}", 1..6),
            dots in 0usize..4,
        ) {
            let dir = format!("/{}", names.join("/"));
            proptest::prop_assert!(bounds_a_workspace(&dir));
            let dotted = dir.clone() + &"/.".repeat(dots);
            let climbed = dir.clone() + &"/..".repeat(names.len() + dots);
            let relative = names.join("/");
            proptest::prop_assert!(bounds_a_workspace(&dotted), "{}", dotted);
            proptest::prop_assert!(!bounds_a_workspace(&climbed), "{}", climbed);
            proptest::prop_assert!(!bounds_a_workspace(&relative), "a relative path is never a workspace: {}", relative);
        }

        /// Whatever the command says and whatever the model wrote as its justification, a Bash
        /// request with a usable cwd reaches the classifier unchanged: the event adds no rule of its
        /// own about the command's text, only the read ceiling on the verdict.
        #[test]
        fn every_bash_request_reaches_the_classifier_verbatim(command in "\\PC*", description in proptest::option::of("\\PC*")) {
            let mut payload: Value = serde_json::from_str(&permission_request("Bash", &command)).unwrap();
            payload["tool_input"]["description"] = description.map_or(Value::Null, Value::String);
            let parsed = CodexPermissionRequestFormat.parse_input(&payload.to_string());
            proptest::prop_assert_eq!(parsed.map(|p| p.command).ok(), Some(command));
        }
    }
}
