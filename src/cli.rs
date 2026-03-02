use clap::Parser;

#[derive(Parser)]
#[command(name = "safe-chains")]
#[command(about = "Auto-allow safe, read-only bash commands in agentic coding tools")]
#[command(version)]
pub struct Cli {
    /// Command string to check (omit for Claude hook mode via stdin)
    pub command: Option<String>,

    /// List all supported commands in Markdown format
    #[arg(long)]
    pub list_commands: bool,
}
