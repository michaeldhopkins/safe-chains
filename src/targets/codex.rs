use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::{HookFormat, HookInput, HookResponse, InstallOutcome, ParseError, Target};
use crate::verdict::Verdict;

pub struct CodexTarget;

impl Target for CodexTarget {
    fn name(&self) -> &'static str {
        "codex"
    }

    fn display_name(&self) -> &'static str {
        "Codex (OpenAI)"
    }

    fn detect_paths(&self, home: &Path) -> Vec<PathBuf> {
        vec![home.join(".codex")]
    }

    fn install(&self, home: &Path) -> Result<InstallOutcome, String> {
        let dir = home.join(".codex");
        if !dir.exists() {
            return Ok(InstallOutcome::Skipped {
                reason: format!(
                    "~/.codex not found at {} (Codex CLI not installed)",
                    dir.display()
                ),
            });
        }

        let path = dir.join("hooks.json");
        let binary = "safe-chains hook codex";

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
        Some(&CodexHookFormat)
    }
}

struct CodexHookFormat;

#[derive(Deserialize)]
struct ToolInput {
    command: String,
}

#[derive(Deserialize)]
struct CodexHookEnvelope {
    tool_input: ToolInput,
    #[serde(default)]
    cwd: Option<String>,
}

impl HookFormat for CodexHookFormat {
    fn parse_input(&self, stdin: &str) -> Result<HookInput, ParseError> {
        let envelope: CodexHookEnvelope = serde_json::from_str(stdin).map_err(|e| ParseError {
            message: e.to_string(),
        })?;
        Ok(HookInput {
            command: envelope.tool_input.command,
            cwd: envelope.cwd,
            root: None, // codex sends cwd but no distinct project root (HARNESS-BEHAVIORS.md)
        })
    }

    fn render_response(&self, _verdict: Verdict) -> HookResponse {
        // SAFE command → emit nothing. Codex has no `grant`: `permissionDecision:"allow"` is
        // rejected as unsupported on v0.144.3 (docs list it, but it errored — version drift), and
        // Codex "continues on unsupported output" anyway. Silence lets the safe command run through
        // Codex's own flow, version-robustly. (Gated commands go through `render_deny`, not here.)
        HookResponse {
            stdout: String::new(),
            exit_code: 0,
        }
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

fn add_hook(settings: &mut Value, binary: &str) {
    if !settings.is_object() {
        *settings = json!({});
    }
    let obj = settings.as_object_mut().expect("settings was just set to an object");
    // Codex nests lifecycle events under a top-level `hooks` object (NOT Claude's flat
    // `PreToolUse` key) — a flat key makes Codex reject the whole file. See developers.openai.com/codex/hooks.
    let hooks = obj.entry("hooks").or_insert_with(|| json!({}));
    if !hooks.is_object() {
        *hooks = json!({});
    }
    let pre_tool_use = hooks
        .as_object_mut()
        .expect("hooks was just set to an object")
        .entry("PreToolUse")
        .or_insert_with(|| json!([]));
    if !pre_tool_use.is_array() {
        *pre_tool_use = json!([]);
    }
    pre_tool_use
        .as_array_mut()
        .expect("PreToolUse was just set to an array")
        .push(hook_entry(binary));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verdict::SafetyLevel;

    fn target() -> CodexTarget {
        CodexTarget
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
        let contents = std::fs::read_to_string(dir.path().join(".codex/hooks.json")).unwrap();
        let settings: Value = serde_json::from_str(&contents).unwrap();
        assert!(has_safe_chains_hook(&settings));
        // Codex nests events under a top-level `hooks` object; a flat top-level `PreToolUse`
        // (Claude's shape) makes Codex reject the entire file (`unknown field PreToolUse`).
        assert!(settings.get("hooks").and_then(|h| h.get("PreToolUse")).is_some());
        assert!(settings.get("PreToolUse").is_none(), "must not use Claude's flat PreToolUse key");
    }

    #[test]
    fn install_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".codex")).unwrap();
        target().install(dir.path()).unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::AlreadyConfigured { .. }));
    }

    #[test]
    fn install_uses_subcommand_invocation() {
        // The binary entry must be `safe-chains hook codex`, not just
        // `safe-chains`, so the runtime knows which envelope to emit.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".codex")).unwrap();
        target().install(dir.path()).unwrap();
        let contents = std::fs::read_to_string(dir.path().join(".codex/hooks.json")).unwrap();
        assert!(contents.contains("safe-chains hook codex"));
    }

    #[test]
    fn install_preserves_existing_hooks() {
        let dir = tempfile::tempdir().unwrap();
        let codex_dir = dir.path().join(".codex");
        std::fs::create_dir(&codex_dir).unwrap();
        std::fs::write(
            codex_dir.join("hooks.json"),
            r#"{"PostToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "log-it"}]}]}"#,
        )
        .unwrap();
        target().install(dir.path()).unwrap();
        let contents = std::fs::read_to_string(codex_dir.join("hooks.json")).unwrap();
        let settings: Value = serde_json::from_str(&contents).unwrap();
        assert!(has_safe_chains_hook(&settings));
        assert!(
            settings.get("PostToolUse").is_some(),
            "existing PostToolUse must be preserved"
        );
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
        // Codex has no `grant` — `permissionDecision:"allow"` is unsupported on v0.144.3. A safe
        // command emits nothing (Codex continues → runs it); it must NOT emit an allow envelope.
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
}
