use clap::Parser;
use crate::verdict::SafetyLevel;

#[derive(Parser)]
#[command(name = "safe-chains")]
#[command(about = "Auto-allow safe, read-only bash commands in agentic coding tools")]
#[command(version)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cli {
    /// Command string to check (omit for Claude hook mode via stdin)
    pub command: Option<String>,

    /// Safety level threshold (inert, safe-read, safe-write). Only commands at or below this level pass.
    #[arg(long, value_enum)]
    pub level: Option<SafetyLevel>,

    /// List all supported commands in Markdown format
    #[arg(long)]
    pub list_commands: bool,

    /// Generate OpenCode permission config (merges with existing opencode.json)
    #[arg(long)]
    pub opencode_config: bool,

    /// Generate mdBook command reference pages in docs/src/commands/
    #[arg(long)]
    pub generate_book: bool,

    /// Configure the Claude Code hook in ~/.claude/settings.json
    #[arg(long)]
    pub setup: bool,
}
