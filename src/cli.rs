use clap::Parser;
use crate::verdict::SafetyLevel;

#[derive(Parser)]
#[command(name = "safe-chains")]
#[command(about = "Auto-allow safe bash commands in agentic coding tools")]
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

    /// Configure the hook for the named tool (default: claude). Use --auto-detect for every installed tool.
    #[arg(long)]
    pub setup: bool,

    /// Pair with --setup to select the target tool by name. See --list-tools.
    #[arg(long, value_name = "NAME")]
    pub tool: Option<String>,

    /// Pair with --setup to install for every installed tool detected on this machine.
    #[arg(long)]
    pub auto_detect: bool,

    /// Print the names of every supported integration target.
    #[arg(long)]
    pub list_tools: bool,

    /// Hook subcommand: read this tool's stdin envelope, validate the command, write the response.
    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,
}

#[derive(clap::Subcommand)]
pub enum Subcommand {
    /// Run as a runtime hook for the named tool.
    Hook {
        /// Tool to read/write the hook envelope for. See --list-tools.
        #[arg(value_name = "TOOL")]
        tool: String,
    },
}
