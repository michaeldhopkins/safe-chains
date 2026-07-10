use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Value, json};

use super::{HookFormat, HookInput, HookResponse, InstallOutcome, ParseError, Target, allow_reason};
use crate::verdict::Verdict;

pub struct CopilotTarget;

impl Target for CopilotTarget {
    fn name(&self) -> &'static str {
        "copilot"
    }

    fn display_name(&self) -> &'static str {
        "GitHub Copilot CLI"
    }

    fn detect_paths(&self, home: &Path) -> Vec<PathBuf> {
        // Copilot's canonical hook location is per-repo
        // (`<repo>/.github/hooks/*.json`), but the docs also document a
        // user-global path at `~/.github/hooks/`. Probe the user-global
        // dir for detection; per-repo install would need a project path.
        vec![home.join(".github").join("hooks")]
    }

    fn install(&self, home: &Path) -> Result<InstallOutcome, String> {
        let dir = home.join(".github").join("hooks");
        if let Err(e) = std::fs::create_dir_all(&dir) {
            return Err(format!("Could not create {}: {e}", dir.display()));
        }

        let path = dir.join("safe-chains.json");

        if path.exists() {
            let contents = std::fs::read_to_string(&path)
                .map_err(|e| format!("Could not read {}: {e}", path.display()))?;
            let settings: Value = serde_json::from_str(&contents)
                .map_err(|e| format!("Could not parse {}: {e}", path.display()))?;
            if has_safe_chains_hook(&settings) {
                return Ok(InstallOutcome::AlreadyConfigured { path });
            }
        }

        let settings = build_settings();
        let output = serde_json::to_string_pretty(&settings).expect("serializing valid JSON");
        std::fs::write(&path, format!("{output}\n"))
            .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
        Ok(InstallOutcome::Installed { path })
    }

    fn hook_format(&self) -> Option<&dyn HookFormat> {
        Some(&CopilotHookFormat)
    }
}

struct CopilotHookFormat;

#[derive(Deserialize)]
struct CopilotHookEnvelope {
    #[serde(default)]
    #[serde(rename = "toolName")]
    tool_name: Option<String>,
    #[serde(default)]
    #[serde(rename = "toolArgs")]
    tool_args: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
}

#[derive(Deserialize)]
struct CopilotToolArgs {
    #[serde(default)]
    command: Option<String>,
}

impl HookFormat for CopilotHookFormat {
    fn parse_input(&self, stdin: &str) -> Result<HookInput, ParseError> {
        // Copilot's quirk: toolArgs is a JSON-encoded *string*, not a
        // nested object. We must parse the outer envelope, then parse
        // the toolArgs string a second time to recover {command}.
        let envelope: CopilotHookEnvelope =
            serde_json::from_str(stdin).map_err(|e| ParseError {
                message: e.to_string(),
            })?;

        // The hook fires for every tool by default — Copilot's config
        // has no matcher. Self-filter to the bash tool here; for other
        // tools, return Err so the runtime exits silently and Copilot
        // falls back to its own permission rules.
        let is_bash_tool = envelope
            .tool_name
            .as_deref()
            .is_some_and(|n| n == "bash");
        if !is_bash_tool {
            return Err(ParseError {
                message: format!(
                    "not a bash tool: {:?}",
                    envelope.tool_name.as_deref().unwrap_or("<missing>")
                ),
            });
        }

        let raw_args = envelope.tool_args.unwrap_or_default();
        let inner: CopilotToolArgs =
            serde_json::from_str(&raw_args).map_err(|e| ParseError {
                message: format!("toolArgs not a parseable JSON string: {e}"),
            })?;
        Ok(HookInput {
            command: inner.command.unwrap_or_default(),
            cwd: envelope.cwd,
            root: None, // copilot sends cwd but no documented project root
        })
    }

    fn render_response(&self, verdict: Verdict) -> HookResponse {
        // Copilot's effective decision space is *only* "deny" right
        // now (per docs: "only `deny` is currently processed"). For
        // allowed commands, the right answer is "no opinion" — empty
        // body, exit 0 — letting Copilot's own permission system fall
        // through to its allow-by-default for safe tools.
        //
        // We DO emit an allow-shaped envelope anyway so future Copilot
        // releases that honor allow/ask see our reasoning. Today it's
        // a no-op; tomorrow it's free upgrade.
        if verdict.is_allowed() {
            let reason = allow_reason(verdict);
            let body = json!({
                "permissionDecision": "allow",
                "permissionDecisionReason": reason,
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
}

fn build_settings() -> Value {
    // Copilot's config: flat object with `version` and `hooks.preToolUse[]`.
    // No `matcher` in entries — script self-filters on `toolName`. Field
    // name is `bash` (the script path), NOT `command`.
    let resolved = std::env::current_exe()
        .ok()
        .and_then(|p| p.canonicalize().ok())
        .map(|p| format!("{} hook copilot", p.display()))
        .unwrap_or_else(|| "safe-chains hook copilot".to_string());
    json!({
        "version": 1,
        "hooks": {
            "preToolUse": [
                {
                    "type": "command",
                    "bash": resolved,
                    "comment": "safe-chains: validate every Bash tool call before it runs.",
                    "timeoutSec": 60,
                }
            ]
        }
    })
}

fn has_safe_chains_hook(settings: &Value) -> bool {
    settings
        .pointer("/hooks/preToolUse")
        .and_then(|arr| arr.as_array())
        .is_some_and(|entries| {
            entries.iter().any(|entry| {
                entry
                    .get("bash")
                    .and_then(|c| c.as_str())
                    .is_some_and(|cmd| cmd.contains("safe-chains"))
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verdict::SafetyLevel;

    fn target() -> CopilotTarget {
        CopilotTarget
    }

    /// Verbatim shape from docs.github.com/en/copilot/reference/
    /// hooks-configuration. The `toolArgs` field is a JSON-encoded
    /// STRING, not a nested object — must be parsed twice.
    const COPILOT_DOCS_SAMPLE: &str = r#"{
        "timestamp": 1704614600000,
        "cwd": "/path/to/project",
        "toolName": "bash",
        "toolArgs": "{\"command\":\"ls -la\",\"description\":\"list files\"}"
    }"#;

    #[test]
    fn install_creates_hooks_file() {
        let dir = tempfile::tempdir().unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::Installed { .. }));
        let path = dir.path().join(".github/hooks/safe-chains.json");
        assert!(path.exists());
        let contents = std::fs::read_to_string(&path).unwrap();
        let settings: Value = serde_json::from_str(&contents).unwrap();
        assert!(has_safe_chains_hook(&settings));
    }

    #[test]
    fn install_uses_bash_field_not_command() {
        // Copilot's script-path field is `bash`, not `command`. Wiring
        // this to `command` would silently mis-configure the hook.
        let dir = tempfile::tempdir().unwrap();
        target().install(dir.path()).unwrap();
        let contents = std::fs::read_to_string(
            dir.path().join(".github/hooks/safe-chains.json"),
        )
        .unwrap();
        let settings: Value = serde_json::from_str(&contents).unwrap();
        let entry = settings.pointer("/hooks/preToolUse/0").unwrap();
        assert!(entry.get("bash").is_some(), "must use `bash` key");
        assert!(entry.get("command").is_none(), "must NOT use `command` key");
    }

    #[test]
    fn install_uses_subcommand_invocation() {
        let dir = tempfile::tempdir().unwrap();
        target().install(dir.path()).unwrap();
        let contents = std::fs::read_to_string(
            dir.path().join(".github/hooks/safe-chains.json"),
        )
        .unwrap();
        assert!(contents.contains("hook copilot"));
    }

    #[test]
    fn install_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        target().install(dir.path()).unwrap();
        let outcome = target().install(dir.path()).unwrap();
        assert!(matches!(outcome, InstallOutcome::AlreadyConfigured { .. }));
    }

    #[test]
    fn parse_input_double_decodes_tool_args() {
        // The headline quirk: outer JSON, then `toolArgs` is itself a
        // JSON-encoded string that must be parsed again to recover the
        // bash command. Tested with the verbatim docs payload.
        let parsed = CopilotHookFormat.parse_input(COPILOT_DOCS_SAMPLE).unwrap();
        assert_eq!(parsed.command, "ls -la");
        assert_eq!(parsed.cwd.as_deref(), Some("/path/to/project"));
    }

    #[test]
    fn parse_input_skips_non_bash_tools() {
        // No matcher in Copilot's config — every tool dispatches
        // through. Self-filter to "bash"; for others, return Err so
        // the runtime exits silently and Copilot's own perms apply.
        let stdin = r#"{
            "timestamp": 1,
            "cwd": "/p",
            "toolName": "edit",
            "toolArgs": "{\"path\":\"x\"}"
        }"#;
        assert!(CopilotHookFormat.parse_input(stdin).is_err());
    }

    #[test]
    fn parse_input_rejects_garbage() {
        assert!(CopilotHookFormat.parse_input("not json").is_err());
    }

    #[test]
    fn parse_input_rejects_unparseable_tool_args() {
        let stdin = r#"{"toolName": "bash", "toolArgs": "not-json"}"#;
        let result = CopilotHookFormat.parse_input(stdin);
        assert!(result.is_err());
    }

    #[test]
    fn render_response_emits_flat_object_no_wrapper() {
        // Copilot uses a FLAT response object — no
        // hookSpecificOutput wrapper key, unlike Claude/Codex/Qwen/
        // Droid. Wrapping would be silently rejected.
        let r = CopilotHookFormat.render_response(Verdict::Allowed(SafetyLevel::Inert));
        let v: Value = serde_json::from_str(&r.stdout).unwrap();
        assert_eq!(
            v.get("permissionDecision").and_then(|s| s.as_str()),
            Some("allow"),
        );
        assert!(
            v.get("hookSpecificOutput").is_none(),
            "must NOT wrap in hookSpecificOutput",
        );
    }

    #[test]
    fn render_response_includes_reason() {
        let r = CopilotHookFormat.render_response(Verdict::Allowed(SafetyLevel::SafeWrite));
        let v: Value = serde_json::from_str(&r.stdout).unwrap();
        assert!(
            v.get("permissionDecisionReason")
                .and_then(|s| s.as_str())
                .is_some()
        );
    }

    #[test]
    fn render_response_deny_emits_empty_body() {
        let r = CopilotHookFormat.render_response(Verdict::Denied);
        assert_eq!(r.stdout, "");
    }
}
