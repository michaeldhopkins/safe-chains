use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::{HookFormat, HookInput, HookResponse, InstallOutcome, ParseError, Target, allow_reason};
use crate::verdict::Verdict;

pub struct ClaudeTarget;

impl Target for ClaudeTarget {
    fn name(&self) -> &'static str {
        "claude"
    }

    fn display_name(&self) -> &'static str {
        "Claude Code"
    }

    #[cfg(test)]
    fn sample_envelope(&self, tool: &str, command: &str) -> Option<String> {
        Some(format!(r#"{{"tool_name":"{tool}","tool_input":{{"command":"{command}"}}}}"#))
    }

    fn detect_paths(&self, home: &Path) -> Vec<PathBuf> {
        vec![home.join(".claude")]
    }

    fn install(&self, home: &Path) -> Result<InstallOutcome, String> {
        let dir = home.join(".claude");
        if !dir.exists() {
            return Ok(InstallOutcome::Skipped {
                reason: format!(
                    "~/.claude not found at {} (Claude Code not installed)",
                    dir.display()
                ),
            });
        }

        let path = dir.join("settings.json");
        let binary = "safe-chains";

        if path.exists() {
            let contents = std::fs::read_to_string(&path)
                .map_err(|e| format!("Could not read {}: {e}", path.display()))?;
            let mut settings: Value = serde_json::from_str(&contents)
                .map_err(|e| format!("Could not parse {}: {e}", path.display()))?;

            if has_safe_chains_hook(&settings) {
                return Ok(InstallOutcome::AlreadyConfigured { path });
            }

            add_hook(&mut settings, binary)?;
            let output = serde_json::to_string_pretty(&settings).expect("serializing valid JSON");
            std::fs::write(&path, format!("{output}\n"))
                .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
            Ok(InstallOutcome::Installed { path })
        } else {
            let mut settings = Value::Object(Map::new());
            add_hook(&mut settings, binary)?;
            let output = serde_json::to_string_pretty(&settings).expect("serializing valid JSON");
            std::fs::write(&path, format!("{output}\n"))
                .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
            Ok(InstallOutcome::Installed { path })
        }
    }

    fn hook_format(&self) -> Option<&dyn HookFormat> {
        Some(&ClaudeHookFormat)
    }
}

struct ClaudeHookFormat;

#[derive(Deserialize)]
struct ToolInput {
    command: String,
}

#[derive(Deserialize)]
struct ClaudeHookEnvelope {
    /// Present in every real envelope; optional so a harness that omits it still works. When it IS
    /// present and names a different tool, we abstain — see `parse_input`.
    #[serde(default)]
    tool_name: Option<String>,
    tool_input: ToolInput,
    #[serde(default)]
    cwd: Option<String>,
    /// Claude Code's per-session UUID. Used only as the unforgeable anchor for recognizing this
    /// session's scratchpad (`pathctx::session_scratchpad`); absent → nothing is recognized.
    #[serde(default)]
    session_id: Option<String>,
}

impl HookFormat for ClaudeHookFormat {
    fn parse_input(&self, stdin: &str) -> Result<HookInput, ParseError> {
        let envelope: ClaudeHookEnvelope = serde_json::from_str(stdin).map_err(|e| ParseError {
            message: e.to_string(),
        })?;
        // Self-filter on the tool, as gemini/copilot/cursor already do. The configured matcher is
        // `Bash`, so normally only shell calls arrive — but a hand-edited matcher, or grok
        // auto-loading `~/.claude/settings.json`, can deliver others, and this target is
        // ALLOW-capable: emitting `permissionDecision: allow` for a `Read`/`Write`/`Edit` call
        // grants permission on a tool whose semantics were never analysed. Absent tool_name still
        // passes, so a harness that omits the field is unaffected.
        if let Some(name) = &envelope.tool_name
            && name != "Bash"
        {
            return Err(ParseError { message: format!("not a shell tool: {name}") });
        }
        Ok(HookInput {
            command: envelope.tool_input.command,
            cwd: envelope.cwd,
            root: super::env_root("CLAUDE_PROJECT_DIR"),
            session_id: envelope.session_id,
        })
    }

    fn decision_pointer(&self) -> &'static str {
        "/hookSpecificOutput/permissionDecision" // nested under hookSpecificOutput
    }

    fn render_response(&self, verdict: Verdict) -> HookResponse {
        if verdict.is_allowed() {
            let reason = allow_reason(verdict);
            let body = json!({
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "allow",
                    "permissionDecisionReason": reason,
                }
            });
            HookResponse {
                stdout: serde_json::to_string(&body).unwrap_or_default(),
                exit_code: 0,
            }
        } else {
            HookResponse {
                stdout: String::new(),
                exit_code: 0,
            }
        }
    }

    fn render_context(&self, context: &str) -> HookResponse {
        // additionalContext injects model-visible text without a
        // permissionDecision, so the normal approval flow (and the user's own
        // allowlist) is untouched.
        let body = json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "additionalContext": context,
            }
        });
        HookResponse {
            stdout: serde_json::to_string(&body).unwrap_or_default(),
            exit_code: 0,
        }
    }
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

fn has_safe_chains_hook(settings: &Value) -> bool {
    settings
        .get("hooks")
        .and_then(|h| h.get("PreToolUse"))
        .and_then(|arr| arr.as_array())
        .is_some_and(|entries| {
            entries.iter().any(|entry| {
                entry
                    .get("hooks")
                    .and_then(|h| h.as_array())
                    .is_some_and(|hooks| {
                        hooks.iter().any(|hook| {
                            hook.get("command")
                                .and_then(|c| c.as_str())
                                .is_some_and(|cmd| cmd.contains("safe-chains"))
                        })
                    })
            })
        })
}

fn add_hook(settings: &mut Value, binary: &str) -> Result<(), String> {
    super::append_hook_entry(settings, "hooks", "PreToolUse", hook_entry(binary))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verdict::SafetyLevel;

    fn target() -> ClaudeTarget {
        ClaudeTarget
    }

    #[test]
    fn install_no_claude_dir_skips() {
        let dir = tempfile::tempdir().unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::Skipped { .. }));
    }

    #[test]
    fn install_creates_settings_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".claude")).unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::Installed { .. }));
        let contents =
            std::fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
        let settings: Value = serde_json::from_str(&contents).unwrap();
        assert!(has_safe_chains_hook(&settings));
    }

    #[test]
    fn install_preserves_existing_settings() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir(&claude_dir).unwrap();
        std::fs::write(
            claude_dir.join("settings.json"),
            r#"{"permissions": {"allow": ["Bash(cargo test *)"]}}"#,
        )
        .unwrap();
        target().install(dir.path()).unwrap();
        let contents = std::fs::read_to_string(claude_dir.join("settings.json")).unwrap();
        let settings: Value = serde_json::from_str(&contents).unwrap();
        assert!(has_safe_chains_hook(&settings));
        assert!(
            settings
                .get("permissions")
                .and_then(|p| p.get("allow"))
                .is_some(),
            "existing permissions must be preserved"
        );
    }

    #[test]
    fn install_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".claude")).unwrap();
        target().install(dir.path()).unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::AlreadyConfigured { .. }));
    }

    #[test]
    fn detect_paths_returns_claude_dir() {
        let dir = tempfile::tempdir().unwrap();
        let paths = target().detect_paths(dir.path());
        assert_eq!(paths, vec![dir.path().join(".claude")]);
    }

    #[test]
    fn parse_input_extracts_command() {
        let stdin = r#"{"tool_input": {"command": "ls -la"}, "cwd": "/tmp"}"#;
        let parsed = ClaudeHookFormat.parse_input(stdin).unwrap();
        assert_eq!(parsed.command, "ls -la");
        assert_eq!(parsed.cwd.as_deref(), Some("/tmp"));
    }

    #[test]
    fn parse_input_rejects_garbage() {
        assert!(ClaudeHookFormat.parse_input("not json").is_err());
        assert!(ClaudeHookFormat.parse_input("{}").is_err());
    }

    #[test]
    fn render_response_allow_emits_allow_envelope() {
        let r = ClaudeHookFormat.render_response(Verdict::Allowed(SafetyLevel::Inert));
        assert_eq!(r.exit_code, 0);
        let v: Value = serde_json::from_str(&r.stdout).unwrap();
        assert_eq!(
            v.pointer("/hookSpecificOutput/permissionDecision")
                .and_then(|d| d.as_str()),
            Some("allow"),
        );
    }

    #[test]
    fn render_response_deny_emits_empty_body() {
        let r = ClaudeHookFormat.render_response(Verdict::Denied);
        assert_eq!(r.exit_code, 0);
        assert_eq!(r.stdout, "");
    }

    #[test]
    fn render_context_injects_additional_context_without_decision() {
        let r = ClaudeHookFormat.render_context("hello model");
        assert_eq!(r.exit_code, 0);
        let v: Value = serde_json::from_str(&r.stdout).unwrap();
        assert_eq!(
            v.pointer("/hookSpecificOutput/additionalContext")
                .and_then(|c| c.as_str()),
            Some("hello model"),
        );
        // Crucial: no permissionDecision, so the user's allowlist/flow is untouched.
        assert!(v.pointer("/hookSpecificOutput/permissionDecision").is_none());
    }

    #[test]
    fn render_response_safewrite_carries_appropriate_reason() {
        let r = ClaudeHookFormat.render_response(Verdict::Allowed(SafetyLevel::SafeWrite));
        let v: Value = serde_json::from_str(&r.stdout).unwrap();
        assert_eq!(
            v.pointer("/hookSpecificOutput/permissionDecisionReason")
                .and_then(|s| s.as_str()),
            Some(allow_reason(Verdict::Allowed(SafetyLevel::SafeWrite))),
        );
    }
}
