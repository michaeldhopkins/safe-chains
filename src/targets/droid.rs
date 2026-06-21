use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::{HookFormat, HookInput, HookResponse, InstallOutcome, ParseError, Target, allow_reason};
use crate::verdict::Verdict;

pub struct DroidTarget;

impl Target for DroidTarget {
    fn name(&self) -> &'static str {
        "droid"
    }

    fn display_name(&self) -> &'static str {
        "Factory Droid"
    }

    fn detect_paths(&self, home: &Path) -> Vec<PathBuf> {
        vec![home.join(".factory")]
    }

    fn install(&self, home: &Path) -> Result<InstallOutcome, String> {
        let dir = home.join(".factory");
        if !dir.exists() {
            return Ok(InstallOutcome::Skipped {
                reason: format!(
                    "~/.factory not found at {} (Factory Droid not installed)",
                    dir.display()
                ),
            });
        }

        let path = dir.join("settings.json");
        // Droid docs require absolute paths for hook commands. We
        // discover the absolute path of the running binary and embed
        // it in the config; falls back to bare "safe-chains hook
        // droid" if discovery fails (and the install message warns).
        let resolved = std::env::current_exe()
            .ok()
            .and_then(|p| p.canonicalize().ok())
            .map(|p| format!("{} hook droid", p.display()))
            .unwrap_or_else(|| "safe-chains hook droid".to_string());
        let binary = resolved.as_str();

        if path.exists() {
            let contents = std::fs::read_to_string(&path)
                .map_err(|e| format!("Could not read {}: {e}", path.display()))?;
            let mut settings: Value = serde_json::from_str(&contents)
                .map_err(|e| format!("Could not parse {}: {e}", path.display()))?;

            if has_safe_chains_hook(&settings) {
                return Ok(InstallOutcome::AlreadyConfigured { path });
            }

            add_hook(&mut settings, binary);
            let output = serde_json::to_string_pretty(&settings).expect("serializing valid JSON");
            std::fs::write(&path, format!("{output}\n"))
                .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
            Ok(InstallOutcome::Installed { path })
        } else {
            let mut settings = Value::Object(Map::new());
            add_hook(&mut settings, binary);
            let output = serde_json::to_string_pretty(&settings).expect("serializing valid JSON");
            std::fs::write(&path, format!("{output}\n"))
                .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
            Ok(InstallOutcome::Installed { path })
        }
    }

    fn hook_format(&self) -> Option<&dyn HookFormat> {
        Some(&DroidHookFormat)
    }
}

struct DroidHookFormat;

#[derive(Deserialize)]
struct ToolInput {
    command: String,
}

#[derive(Deserialize)]
struct DroidHookEnvelope {
    tool_input: ToolInput,
    #[serde(default)]
    cwd: Option<String>,
}

impl HookFormat for DroidHookFormat {
    fn parse_input(&self, stdin: &str) -> Result<HookInput, ParseError> {
        let envelope: DroidHookEnvelope = serde_json::from_str(stdin).map_err(|e| ParseError {
            message: e.to_string(),
        })?;
        Ok(HookInput {
            command: envelope.tool_input.command,
            cwd: envelope.cwd,
        })
    }

    fn render_response(&self, verdict: Verdict) -> HookResponse {
        if verdict.is_allowed() {
            let reason = allow_reason(verdict);
            // Droid mirrors Claude Code's hookSpecificOutput envelope.
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
        // Droid mirrors Claude Code's hookSpecificOutput envelope, including
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
    // Droid's bash tool name is `Execute`, not `Bash`. timeout is in
    // seconds (different from Qwen/Gemini ms).
    json!({
        "matcher": "Execute",
        "hooks": [{
            "type": "command",
            "command": binary,
            "timeout": 60,
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

fn add_hook(settings: &mut Value, binary: &str) {
    if !settings.is_object() {
        *settings = json!({});
    }
    let Some(obj) = settings.as_object_mut() else {
        unreachable!("settings was just set to an object");
    };
    let hooks = obj
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .expect("hooks key was created above as an object");
    let pre_tool_use = hooks
        .entry("PreToolUse")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .expect("PreToolUse was created above as an array");
    pre_tool_use.push(hook_entry(binary));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verdict::SafetyLevel;

    fn target() -> DroidTarget {
        DroidTarget
    }

    /// Verbatim shape from the Factory Droid hooks reference. Note
    /// tool_name is "Execute", not "Bash".
    const DROID_DOCS_SAMPLE: &str = r#"{
        "session_id": "abc123",
        "transcript_path": "/Users/me/.factory/projects/p/uuid.jsonl",
        "cwd": "/Users/me/project",
        "permission_mode": "off",
        "hook_event_name": "PreToolUse",
        "tool_name": "Execute",
        "tool_input": {"command": "ls -la"}
    }"#;

    #[test]
    fn install_no_factory_dir_skips() {
        let dir = tempfile::tempdir().unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::Skipped { .. }));
    }

    #[test]
    fn install_creates_settings_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".factory")).unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::Installed { .. }));
        let contents = std::fs::read_to_string(dir.path().join(".factory/settings.json")).unwrap();
        let settings: Value = serde_json::from_str(&contents).unwrap();
        assert!(has_safe_chains_hook(&settings));
    }

    #[test]
    fn install_uses_execute_matcher_not_bash() {
        // Droid's bash tool is `Execute` — wiring a `Bash` matcher
        // wouldn't fire on shell calls.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".factory")).unwrap();
        target().install(dir.path()).unwrap();
        let contents = std::fs::read_to_string(dir.path().join(".factory/settings.json")).unwrap();
        assert!(contents.contains("\"matcher\": \"Execute\""));
    }

    #[test]
    fn install_uses_absolute_path_to_binary() {
        // Droid docs explicitly require absolute paths for hook
        // commands. We resolve via env::current_exe.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".factory")).unwrap();
        target().install(dir.path()).unwrap();
        let contents = std::fs::read_to_string(dir.path().join(".factory/settings.json")).unwrap();
        let settings: Value = serde_json::from_str(&contents).unwrap();
        let cmd = settings
            .pointer("/hooks/PreToolUse/0/hooks/0/command")
            .and_then(|s| s.as_str())
            .unwrap_or("");
        // Either an absolute path or the fallback bare invocation.
        assert!(
            cmd.starts_with('/') || cmd == "safe-chains hook droid",
            "unexpected command: {cmd}",
        );
        assert!(cmd.ends_with(" hook droid") || cmd == "safe-chains hook droid");
    }

    #[test]
    fn install_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".factory")).unwrap();
        target().install(dir.path()).unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::AlreadyConfigured { .. }));
    }

    #[test]
    fn parse_input_extracts_command() {
        let parsed = DroidHookFormat.parse_input(DROID_DOCS_SAMPLE).unwrap();
        assert_eq!(parsed.command, "ls -la");
        assert_eq!(parsed.cwd.as_deref(), Some("/Users/me/project"));
    }

    #[test]
    fn parse_input_rejects_garbage() {
        assert!(DroidHookFormat.parse_input("not json").is_err());
        assert!(DroidHookFormat.parse_input("{}").is_err());
    }

    #[test]
    fn render_response_emits_claude_shaped_envelope() {
        let r = DroidHookFormat.render_response(Verdict::Allowed(SafetyLevel::Inert));
        let v: Value = serde_json::from_str(&r.stdout).unwrap();
        assert_eq!(
            v.pointer("/hookSpecificOutput/permissionDecision")
                .and_then(|d| d.as_str()),
            Some("allow"),
        );
    }

    #[test]
    fn render_response_deny_emits_empty_body() {
        let r = DroidHookFormat.render_response(Verdict::Denied);
        assert_eq!(r.stdout, "");
    }
}
