use std::path::{Path, PathBuf};

use crate::verdict::{SafetyLevel, Verdict};

pub mod agy;
pub mod claude;
pub mod codex;
pub mod copilot;
pub mod cursor;
pub mod droid;
pub mod gemini;
pub mod grok;
pub mod opencode;
pub mod qwen;

pub trait Target: Send + Sync {
    fn name(&self) -> &'static str;

    /// The harness's SHELL tool — the only tool this hook should decide about. Droid's is
    /// `Execute`, Gemini's `run_shell_command`; most are `Bash`. Defaults to `Bash`, the common
    /// case.
    fn shell_tool_name(&self) -> &'static str {
        "Bash"
    }

    /// A sample envelope this target's `parse_input` accepts, naming `tool`, or `None` when the
    /// harness's envelope carries NO tool identifier.
    ///
    /// `None` is a researched claim, not a default: it says the envelope has no field naming the
    /// tool, so the hook cannot tell a shell call from any other and must rely on its configured
    /// matcher alone. `Some` obliges the target to abstain on a foreign tool —
    /// `no_target_decides_on_a_foreign_tool` holds it to that in both directions.
    ///
    /// Test-only; each target knows its own envelope shape, and a generic one cannot stand in for
    /// nine different schemas (Copilot nests `toolArgs` as a JSON STRING, Antigravity uses
    /// `toolCall.args.commandLine`, Grok is camelCase).
    #[cfg(test)]
    fn sample_envelope(&self, _tool: &str, _command: &str) -> Option<String> {
        None
    }

    fn display_name(&self) -> &'static str;

    fn detect_paths(&self, home: &Path) -> Vec<PathBuf>;

    fn install(&self, home: &Path) -> Result<InstallOutcome, String>;

    fn hook_format(&self) -> Option<&dyn HookFormat> {
        None
    }
}

pub trait HookFormat: Send + Sync {
    fn parse_input(&self, stdin: &str) -> Result<HookInput, ParseError>;

    fn render_response(&self, verdict: Verdict) -> HookResponse;

    /// The JSON pointer this harness reads its decision from.
    ///
    /// Deliberately has NO default: getting the field wrong fails SILENTLY — the harness ignores
    /// the unknown key and falls back to its own permissions, so a mis-wired target still lets
    /// commands run and looks like it works while never deciding anything. Requiring the
    /// declaration means a new target cannot be added without stating its contract, and
    /// `every_target_emits_its_decision_at_the_declared_field` checks every emission against it —
    /// including that the decision does NOT appear at another harness's pointer, which is what a
    /// copy-pasted target looks like.
    ///
    /// Note the leaf name alone is not the contract: Claude nests
    /// `/hookSpecificOutput/permissionDecision` while Copilot uses a flat `/permissionDecision`.
    fn decision_pointer(&self) -> &'static str;

    /// Surface explanatory context to the model on a non-approval *without*
    /// changing the permission decision (the command still flows through the
    /// tool's normal approval path, and the user's own allowlist still applies).
    ///
    /// The default abstains silently — same as today's empty deny body. A target
    /// overrides this only when its hook schema has a verified field for
    /// injecting model-visible context without a permission decision.
    fn render_context(&self, _context: &str) -> HookResponse {
        HookResponse {
            stdout: String::new(),
            exit_code: 0,
        }
    }

    /// How this harness's hook must handle a GATED command (one safe-chains does not auto-approve),
    /// derived from its capabilities (`docs/design/harness-capability-model.md`):
    /// - `Defer` — stay silent; the harness's own per-command human review is the check (Claude).
    /// - `Deny` — veto it; the harness has no human review and no escalate (Codex).
    /// - `Ask` — escalate to an in-the-moment human prompt (Antigravity's `ask`).
    fn gated_policy(&self) -> GatedPolicy {
        GatedPolicy::Defer
    }

    /// The hook output that VETOES a gated command, for a `Deny` harness. Default abstains (so a
    /// stray call can't fail open). The shape must be exactly what the harness supports, or a
    /// harness that "continues on malformed output" (e.g. Codex) fails open.
    fn render_deny(&self, _reason: &str) -> HookResponse {
        HookResponse {
            stdout: String::new(),
            exit_code: 0,
        }
    }

    /// The hook output that ESCALATES a gated command to a human prompt, for an `Ask` harness.
    /// Default abstains. (Antigravity fails CLOSED on a malformed/absent decision, so an Ask target
    /// must always emit a valid decision.)
    fn render_ask(&self, _reason: &str) -> HookResponse {
        HookResponse {
            stdout: String::new(),
            exit_code: 0,
        }
    }
}

/// How a harness's hook handles a gated command — see `HookFormat::gated_policy`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GatedPolicy {
    Defer,
    Deny,
    Ask,
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

pub struct HookInput {
    pub command: String,
    pub cwd: Option<String>,
    /// The project root, when the harness supplies one (HP-19) — a `*_PROJECT_DIR` env var
    /// for most, `workspace_roots` in the payload for cursor. Absent for codex/copilot.
    pub root: Option<String>,
    /// The harness's session/conversation id, when it supplies one (`session_id` for
    /// Claude/Gemini/Qwen/Droid, `sessionId` for grok, `conversation_id` for cursor). It comes from
    /// the harness's own envelope, so the agent cannot forge it — which is what makes it usable as
    /// the anchor for recognizing the session's scratchpad (see `pathctx::session_scratchpad`).
    pub session_id: Option<String>,
}

/// May a GRANT be emitted for this command?
///
/// A blank command classifies as `Allowed(Inert)` — an empty script really is inert — but rendering
/// that as `permissionDecision: "allow"` asserts "every command in this chain is safe" about ZERO
/// commands, and on the harnesses whose allow is authoritative it replaces the user's prompt.
///
/// The check lives HERE, next to the decision contract, rather than in the binary. It was in
/// `main.rs` first: the shipped hook was safe, but `render_response` is public and knew nothing
/// about blankness, so any second caller reintroduced the bug — and the integration guard passed
/// only because it drives the binary. The `hook_envelope` fuzz target found exactly that by calling
/// the format directly.
pub fn may_grant(command: &str, verdict: crate::Verdict) -> bool {
    verdict.is_allowed() && !command.trim().is_empty()
}

/// The decision for one parsed envelope: the response to emit, or `None` to abstain.
///
/// The single seam every caller goes through, so the blank-command rule cannot be bypassed by
/// reaching for `render_response` directly.
pub fn respond(format: &dyn HookFormat, command: &str, verdict: crate::Verdict) -> Option<HookResponse> {
    may_grant(command, verdict).then(|| format.render_response(verdict))
}

/// Append `entry` to `settings[outer][event]`, creating the path when absent.
///
/// Refuses — rather than overwriting — when an existing key has the wrong TYPE. Four targets wrote
/// this by hand as `entry(k).or_insert_with(…).as_object_mut().expect("created above as an
/// object")`, and the message says why it looked safe: it reads as if the key had just been
/// created. `or_insert_with` returns the EXISTING value, so a settings file carrying
/// `"hooks": "something"` made `--setup` PANIC — and under `--auto-detect` that aborts the whole
/// run, so every target after it goes uninstalled.
///
/// Erroring beats replacing. The value is the user's, an unreadable one usually means a
/// hand-edit or a schema we don't know, and silently rewriting config we did not understand is
/// not ours to do. `install` writes only on `Ok`, so the file is left untouched either way.
pub(crate) fn append_hook_entry(
    settings: &mut serde_json::Value,
    outer: &str,
    event: &str,
    entry: serde_json::Value,
) -> Result<(), String> {
    use serde_json::json;
    if !settings.is_object() {
        *settings = json!({});
    }
    let Some(obj) = settings.as_object_mut() else {
        unreachable!("settings was just set to an object");
    };
    let hooks = obj.entry(outer).or_insert_with(|| json!({}));
    let Some(hooks) = hooks.as_object_mut() else {
        return Err(format!(
            "`{outer}` is {}, expected an object — leaving the file unchanged",
            json_kind(&obj[outer])
        ));
    };
    let slot = hooks.entry(event).or_insert_with(|| json!([]));
    if !slot.is_array() {
        return Err(format!(
            "`{outer}.{event}` is {}, expected an array — leaving the file unchanged",
            json_kind(slot)
        ));
    }
    let Some(arr) = slot.as_array_mut() else {
        unreachable!("just checked it is an array");
    };
    arr.push(entry);
    Ok(())
}

fn json_kind(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "a boolean",
        serde_json::Value::Number(_) => "a number",
        serde_json::Value::String(_) => "a string",
        serde_json::Value::Array(_) => "an array",
        serde_json::Value::Object(_) => "an object",
    }
}

/// Read a harness project-root env var from the hook process environment (set by the
/// harness, not the agent's shell — see HARNESS-BEHAVIORS.md). Empty → `None`.
pub(crate) fn env_root(var: &str) -> Option<String> {
    std::env::var(var).ok().filter(|s| !s.is_empty())
}

pub struct HookResponse {
    pub stdout: String,
    pub exit_code: i32,
}

pub enum InstallOutcome {
    Installed { path: PathBuf },
    AlreadyConfigured { path: PathBuf },
    Skipped { reason: String },
}

impl InstallOutcome {
    pub fn message(&self, target_display: &str) -> String {
        match self {
            InstallOutcome::Installed { path } => {
                format!("{target_display}: installed → {}", path.display())
            }
            InstallOutcome::AlreadyConfigured { path } => {
                format!("{target_display}: already configured at {}", path.display())
            }
            InstallOutcome::Skipped { reason } => {
                format!("{target_display}: skipped — {reason}")
            }
        }
    }
}

pub fn registry() -> Vec<Box<dyn Target>> {
    vec![
        Box::new(claude::ClaudeTarget),
        Box::new(codex::CodexTarget),
        Box::new(agy::AntigravityTarget),
        Box::new(cursor::CursorTarget),
        Box::new(gemini::GeminiTarget),
        Box::new(grok::GrokTarget),
        Box::new(copilot::CopilotTarget),
        Box::new(qwen::QwenTarget),
        Box::new(droid::DroidTarget),
        Box::new(opencode::OpenCodeTarget),
    ]
}

pub fn find(name: &str) -> Option<Box<dyn Target>> {
    registry().into_iter().find(|t| t.name() == name)
}

pub fn detect_installed(home: &Path) -> Vec<Box<dyn Target>> {
    registry()
        .into_iter()
        .filter(|t| t.detect_paths(home).iter().any(|p| p.exists()))
        .collect()
}

pub fn allow_reason(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Allowed(SafetyLevel::SafeWrite) => {
            "All commands in chain are safe utilities (includes file writes)"
        }
        Verdict::Allowed(SafetyLevel::SafeRead) => {
            "All commands in chain are safe utilities (includes code execution)"
        }
        _ => "All commands in chain are safe utilities",
    }
}

#[cfg(test)]
mod append_hook_entry_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn creates_the_path_when_absent() {
        let mut s = json!({});
        append_hook_entry(&mut s, "hooks", "PreToolUse", json!({"matcher": "Bash"})).unwrap();
        assert_eq!(s["hooks"]["PreToolUse"][0]["matcher"], "Bash");
    }

    #[test]
    fn appends_beside_an_existing_entry() {
        let mut s = json!({"hooks": {"PreToolUse": [{"matcher": "Other"}]}});
        append_hook_entry(&mut s, "hooks", "PreToolUse", json!({"matcher": "Bash"})).unwrap();
        let arr = s["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(arr.len(), 2, "the user's existing hook must survive");
        assert_eq!(arr[0]["matcher"], "Other");
    }

    /// The panic this replaced: `entry(k).or_insert_with(…).as_object_mut().expect(…)` reads as if
    /// the key was just created, but `or_insert_with` returns the EXISTING value. A settings file
    /// with `"hooks": "x"` crashed `--setup` — and under `--auto-detect` that aborted the whole run.
    #[test]
    fn refuses_a_wrong_typed_outer_key_without_panicking() {
        for wrong in [json!("a string"), json!([1, 2]), json!(7), json!(null)] {
            let mut s = json!({ "hooks": wrong });
            let before = s.clone();
            let err = append_hook_entry(&mut s, "hooks", "PreToolUse", json!({})).unwrap_err();
            assert!(err.contains("expected an object"), "unhelpful error: {err}");
            assert_eq!(s, before, "the user's value must be left alone, not replaced");
        }
    }

    #[test]
    fn refuses_a_wrong_typed_event_key_without_panicking() {
        let mut s = json!({"hooks": {"PreToolUse": "a string"}});
        let before = s.clone();
        let err = append_hook_entry(&mut s, "hooks", "PreToolUse", json!({})).unwrap_err();
        assert!(err.contains("expected an array"), "unhelpful error: {err}");
        assert_eq!(s, before, "the user's value must be left alone, not replaced");
    }

    #[test]
    fn replaces_a_non_object_root() {
        // A file whose ROOT is not an object carries nothing to preserve.
        let mut s = json!("garbage");
        append_hook_entry(&mut s, "hooks", "PreToolUse", json!({"matcher": "Bash"})).unwrap();
        assert_eq!(s["hooks"]["PreToolUse"][0]["matcher"], "Bash");
    }
}

#[cfg(test)]
mod tool_filter_tests {
    use super::*;

    /// No target decides on a tool that is not its shell tool.
    ///
    /// The hook is wired with a matcher (`Bash`, `Execute`, `run_shell_command`), so normally only
    /// shell calls arrive. But a matcher is configuration: it can be hand-edited, and grok is
    /// documented to auto-load `~/.claude/settings.json`, which hands Claude's hook a foreign
    /// envelope. Deciding on a `Read`/`Write`/`Edit` call grants or vetoes a tool whose semantics
    /// were never analysed — and for the ALLOW-capable targets that is a grant, issued on the
    /// strength of a `command` field the tool does not even have. Four targets did exactly that.
    ///
    /// Driven by each target's OWN `sample_envelope`, because nine harnesses have nine schemas and
    /// a generic probe silently fails to parse (which looks like a pass). A target whose envelope
    /// carries no tool identifier returns `None` and is exempt — a researched claim, recorded per
    /// target, not a default.
    #[test]
    fn no_target_decides_on_a_foreign_tool() {
        let mut failures = Vec::new();
        let mut checked = 0usize;
        for target in registry() {
            let Some(fmt) = target.hook_format() else { continue };
            let Some(shell) = target.sample_envelope(target.shell_tool_name(), "ls") else {
                continue; // envelope carries no tool identifier — cannot self-filter
            };
            let name = target.name();
            // The shell tool must still parse, or "reject everything" would satisfy the negative
            // half and look like a working filter.
            if let Err(e) = fmt.parse_input(&shell) {
                failures.push(format!(
                    "{name}: rejected its own shell tool `{}`: {}",
                    target.shell_tool_name(),
                    e.message
                ));
            }
            for foreign in ["Read", "Write", "Edit", "WebFetch"] {
                let Some(env) = target.sample_envelope(foreign, "rm -rf /") else { continue };
                checked += 1;
                if fmt.parse_input(&env).is_ok() {
                    failures.push(format!(
                        "{name}: parsed a `{foreign}` envelope instead of abstaining"
                    ));
                }
            }
        }
        assert!(checked > 0, "no target was probed — the guard is vacuous");
        assert!(failures.is_empty(), "foreign-tool decisions:\n{}", failures.join("\n"));
    }
}
