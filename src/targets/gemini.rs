use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::{HookFormat, HookInput, HookResponse, InstallOutcome, ParseError, Target, allow_reason};
use crate::verdict::Verdict;

pub struct GeminiTarget;

impl Target for GeminiTarget {
    fn name(&self) -> &'static str {
        "gemini"
    }

    fn display_name(&self) -> &'static str {
        "Gemini CLI"
    }

    fn detect_paths(&self, home: &Path) -> Vec<PathBuf> {
        vec![home.join(".gemini")]
    }

    fn install(&self, home: &Path) -> Result<InstallOutcome, String> {
        let dir = home.join(".gemini");
        if !dir.exists() {
            return Ok(InstallOutcome::Skipped {
                reason: format!(
                    "~/.gemini not found at {} (Gemini CLI not installed)",
                    dir.display()
                ),
            });
        }

        let path = dir.join("settings.json");
        let binary = "safe-chains hook gemini";

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
        Some(&GeminiHookFormat)
    }
}

struct GeminiHookFormat;

#[derive(Deserialize)]
struct ToolInput {
    command: String,
}

#[derive(Deserialize)]
struct GeminiHookEnvelope {
    #[serde(default)]
    tool_name: Option<String>,
    tool_input: ToolInput,
    #[serde(default)]
    cwd: Option<String>,
}

impl HookFormat for GeminiHookFormat {
    fn parse_input(&self, stdin: &str) -> Result<HookInput, ParseError> {
        let envelope: GeminiHookEnvelope = serde_json::from_str(stdin).map_err(|e| ParseError {
            message: e.to_string(),
        })?;
        // Gemini's matcher narrows to run_shell_command in config, but
        // some setups may dispatch all tools through the same hook.
        // For non-shell tools, return Err so the runtime exits 0
        // silently — equivalent to "no opinion" — and Gemini falls
        // back to its own permission rules.
        if let Some(name) = &envelope.tool_name
            && name != "run_shell_command"
            && name != "Shell"
        {
            return Err(ParseError {
                message: format!("not a shell tool: {name}"),
            });
        }
        Ok(HookInput {
            command: envelope.tool_input.command,
            cwd: envelope.cwd,
        })
    }

    fn render_response(&self, verdict: Verdict) -> HookResponse {
        if verdict.is_allowed() {
            let reason = allow_reason(verdict);
            // Gemini contract: `decision` (not permission /
            // permissionDecision). Values: "allow" or "deny" only —
            // no "ask".
            let body = json!({
                "decision": "allow",
                "reason": reason,
            });
            HookResponse {
                stdout: serde_json::to_string(&body).unwrap_or_default(),
                exit_code: 0,
            }
        } else {
            // Empty stdout is "no opinion" — Gemini's docs note that
            // exit code drives the outcome and an unparseable stdout
            // is a warning. Exit 0 + empty body lets Gemini's own
            // permission system handle it.
            HookResponse {
                stdout: String::new(),
                exit_code: 0,
            }
        }
    }
}

fn hook_entry(binary: &str) -> Value {
    json!({
        "matcher": "^run_shell_command$",
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
        .and_then(|h| h.get("BeforeTool"))
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
    let before_tool = hooks
        .entry("BeforeTool")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .expect("BeforeTool was created above as an array");
    before_tool.push(hook_entry(binary));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verdict::SafetyLevel;

    fn target() -> GeminiTarget {
        GeminiTarget
    }

    /// Verbatim shape from the Gemini CLI hooks reference. The bash
    /// command lives in tool_input.command, matched by tool_name.
    const GEMINI_DOCS_SAMPLE: &str = r#"{
        "session_id": "abc123",
        "transcript_path": "/Users/me/.gemini/transcripts/abc.json",
        "cwd": "/Users/me/project",
        "hook_event_name": "BeforeTool",
        "timestamp": "2026-05-06T12:00:00Z",
        "tool_name": "run_shell_command",
        "tool_input": {"command": "ls -la"}
    }"#;

    #[test]
    fn install_no_gemini_dir_skips() {
        let dir = tempfile::tempdir().unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::Skipped { .. }));
    }

    #[test]
    fn install_creates_settings_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".gemini")).unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::Installed { .. }));
        let contents = std::fs::read_to_string(dir.path().join(".gemini/settings.json")).unwrap();
        let settings: Value = serde_json::from_str(&contents).unwrap();
        assert!(has_safe_chains_hook(&settings));
    }

    #[test]
    fn install_uses_subcommand_invocation() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".gemini")).unwrap();
        target().install(dir.path()).unwrap();
        let contents = std::fs::read_to_string(dir.path().join(".gemini/settings.json")).unwrap();
        assert!(contents.contains("safe-chains hook gemini"));
    }

    #[test]
    fn install_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".gemini")).unwrap();
        target().install(dir.path()).unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::AlreadyConfigured { .. }));
    }

    #[test]
    fn parse_input_extracts_command_from_tool_input() {
        let parsed = GeminiHookFormat.parse_input(GEMINI_DOCS_SAMPLE).unwrap();
        assert_eq!(parsed.command, "ls -la");
        assert_eq!(parsed.cwd.as_deref(), Some("/Users/me/project"));
    }

    #[test]
    fn parse_input_skips_non_shell_tool_names() {
        // If the matcher in config doesn't narrow to run_shell_command,
        // a non-shell tool may dispatch through. We return Err so the
        // runtime exits silently — Gemini falls back to its own perms.
        let stdin = r#"{"tool_name": "list_files", "tool_input": {"command": "ignored"}}"#;
        assert!(GeminiHookFormat.parse_input(stdin).is_err());
    }

    #[test]
    fn parse_input_rejects_garbage() {
        assert!(GeminiHookFormat.parse_input("not json").is_err());
        assert!(GeminiHookFormat.parse_input("{}").is_err());
    }

    #[test]
    fn render_response_uses_decision_key_not_permission() {
        // Gemini contract is `decision`, NOT `permission` /
        // `permissionDecision`. Wiring this wrong silently fails the
        // hook (warning, action proceeds) rather than blocking.
        let r = GeminiHookFormat.render_response(Verdict::Allowed(SafetyLevel::Inert));
        let v: Value = serde_json::from_str(&r.stdout).unwrap();
        assert_eq!(v.get("decision").and_then(|s| s.as_str()), Some("allow"));
        assert!(v.get("permission").is_none());
        assert!(v.get("permissionDecision").is_none());
    }

    #[test]
    fn render_response_includes_reason() {
        let r = GeminiHookFormat.render_response(Verdict::Allowed(SafetyLevel::SafeWrite));
        let v: Value = serde_json::from_str(&r.stdout).unwrap();
        assert!(v.get("reason").and_then(|s| s.as_str()).is_some());
    }

    #[test]
    fn render_response_deny_emits_empty_body() {
        let r = GeminiHookFormat.render_response(Verdict::Denied);
        assert_eq!(r.stdout, "");
    }

    #[test]
    fn install_uses_correct_matcher() {
        // Gemini's matcher is regex on tool name; `^run_shell_command$`
        // is the canonical shell-tool matcher.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".gemini")).unwrap();
        target().install(dir.path()).unwrap();
        let contents = std::fs::read_to_string(dir.path().join(".gemini/settings.json")).unwrap();
        assert!(contents.contains("run_shell_command"));
    }
}
