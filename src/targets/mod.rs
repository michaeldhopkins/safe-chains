use std::path::{Path, PathBuf};

use crate::verdict::{SafetyLevel, Verdict};

pub mod claude;
pub mod codex;
pub mod copilot;
pub mod cursor;
pub mod droid;
pub mod gemini;
pub mod opencode;
pub mod qwen;

pub trait Target: Send + Sync {
    fn name(&self) -> &'static str;

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
        Box::new(cursor::CursorTarget),
        Box::new(gemini::GeminiTarget),
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
