use clap::Parser;

#[derive(Parser)]
#[command(name = "safe-chains")]
#[command(about = "Auto-allow safe bash commands in agentic coding tools")]
#[command(version)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cli {
    /// Command string to check (omit for Claude hook mode via stdin)
    pub command: Option<String>,

    /// Safety level threshold; only commands at or below it auto-approve. Levels, locked → open:
    /// paranoid, reader, editor, developer, local-admin, network-admin, yolo. The legacy names
    /// inert / safe-read / safe-write still work (mapped to paranoid / reader / developer, with a
    /// notice). Default: developer.
    #[arg(long)]
    pub level: Option<String>,

    /// Working directory to resolve relative paths against (as a harness hook would pass).
    /// Pair with --root so e.g. `cd`-relative writes classify against the real directory.
    #[arg(long)]
    pub cwd: Option<String>,

    /// Project root, so a relative path under it is worktree-local and one outside it (the
    /// cwd having escaped the project) is scored as its real absolute target.
    #[arg(long)]
    pub root: Option<String>,

    /// Print a per-segment breakdown of why a command would or would not auto-approve.
    #[arg(long)]
    pub explain: bool,

    /// For a command safe-chains doesn't recognize, generate the `.safe-chains.toml` needed to
    /// support it, and print the `[[trusted]]` pin to add to ~/.config/safe-chains.toml.
    #[arg(long)]
    pub suggest: bool,

    /// List all supported commands in Markdown format
    #[arg(long)]
    pub list_commands: bool,

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
