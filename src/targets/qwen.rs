use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::{HookFormat, HookInput, HookResponse, InstallOutcome, ParseError, Target, allow_reason};
use crate::verdict::Verdict;

pub struct QwenTarget;

impl Target for QwenTarget {
    fn name(&self) -> &'static str {
        "qwen"
    }

    fn display_name(&self) -> &'static str {
        "Qwen Code"
    }

    #[cfg(test)]
    fn sample_envelope(&self, tool: &str, command: &str) -> Option<String> {
        Some(format!(r#"{{"tool_name":"{tool}","tool_input":{{"command":"{command}"}}}}"#))
    }

    fn detect_paths(&self, home: &Path) -> Vec<PathBuf> {
        vec![home.join(".qwen")]
    }

    fn install(&self, home: &Path) -> Result<InstallOutcome, String> {
        let dir = home.join(".qwen");
        if !dir.exists() {
            return Ok(InstallOutcome::Skipped {
                reason: format!(
                    "~/.qwen not found at {} (Qwen Code not installed)",
                    dir.display()
                ),
            });
        }

        let path = dir.join("settings.json");
        let binary = "safe-chains hook qwen";

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
        Some(&QwenHookFormat)
    }
}

struct QwenHookFormat;

#[derive(Deserialize)]
struct ToolInput {
    command: String,
}

#[derive(Deserialize)]
struct QwenHookEnvelope {
    /// Optional so a harness that omits it still works; when present and naming another tool we
    /// abstain (see parse_input).
    #[serde(default)]
    tool_name: Option<String>,
    tool_input: ToolInput,
    #[serde(default)]
    cwd: Option<String>,
}

impl HookFormat for QwenHookFormat {
    fn parse_input(&self, stdin: &str) -> Result<HookInput, ParseError> {
        let envelope: QwenHookEnvelope = serde_json::from_str(stdin).map_err(|e| ParseError {
            message: e.to_string(),
        })?;
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
            root: super::env_root("QWEN_PROJECT_DIR"),
            // No scratchpad layout researched for this harness yet (see docs/design/agent-scratchpad.md).
            session_id: None,
        })
    }

    fn decision_pointer(&self) -> &'static str {
        "/hookSpecificOutput/permissionDecision" // mirrors Claude's nesting
    }

    fn render_response(&self, verdict: Verdict) -> HookResponse {
        if verdict.is_allowed() {
            let reason = allow_reason(verdict);
            // Qwen mirrors Claude Code's hookSpecificOutput envelope.
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
        // Qwen mirrors Claude Code's hookSpecificOutput envelope, including
        // additionalContext (injects model-visible text, no permission decision).
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
        "matcher": "^Bash$",
        "hooks": [{
            "type": "command",
            "command": binary,
            "timeout": 60_000,
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

    fn target() -> QwenTarget {
        QwenTarget
    }

    /// Verbatim shape from the Qwen Code hooks docs.
    const QWEN_DOCS_SAMPLE: &str = r#"{
        "session_id": "abc123",
        "transcript_path": "/Users/me/.qwen/transcripts/abc.json",
        "cwd": "/Users/me/project",
        "hook_event_name": "PreToolUse",
        "timestamp": "2026-05-06T12:00:00Z",
        "permission_mode": "default",
        "tool_name": "Bash",
        "tool_input": {"command": "ls -la"},
        "tool_use_id": "tu_123"
    }"#;

    #[test]
    fn install_no_qwen_dir_skips() {
        let dir = tempfile::tempdir().unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::Skipped { .. }));
    }

    #[test]
    fn install_creates_settings_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".qwen")).unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::Installed { .. }));
        let contents = std::fs::read_to_string(dir.path().join(".qwen/settings.json")).unwrap();
        let settings: Value = serde_json::from_str(&contents).unwrap();
        assert!(has_safe_chains_hook(&settings));
    }

    #[test]
    fn install_uses_bash_matcher() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".qwen")).unwrap();
        target().install(dir.path()).unwrap();
        let contents = std::fs::read_to_string(dir.path().join(".qwen/settings.json")).unwrap();
        assert!(contents.contains("^Bash$"));
        assert!(contents.contains("safe-chains hook qwen"));
    }

    #[test]
    fn install_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".qwen")).unwrap();
        target().install(dir.path()).unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::AlreadyConfigured { .. }));
    }

    #[test]
    fn parse_input_extracts_command() {
        let parsed = QwenHookFormat.parse_input(QWEN_DOCS_SAMPLE).unwrap();
        assert_eq!(parsed.command, "ls -la");
        assert_eq!(parsed.cwd.as_deref(), Some("/Users/me/project"));
    }

    #[test]
    fn parse_input_rejects_garbage() {
        assert!(QwenHookFormat.parse_input("not json").is_err());
        assert!(QwenHookFormat.parse_input("{}").is_err());
    }

    #[test]
    fn render_response_emits_claude_shaped_envelope() {
        let r = QwenHookFormat.render_response(Verdict::Allowed(SafetyLevel::Inert));
        let v: Value = serde_json::from_str(&r.stdout).unwrap();
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
    fn render_response_deny_emits_empty_body() {
        let r = QwenHookFormat.render_response(Verdict::Denied);
        assert_eq!(r.stdout, "");
    }
}
