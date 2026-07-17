use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Value, json};

use super::{HookFormat, HookInput, HookResponse, InstallOutcome, ParseError, Target, allow_reason};
use crate::verdict::Verdict;

pub struct CursorTarget;

impl Target for CursorTarget {
    fn name(&self) -> &'static str {
        "cursor"
    }

    fn display_name(&self) -> &'static str {
        "Cursor CLI"
    }

    fn detect_paths(&self, home: &Path) -> Vec<PathBuf> {
        vec![home.join(".cursor")]
    }

    fn install(&self, home: &Path) -> Result<InstallOutcome, String> {
        let dir = home.join(".cursor");
        if !dir.exists() {
            return Ok(InstallOutcome::Skipped {
                reason: format!(
                    "~/.cursor not found at {} (Cursor not installed for this user)",
                    dir.display()
                ),
            });
        }

        let path = dir.join("hooks.json");
        let binary = "safe-chains hook cursor";

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
            let mut settings = json!({"version": 1});
            add_hook(&mut settings, binary);
            let output = serde_json::to_string_pretty(&settings).expect("serializing valid JSON");
            std::fs::write(&path, format!("{output}\n"))
                .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
            Ok(InstallOutcome::Installed { path })
        }
    }

    fn hook_format(&self) -> Option<&dyn HookFormat> {
        Some(&CursorHookFormat)
    }
}

struct CursorHookFormat;

#[derive(Deserialize)]
struct CursorHookEnvelope {
    command: String,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    workspace_roots: Vec<String>,
}

impl HookFormat for CursorHookFormat {
    fn parse_input(&self, stdin: &str) -> Result<HookInput, ParseError> {
        let mut envelope: CursorHookEnvelope =
            serde_json::from_str(stdin).map_err(|e| ParseError { message: e.to_string() })?;
        Ok(HookInput {
            command: envelope.command,
            cwd: envelope.cwd,
            // cursor sends the project root(s) in the payload; take the first.
            root: (!envelope.workspace_roots.is_empty()).then(|| envelope.workspace_roots.swap_remove(0)),
        })
    }

    fn render_response(&self, verdict: Verdict) -> HookResponse {
        if verdict.is_allowed() {
            let reason = allow_reason(verdict);
            // cursor-agent (v2026.07.16) IGNORES a hook `permission:"allow"` — its own command
            // allowlist still prompts (a known bug: forum.cursor.com/t/…/144244, HARNESS-BEHAVIORS
            // §Cursor). We keep emitting it anyway: it is harmless (cursor just prompts, as it would
            // on silence) and becomes a real grant the moment cursor honors `allow`.
            let body = json!({
                "permission": "allow",
                "agent_message": reason,
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

    // cursor-agent ignores hook `allow` (above) but HONORS `deny` — verified live: a `permission:
    // "deny"` blocks the command and shows our message. Since `allow` is inert, `deny` is the only
    // lever that adds protection, so Cursor is a DENY harness (like Codex). Revisit if cursor fixes
    // `allow`, or if the Cursor IDE differs from the CLI. See HARNESS-BEHAVIORS §Cursor.
    fn gated_policy(&self) -> super::GatedPolicy {
        super::GatedPolicy::Deny
    }

    fn render_deny(&self, reason: &str) -> HookResponse {
        let body = json!({
            "permission": "deny",
            "user_message": reason,
            "agent_message": reason,
        });
        HookResponse {
            stdout: serde_json::to_string(&body).unwrap_or_default(),
            exit_code: 0,
        }
    }
}

fn hook_entry(binary: &str) -> Value {
    json!({
        "command": binary,
        "timeout": 30,
    })
}

fn has_safe_chains_hook(settings: &Value) -> bool {
    settings
        .get("hooks")
        .and_then(|h| h.get("beforeShellExecution"))
        .and_then(|arr| arr.as_array())
        .is_some_and(|entries| {
            entries.iter().any(|entry| {
                entry
                    .get("command")
                    .and_then(|c| c.as_str())
                    .is_some_and(|cmd| cmd.contains("safe-chains"))
            })
        })
}

fn add_hook(settings: &mut Value, binary: &str) {
    if !settings.is_object() {
        *settings = json!({"version": 1});
    }
    let Some(obj) = settings.as_object_mut() else {
        unreachable!("settings was just set to an object");
    };
    if !obj.contains_key("version") {
        obj.insert("version".to_string(), json!(1));
    }
    let hooks = obj
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .expect("hooks key was created above as an object");
    let before_shell = hooks
        .entry("beforeShellExecution")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .expect("beforeShellExecution was created above as an array");
    before_shell.push(hook_entry(binary));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verdict::SafetyLevel;

    fn target() -> CursorTarget {
        CursorTarget
    }

    #[test]
    fn install_no_cursor_dir_skips() {
        let dir = tempfile::tempdir().unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::Skipped { .. }));
    }

    #[test]
    fn install_creates_hooks_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".cursor")).unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::Installed { .. }));
        let contents = std::fs::read_to_string(dir.path().join(".cursor/hooks.json")).unwrap();
        let settings: Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(settings.get("version").and_then(|v| v.as_u64()), Some(1));
        assert!(has_safe_chains_hook(&settings));
    }

    #[test]
    fn install_uses_subcommand_invocation() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".cursor")).unwrap();
        target().install(dir.path()).unwrap();
        let contents = std::fs::read_to_string(dir.path().join(".cursor/hooks.json")).unwrap();
        assert!(contents.contains("safe-chains hook cursor"));
    }

    #[test]
    fn install_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".cursor")).unwrap();
        target().install(dir.path()).unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::AlreadyConfigured { .. }));
    }

    #[test]
    fn install_preserves_existing_hooks() {
        let dir = tempfile::tempdir().unwrap();
        let cursor_dir = dir.path().join(".cursor");
        std::fs::create_dir(&cursor_dir).unwrap();
        std::fs::write(
            cursor_dir.join("hooks.json"),
            r#"{"version": 1, "hooks": {"afterFileEdit": [{"command": "format-it", "timeout": 30}]}}"#,
        )
        .unwrap();
        target().install(dir.path()).unwrap();
        let contents = std::fs::read_to_string(cursor_dir.join("hooks.json")).unwrap();
        let settings: Value = serde_json::from_str(&contents).unwrap();
        assert!(has_safe_chains_hook(&settings));
        assert!(
            settings
                .pointer("/hooks/afterFileEdit")
                .and_then(|a| a.as_array())
                .is_some_and(|a| !a.is_empty()),
            "existing afterFileEdit hook must be preserved"
        );
    }

    /// Verbatim sample payload from cursor.com/docs/hooks for the
    /// `beforeShellExecution` event. Bash command is at top level
    /// (not nested in tool_input as Claude/Codex do).
    const CURSOR_DOCS_SAMPLE: &str = r#"{
        "conversation_id": "abc-123",
        "generation_id": "gen-456",
        "model": "claude-sonnet-4-5",
        "hook_event_name": "beforeShellExecution",
        "cursor_version": "2.0.43",
        "workspace_roots": ["/Users/me/project"],
        "user_email": "me@example.com",
        "transcript_path": "/Users/me/.cursor/transcripts/abc.json",
        "command": "ls -la",
        "cwd": "/Users/me/project",
        "sandbox": false
    }"#;

    #[test]
    fn parse_input_extracts_top_level_command() {
        let parsed = CursorHookFormat.parse_input(CURSOR_DOCS_SAMPLE).unwrap();
        assert_eq!(parsed.command, "ls -la");
        assert_eq!(parsed.cwd.as_deref(), Some("/Users/me/project"));
    }

    #[test]
    fn parse_input_rejects_garbage() {
        assert!(CursorHookFormat.parse_input("not json").is_err());
        assert!(CursorHookFormat.parse_input("{}").is_err());
    }

    #[test]
    fn parse_input_takes_the_project_root_from_workspace_roots() {
        let stdin = r#"{"command": "ls", "cwd": "/w/p/sub", "workspace_roots": ["/w/p", "/w/other"]}"#;
        let parsed = CursorHookFormat.parse_input(stdin).unwrap();
        assert_eq!(parsed.cwd.as_deref(), Some("/w/p/sub"));
        assert_eq!(parsed.root.as_deref(), Some("/w/p"), "first workspace root");
        // absent workspace_roots → no root
        let bare = CursorHookFormat.parse_input(r#"{"command": "ls"}"#).unwrap();
        assert_eq!(bare.root, None);
    }

    #[test]
    fn render_response_uses_permission_key_not_decision() {
        // Cursor's contract is `permission`, NOT `decision` /
        // `permissionDecision`. Wiring this wrong is silently fail-
        // open per their failure semantics — tested explicitly.
        let r = CursorHookFormat.render_response(Verdict::Allowed(SafetyLevel::Inert));
        let v: Value = serde_json::from_str(&r.stdout).unwrap();
        assert_eq!(v.get("permission").and_then(|s| s.as_str()), Some("allow"));
        assert!(v.get("decision").is_none());
        assert!(v.get("permissionDecision").is_none());
    }

    #[test]
    fn render_response_includes_agent_message() {
        let r = CursorHookFormat.render_response(Verdict::Allowed(SafetyLevel::Inert));
        let v: Value = serde_json::from_str(&r.stdout).unwrap();
        assert!(v.get("agent_message").and_then(|s| s.as_str()).is_some());
    }

    #[test]
    fn render_response_deny_emits_empty_body() {
        // render_response is only called for ALLOWED verdicts; the Denied branch is defensive.
        let r = CursorHookFormat.render_response(Verdict::Denied);
        assert_eq!(r.stdout, "");
    }

    #[test]
    fn cursor_is_a_deny_harness() {
        // cursor-agent honors `deny` (verified live) but ignores `allow`, so gated commands are
        // VETOED rather than deferred — the only lever that adds protection.
        assert_eq!(CursorHookFormat.gated_policy(), super::super::GatedPolicy::Deny);
    }

    #[test]
    fn render_deny_emits_permission_deny_with_message() {
        let r = CursorHookFormat.render_deny("safe-chains blocked this: not on the allowlist");
        let v: Value = serde_json::from_str(&r.stdout).unwrap();
        assert_eq!(v.get("permission").and_then(|s| s.as_str()), Some("deny"));
        // Cursor renders `user_message` in the client and passes `agent_message` to the model.
        assert_eq!(
            v.get("user_message").and_then(|s| s.as_str()),
            Some("safe-chains blocked this: not on the allowlist"),
        );
        assert!(v.get("agent_message").and_then(|s| s.as_str()).is_some());
        assert!(v.get("permissionDecision").is_none());
    }
}
