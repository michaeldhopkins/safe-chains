use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Value, json};

use super::{HookFormat, HookInput, HookResponse, InstallOutcome, ParseError, Target};
use crate::verdict::Verdict;

pub struct GrokTarget;

impl Target for GrokTarget {
    fn name(&self) -> &'static str {
        "grok"
    }

    fn display_name(&self) -> &'static str {
        "Grok CLI (xAI)"
    }

    fn detect_paths(&self, home: &Path) -> Vec<PathBuf> {
        vec![home.join(".grok")]
    }

    /// Grok discovers hooks from every `~/.grok/hooks/*.json` (globally trusted, no folder-trust
    /// needed), so we own a DEDICATED `safe-chains.json` rather than editing a shared file — no risk
    /// of clobbering the user's other hook files, and idempotency is trivial.
    fn install(&self, home: &Path) -> Result<InstallOutcome, String> {
        let dir = home.join(".grok");
        if !dir.exists() {
            return Ok(InstallOutcome::Skipped {
                reason: format!("~/.grok not found at {} (Grok CLI not installed)", dir.display()),
            });
        }

        let hooks_dir = dir.join("hooks");
        let path = hooks_dir.join("safe-chains.json");
        let binary = "safe-chains hook grok";

        if path.exists()
            && let Ok(contents) = std::fs::read_to_string(&path)
            && let Ok(value) = serde_json::from_str::<Value>(&contents)
            && has_safe_chains_hook(&value)
        {
            return Ok(InstallOutcome::AlreadyConfigured { path });
        }

        std::fs::create_dir_all(&hooks_dir)
            .map_err(|e| format!("Could not create {}: {e}", hooks_dir.display()))?;
        let output = serde_json::to_string_pretty(&hook_file(binary)).expect("serializing valid JSON");
        std::fs::write(&path, format!("{output}\n"))
            .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
        Ok(InstallOutcome::Installed { path })
    }

    fn hook_format(&self) -> Option<&dyn HookFormat> {
        Some(&GrokHookFormat)
    }
}

struct GrokHookFormat;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GrokToolInput {
    command: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GrokHookEnvelope {
    tool_input: GrokToolInput,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    workspace_root: Option<String>,
}

impl HookFormat for GrokHookFormat {
    /// Grok's PreToolUse envelope is camelCase (`toolInput.command`, `workspaceRoot`) — unlike
    /// Claude/Codex snake_case. Getting the casing wrong parses to nothing and fails OPEN, so it is
    /// pinned by `parse_input_rejects_snake_case_envelope` below.
    fn parse_input(&self, stdin: &str) -> Result<HookInput, ParseError> {
        let envelope: GrokHookEnvelope =
            serde_json::from_str(stdin).map_err(|e| ParseError { message: e.to_string() })?;
        Ok(HookInput {
            command: envelope.tool_input.command,
            cwd: envelope.cwd,
            // The project root arrives in the payload as `workspaceRoot`; grok also exports it as
            // `GROK_WORKSPACE_ROOT` and (for Claude compat) `CLAUDE_PROJECT_DIR`.
            root: envelope
                .workspace_root
                .or_else(|| super::env_root("GROK_WORKSPACE_ROOT"))
                .or_else(|| super::env_root("CLAUDE_PROJECT_DIR")),
        })
    }

    fn render_response(&self, verdict: Verdict) -> HookResponse {
        // A safe command → `allow`. Grok treats a hook `allow` as "declines to deny", NOT a grant:
        // the command still runs grok's own permission gauntlet and may prompt (so safe-chains cannot
        // auto-approve on grok — same as Cursor/Codex). Emitting it is honest and harmless, and
        // becomes a real grant if grok ever promotes `allow`. `decision` is the top-level field grok
        // reads (NOT Claude's `hookSpecificOutput.permissionDecision`). render_response is only
        // called for ALLOWED verdicts; the Denied branch is defensive — it must stay empty (never
        // emit allow) so a stray call can't fail open.
        if verdict.is_allowed() {
            HookResponse {
                stdout: json!({ "decision": "allow" }).to_string(),
                exit_code: 0,
            }
        } else {
            HookResponse {
                stdout: String::new(),
                exit_code: 0,
            }
        }
    }

    /// Grok, like Codex/Cursor, has no hook `grant` and no hook `ask`: a hook can only DENY. A gated
    /// command must therefore be vetoed — otherwise in `bypassPermissions`/`dontAsk` mode grok would
    /// run it (the hook's `allow` only "declines to deny"). Deny protects in every mode; the escape
    /// valve is a `~/.config/safe-chains.toml` grant or a grok `--allow` rule.
    fn gated_policy(&self) -> super::GatedPolicy {
        super::GatedPolicy::Deny
    }

    fn render_deny(&self, reason: &str) -> HookResponse {
        // Both signals say deny: the top-level `decision` (honored regardless of exit code) and exit
        // 2 (grok's deny code; any OTHER non-zero fails OPEN, so it must be exactly 2).
        HookResponse {
            stdout: json!({ "decision": "deny", "reason": reason }).to_string(),
            exit_code: 2,
        }
    }
}

fn hook_file(binary: &str) -> Value {
    json!({
        "hooks": {
            "PreToolUse": [{
                "matcher": "Bash",
                "hooks": [{
                    "type": "command",
                    "command": binary,
                    "timeout": 10,
                }]
            }]
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verdict::SafetyLevel;

    fn target() -> GrokTarget {
        GrokTarget
    }

    #[test]
    fn install_no_grok_dir_skips() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(target().install(dir.path()).unwrap(), InstallOutcome::Skipped { .. }));
    }

    #[test]
    fn install_creates_dedicated_hook_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".grok")).unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::Installed { .. }));
        let path = dir.path().join(".grok/hooks/safe-chains.json");
        assert!(path.is_file(), "must write ~/.grok/hooks/safe-chains.json");
        let settings: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(has_safe_chains_hook(&settings));
        // Nested under a top-level `hooks` object with a `PreToolUse` array (grok/Codex shape), never
        // a flat top-level `PreToolUse` key.
        assert!(settings.pointer("/hooks/PreToolUse").and_then(|a| a.as_array()).is_some());
        assert!(settings.get("PreToolUse").is_none());
        assert_eq!(settings.pointer("/hooks/PreToolUse/0/matcher").and_then(|m| m.as_str()), Some("Bash"));
    }

    #[test]
    fn install_uses_subcommand_invocation() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".grok")).unwrap();
        target().install(dir.path()).unwrap();
        let contents = std::fs::read_to_string(dir.path().join(".grok/hooks/safe-chains.json")).unwrap();
        assert!(contents.contains("safe-chains hook grok"));
    }

    #[test]
    fn install_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".grok")).unwrap();
        target().install(dir.path()).unwrap();
        assert!(matches!(target().install(dir.path()).unwrap(), InstallOutcome::AlreadyConfigured { .. }));
    }

    // The verbatim PreToolUse envelope from ~/.grok/docs/user-guide/10-hooks.md — camelCase, with the
    // command nested at toolInput.command and the project root at workspaceRoot.
    const GROK_DOCS_SAMPLE: &str = r#"{
        "hookEventName": "pre_tool_use",
        "sessionId": "abc-123",
        "cwd": "/Users/me/project/sub",
        "workspaceRoot": "/Users/me/project",
        "toolName": "run_terminal_command",
        "toolInput": {"command": "npm test"},
        "timestamp": "2026-07-22T00:00:00Z"
    }"#;

    #[test]
    fn parse_input_extracts_camelcase_command_and_root() {
        let parsed = GrokHookFormat.parse_input(GROK_DOCS_SAMPLE).unwrap();
        assert_eq!(parsed.command, "npm test");
        assert_eq!(parsed.cwd.as_deref(), Some("/Users/me/project/sub"));
        assert_eq!(parsed.root.as_deref(), Some("/Users/me/project"));
    }

    #[test]
    fn parse_input_rejects_snake_case_envelope() {
        // The Claude/Codex snake_case shape must NOT parse — if it did, grok's camelCase payload would
        // silently fail to parse and fail OPEN. This is the casing tripwire.
        let snake = r#"{"tool_input": {"command": "ls"}, "workspace_root": "/p"}"#;
        assert!(GrokHookFormat.parse_input(snake).is_err());
    }

    #[test]
    fn parse_input_rejects_garbage() {
        assert!(GrokHookFormat.parse_input("not json").is_err());
        assert!(GrokHookFormat.parse_input("{}").is_err());
    }

    #[test]
    fn grok_is_a_deny_harness() {
        assert_eq!(GrokHookFormat.gated_policy(), super::super::GatedPolicy::Deny);
    }

    #[test]
    fn render_response_uses_top_level_decision_allow() {
        // Grok reads a TOP-LEVEL `decision`, not Claude's `hookSpecificOutput.permissionDecision` nor
        // Cursor's `permission`. Wiring this wrong fails open — pinned here.
        let r = GrokHookFormat.render_response(Verdict::Allowed(SafetyLevel::Inert));
        let v: Value = serde_json::from_str(&r.stdout).unwrap();
        assert_eq!(v.get("decision").and_then(|d| d.as_str()), Some("allow"));
        assert!(v.get("permissionDecision").is_none());
        assert!(v.get("permission").is_none());
        assert_eq!(r.exit_code, 0);
    }

    #[test]
    fn render_response_denied_is_empty_fail_safe() {
        // render_response is only called for ALLOWED verdicts; a defensive call with Denied must NOT
        // emit an allow (else a stray call fails open). Pinned by the cross-target contract test too.
        let r = GrokHookFormat.render_response(Verdict::Denied);
        assert_eq!(r.stdout, "");
    }

    #[test]
    fn render_deny_uses_decision_deny_and_exit_2() {
        let r = GrokHookFormat.render_deny("blocked: not on the allowlist");
        let v: Value = serde_json::from_str(&r.stdout).unwrap();
        assert_eq!(v.get("decision").and_then(|d| d.as_str()), Some("deny"));
        assert_eq!(v.get("reason").and_then(|d| d.as_str()), Some("blocked: not on the allowlist"));
        assert!(v.get("permissionDecision").is_none());
        // Exit 2 is grok's deny code; any OTHER non-zero fails OPEN, so this must be exactly 2.
        assert_eq!(r.exit_code, 2);
    }

    #[test]
    fn render_context_defaults_to_abstain() {
        // Grok's PreToolUse output has no additionalContext channel, so context injection keeps the
        // safe default: emit nothing.
        let r = GrokHookFormat.render_context("anything");
        assert_eq!(r.stdout, "");
        assert_eq!(r.exit_code, 0);
    }
}
