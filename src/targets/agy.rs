//! Antigravity CLI (`agy`) — Google's successor to the retired Gemini CLI. Hooks are configured
//! via a `hooks.json` in the customization root (`~/.gemini/config/` globally, `.agents/` per
//! project). The `run_command` PreToolUse hook decision set is `allow` / `deny` / `ask` /
//! `force_ask`. Verified LIVE against v1.1.2 CLI on 2026-07-13 (see HARNESS-BEHAVIORS.md):
//!   - `deny` → hard block, our reason shown to the model. WORKS.
//!   - `force_ask` → forces a human prompt, *ignoring* agy's Always-Allow cache. WORKS.
//!   - `allow` → does NOT suppress agy's own `request-review` confirmation in the CLI; the user is
//!     still prompted. So there is no effective *grant* on agy 1.1.2. We still emit `allow` for a
//!     safe command: it is semantically correct, harmless (agy prompts anyway by default), and
//!     future-proof if a later build honors it.
//!
//! agy's default `toolPermission=request-review` prompts for `run_command` on silence, so it HAS
//! human review — a gated command therefore *escalates* (force_ask) rather than hard-denies. We use
//! `force_ask` over plain `ask` because `ask` respects the Always-Allow cache: a user who once
//! picked "always allow commands starting with cat" would have a gated `cat /etc/hosts` auto-run.
//! Antigravity FAILS CLOSED on a missing/malformed decision. See docs/design/harness-capability-model.md.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::{GatedPolicy, HookFormat, HookInput, HookResponse, InstallOutcome, ParseError, Target};
use crate::verdict::Verdict;

pub struct AntigravityTarget;

impl Target for AntigravityTarget {
    fn name(&self) -> &'static str {
        "antigravity"
    }

    fn display_name(&self) -> &'static str {
        "Antigravity CLI (agy)"
    }

    fn detect_paths(&self, home: &Path) -> Vec<PathBuf> {
        vec![home.join(".gemini/antigravity-cli")]
    }

    fn install(&self, home: &Path) -> Result<InstallOutcome, String> {
        // Global customization root for the CLI (per agy-customizations/docs/json_configs.md).
        let dir = home.join(".gemini/config");
        if !dir.exists() {
            return Ok(InstallOutcome::Skipped {
                reason: format!("{} not found (Antigravity CLI not set up)", dir.display()),
            });
        }
        let path = dir.join("hooks.json");
        let binary = "safe-chains hook antigravity";

        let mut settings: Value = if path.exists() {
            let contents = std::fs::read_to_string(&path)
                .map_err(|e| format!("Could not read {}: {e}", path.display()))?;
            serde_json::from_str(&contents)
                .map_err(|e| format!("Could not parse {}: {e}", path.display()))?
        } else {
            Value::Object(Map::new())
        };

        if has_safe_chains_hook(&settings) {
            return Ok(InstallOutcome::AlreadyConfigured { path });
        }
        add_hook(&mut settings, binary);
        let output = serde_json::to_string_pretty(&settings).expect("serializing valid JSON");
        std::fs::write(&path, format!("{output}\n"))
            .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
        Ok(InstallOutcome::Installed { path })
    }

    fn hook_format(&self) -> Option<&dyn HookFormat> {
        Some(&AntigravityHookFormat)
    }
}

struct AntigravityHookFormat;

// Antigravity payloads are protojson (camelCase). A PreToolUse `run_command` step carries the shell
// command at `toolCall.args.CommandLine`, and the workspace at `workspacePaths`.
#[derive(Deserialize)]
struct ToolArgs {
    #[serde(rename = "CommandLine")]
    command_line: Option<String>,
}

#[derive(Deserialize)]
struct ToolCall {
    #[serde(default)]
    args: Option<ToolArgs>,
}

#[derive(Deserialize)]
struct AntigravityEnvelope {
    #[serde(rename = "toolCall")]
    tool_call: Option<ToolCall>,
    #[serde(rename = "workspacePaths", default)]
    workspace_paths: Vec<String>,
}

impl HookFormat for AntigravityHookFormat {
    fn parse_input(&self, stdin: &str) -> Result<HookInput, ParseError> {
        let env: AntigravityEnvelope =
            serde_json::from_str(stdin).map_err(|e| ParseError { message: e.to_string() })?;
        let command = env
            .tool_call
            .and_then(|t| t.args)
            .and_then(|a| a.command_line)
            .ok_or_else(|| ParseError { message: "no toolCall.args.CommandLine".into() })?;
        let cwd = env.workspace_paths.into_iter().next();
        Ok(HookInput { command, root: cwd.clone(), cwd })
    }

    fn render_response(&self, verdict: Verdict) -> HookResponse {
        // SAFE command → `allow`. agy 1.1.2 does not honor this to skip its own confirmation, but
        // it's semantically correct + future-proof, and agy fails CLOSED on a missing decision, so
        // we emit it explicitly rather than stay silent. (The flow only calls this for an allowed
        // verdict; a non-allowed one defensively emits nothing — gated goes through `render_ask`.)
        if verdict.is_allowed() {
            decision("allow", "safe-chains: all commands in the chain are allowlisted")
        } else {
            HookResponse { stdout: String::new(), exit_code: 0 }
        }
    }

    // agy has human review (default `request-review` prompts), so a gated command ESCALATES to a
    // human prompt rather than hard-denying.
    fn gated_policy(&self) -> GatedPolicy {
        GatedPolicy::Ask
    }

    fn render_ask(&self, reason: &str) -> HookResponse {
        // `force_ask`, not `ask`: `ask` respects agy's Always-Allow cache, so a coarse prefix grant
        // ("always allow commands starting with cat") would let a gated `cat /etc/hosts` auto-run.
        // `force_ask` ignores the cache and always prompts.
        decision("force_ask", reason)
    }
}

fn decision(kind: &str, reason: &str) -> HookResponse {
    let body = json!({ "decision": kind, "reason": reason });
    HookResponse { stdout: serde_json::to_string(&body).unwrap_or_default(), exit_code: 0 }
}

fn hook_entry(binary: &str) -> Value {
    json!({
        "PreToolUse": [{
            "matcher": "run_command",
            "hooks": [{ "type": "command", "command": binary }],
        }]
    })
}

fn has_safe_chains_hook(settings: &Value) -> bool {
    // Top-level keys are hook NAMES; ours is "safe-chains".
    settings
        .get("safe-chains")
        .and_then(|h| h.get("PreToolUse"))
        .and_then(Value::as_array)
        .is_some_and(|groups| {
            groups.iter().any(|g| {
                g.get("hooks").and_then(Value::as_array).is_some_and(|hs| {
                    hs.iter().any(|h| {
                        h.get("command").and_then(Value::as_str).is_some_and(|c| c.contains("safe-chains"))
                    })
                })
            })
        })
}

fn add_hook(settings: &mut Value, binary: &str) {
    if !settings.is_object() {
        *settings = json!({});
    }
    settings
        .as_object_mut()
        .expect("settings is an object")
        .insert("safe-chains".to_string(), hook_entry(binary));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verdict::SafetyLevel;

    #[test]
    fn install_skips_when_no_config_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(AntigravityTarget.install(dir.path()).unwrap(), InstallOutcome::Skipped { .. }));
    }

    #[test]
    fn install_writes_named_hook_with_run_command_matcher() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".gemini/config")).unwrap();
        assert!(matches!(AntigravityTarget.install(dir.path()).unwrap(), InstallOutcome::Installed { .. }));
        let s: Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join(".gemini/config/hooks.json")).unwrap(),
        )
        .unwrap();
        assert!(has_safe_chains_hook(&s));
        assert_eq!(
            s.pointer("/safe-chains/PreToolUse/0/matcher").and_then(Value::as_str),
            Some("run_command"),
        );
    }

    #[test]
    fn install_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".gemini/config")).unwrap();
        AntigravityTarget.install(dir.path()).unwrap();
        assert!(matches!(
            AntigravityTarget.install(dir.path()).unwrap(),
            InstallOutcome::AlreadyConfigured { .. }
        ));
    }

    #[test]
    fn install_preserves_other_named_hooks() {
        // hooks.json merges multiple named hooks; installing must not clobber a user's own.
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join(".gemini/config");
        std::fs::create_dir_all(&cfg).unwrap();
        std::fs::write(
            cfg.join("hooks.json"),
            r#"{"lint-checker":{"PostToolUse":[{"matcher":"run_command","hooks":[{"type":"command","command":"./lint.sh"}]}]}}"#,
        )
        .unwrap();
        AntigravityTarget.install(dir.path()).unwrap();
        let s: Value =
            serde_json::from_str(&std::fs::read_to_string(cfg.join("hooks.json")).unwrap()).unwrap();
        assert!(has_safe_chains_hook(&s));
        assert!(
            s.pointer("/lint-checker/PostToolUse/0/hooks/0/command").and_then(Value::as_str)
                == Some("./lint.sh"),
            "the user's own named hook must survive install",
        );
    }

    #[test]
    fn parses_command_and_workspace() {
        let input = r#"{"toolCall":{"name":"run_command","args":{"CommandLine":"cat /etc/hosts"}},"workspacePaths":["/w"]}"#;
        let parsed = AntigravityHookFormat.parse_input(input).unwrap();
        assert_eq!(parsed.command, "cat /etc/hosts");
        assert_eq!(parsed.cwd.as_deref(), Some("/w"));
    }

    #[test]
    fn safe_emits_allow_gated_asks_never_silent() {
        // Antigravity fails CLOSED on a missing decision, so both paths emit an explicit decision.
        let safe = AntigravityHookFormat.render_response(Verdict::Allowed(SafetyLevel::Inert));
        let v: Value = serde_json::from_str(&safe.stdout).unwrap();
        assert_eq!(v.get("decision").and_then(Value::as_str), Some("allow"));

        assert_eq!(AntigravityHookFormat.gated_policy(), GatedPolicy::Ask);
        let ask = AntigravityHookFormat.render_ask("please confirm");
        let v: Value = serde_json::from_str(&ask.stdout).unwrap();
        // `force_ask` (not `ask`) so agy's Always-Allow cache can't auto-run a gated command.
        assert_eq!(v.get("decision").and_then(Value::as_str), Some("force_ask"));
        assert_eq!(v.get("reason").and_then(Value::as_str), Some("please confirm"));
    }
}
