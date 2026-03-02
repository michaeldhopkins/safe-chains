use std::io;
use std::process;

use clap::Parser;
use serde::Deserialize;
use serde_json::json;

use safe_chains::cli::Cli;
use safe_chains::{is_safe, is_safe_command};

#[derive(Deserialize)]
struct ToolInput {
    command: String,
}

#[derive(Deserialize)]
struct HookInput {
    tool_input: ToolInput,
}

fn print_docs() {
    let docs = safe_chains::docs::all_command_docs();
    print!("{}", safe_chains::docs::render_markdown(&docs));
}

fn run_cli(command: &str) {
    process::exit(i32::from(!is_safe_command(command)));
}

fn emit_allow(reason: &str) {
    let output = json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "allow",
            "permissionDecisionReason": reason,
        }
    });
    serde_json::to_writer(io::stdout(), &output).ok();
}

fn run_claude_hook() {
    let input: HookInput = match serde_json::from_reader(io::stdin()) {
        Ok(v) => v,
        Err(_) => process::exit(0),
    };

    if is_safe_command(&input.tool_input.command) {
        emit_allow("All commands in chain are safe read-only utilities");
        return;
    }

    let patterns = safe_chains::allowlist::Matcher::load();
    if patterns.is_empty() {
        process::exit(0);
    }

    let cmd_line = safe_chains::parse::CommandLine::new(&input.tool_input.command);
    let segments = cmd_line.segments();
    let all_covered = segments.iter().all(|s| {
        is_safe(s) || (!s.has_unsafe_shell_syntax() && patterns.matches(s))
    });

    if all_covered {
        emit_allow("All commands covered by safe-chains rules or user-approved settings");
    } else {
        process::exit(0);
    }
}

fn main() {
    let cli = Cli::try_parse();

    match cli {
        Ok(cli) => {
            if cli.list_commands {
                print_docs();
            } else if let Some(command) = cli.command {
                run_cli(&command);
            } else {
                run_claude_hook();
            }
        }
        Err(e) if e.kind() == clap::error::ErrorKind::DisplayHelp
              || e.kind() == clap::error::ErrorKind::DisplayVersion =>
        {
            print!("{e}");
        }
        Err(_) => {
            run_claude_hook();
        }
    }
}
