use clap::Parser;

const EXAMPLES: &str = "\
EXAMPLES:
  # Check whether a command would auto-approve (exit 0 = allowed):
  safe-chains \"git status\"

  # See why a command does or doesn't approve, and what would change that:
  safe-chains --explain \"grep foo . && ./deploy.sh\"

  # For a single command, --explain also shows the behavior it resolved to and
  # says in plain words what it does and which limit refused it:
  safe-chains --explain \"aws dynamodb put-item --table-name t --item {}\"

  # List the rules safe-chains reads from your files, and where each came from:
  safe-chains config list

  # Change them. safe-chains never approves these itself, so a person does:
  safe-chains config add \"mytool sync --dry-run\"
  safe-chains config grant ~/notes --write
  safe-chains config remove grant ~/notes

  # Show the project .safe-chains.toml entry for a command safe-chains doesn't
  # recognize, and the pin that trusts it. Writes nothing:
  safe-chains --suggest \"mytool sync --dry-run\"
";

#[derive(Parser)]
#[command(name = "safe-chains")]
#[command(about = "Auto-allow safe bash commands in agentic coding tools")]
#[command(version, disable_version_flag = true)]
#[command(after_help = EXAMPLES)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cli {
    /// Command string to check (omit for Claude hook mode via stdin)
    pub command: Option<String>,

    /// Print version information.
    #[arg(short = 'v', short_alias = 'V', long, action = clap::ArgAction::Version)]
    pub version: Option<bool>,

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

    /// Check the command as a hook does when the tool does not say which folder it runs in, with
    /// writes approved as far as LEVEL allows: reads, developer or workspace. --root is the
    /// workspace; --cwd is ignored.
    #[arg(long, value_name = "LEVEL", value_parser = ["reads", "developer", "workspace"])]
    pub unknown_folder: Option<String>,

    /// The harness session id (as a hook would pass), used to recognize this session's scratchpad
    /// under a temp root as a trusted working area rather than anonymous `/tmp`.
    #[arg(long, value_name = "ID")]
    pub session_id: Option<String>,

    /// Print a per-segment breakdown of why a command would or would not auto-approve. For a
    /// single command it also prints the behavior it resolved to and, when it does not approve,
    /// says which limit refused it.
    #[arg(long)]
    pub explain: bool,

    /// Show how to support a command safe-chains doesn't recognize yet. Pass the command as the
    /// argument: `safe-chains --suggest "<command>"`. It prints the `.safe-chains.toml` entry for
    /// the unrecognized command, the path that file belongs at, and the `[[trusted]]` pin to add to
    /// ~/.config/safe-chains.toml so it takes effect. Writes nothing — copy the parts you want.
    #[arg(long)]
    pub suggest: bool,

    /// Record every command that did NOT auto-approve — with the directory it ran in and why it was
    /// refused — as JSON Lines in ~/.local/state/safe-chains/log.jsonl. Off unless given. The file
    /// holds commands verbatim, credentials included, and is created owner-only.
    #[arg(long)]
    pub log: bool,

    /// Like --log, but records approvals too. Writes an entry per command rather than per refusal.
    #[arg(long)]
    pub log_everything: bool,

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

        /// How far to approve writes when this tool does not say which folder the command runs
        /// in: reads, developer or workspace. Overrides `[unknown_folder] writes` in
        /// ~/.config/safe-chains.toml. Default: developer.
        #[arg(long, value_name = "LEVEL", value_parser = ["reads", "developer", "workspace"])]
        unknown_folder: Option<String>,
    },
    /// List or change the rules in ~/.config/safe-chains.toml. safe-chains never approves `add`,
    /// `remove` or `grant` by itself, so a person approves each change.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Serve the same checks and config changes over MCP on stdin and stdout, for agents that call
    /// tools rather than run commands.
    Mcp,
}

#[derive(clap::Subcommand)]
pub enum ConfigAction {
    /// List every rule safe-chains reads from a file, under the file it came from.
    List,
    /// Add an entry for a command safe-chains has no entry for, approving it used the way it is
    /// here: the same subcommand and flags, and up to as many other arguments.
    Add {
        /// The command, as one quoted string or as separate words.
        #[arg(required = true, num_args = 1.., trailing_var_arg = true, allow_hyphen_values = true, value_name = "COMMAND")]
        command: Vec<String>,
        /// How much the command may do: inert, safe-read or safe-write.
        #[arg(long, default_value = "safe-write")]
        level: String,
    },
    /// Remove a command entry or a grant from ~/.config/safe-chains.toml.
    Remove {
        /// What to remove.
        #[arg(value_enum)]
        kind: RuleKind,
        /// The command's name, or the grant's path.
        #[arg(value_name = "NAME_OR_PATH")]
        key: String,
    },
    /// Let commands read or write in a folder outside the places safe-chains approves by default.
    Grant {
        /// The folder or file. A relative path is taken from the current directory.
        path: String,
        /// Approve reading there.
        #[arg(long)]
        read: bool,
        /// Approve writing there.
        #[arg(long)]
        write: bool,
    },
}

/// The two kinds of rule `config remove` can remove.
#[derive(clap::ValueEnum, Clone, Copy)]
pub enum RuleKind {
    Command,
    Grant,
}
