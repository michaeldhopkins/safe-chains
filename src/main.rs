use std::io;
use std::process;

use clap::Parser;
use serde::Deserialize;
use serde_json::json;

use safe_chains::cli::Cli;
use safe_chains::verdict::{SafetyLevel, Verdict};

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

fn print_opencode_config() {
    let patterns = safe_chains::all_opencode_patterns();
    print!("{}", safe_chains::docs::render_opencode_json(&patterns));
}

fn run_cli(command: &str, threshold: SafetyLevel) {
    let verdict = safe_chains::command_verdict(command);
    let ok = match verdict {
        Verdict::Allowed(level) => level <= threshold,
        Verdict::Denied => false,
    };
    process::exit(i32::from(!ok));
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

    let verdict = safe_chains::command_verdict(&input.tool_input.command);
    if verdict.is_allowed() {
        let reason = match verdict {
            Verdict::Allowed(SafetyLevel::SafeWrite) => "All commands in chain are safe utilities (includes file output)",
            _ => "All commands in chain are safe read-only utilities",
        };
        emit_allow(reason);
        return;
    }

    let patterns = safe_chains::allowlist::Matcher::load();
    if patterns.is_empty() {
        process::exit(0);
    }

    let Some(script) = safe_chains::cst::parse(&input.tool_input.command) else {
        process::exit(0);
    };

    let all_covered = script.0.iter().all(|stmt| {
        safe_chains::cst::is_safe_pipeline(&stmt.pipeline)
            || stmt
                .pipeline
                .commands
                .iter()
                .all(|cmd| safe_chains::allowlist::is_cmd_covered(cmd, &patterns))
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
            if cli.setup {
                safe_chains::setup::run_setup();
            } else if cli.list_commands {
                print_docs();
            } else if cli.opencode_config {
                print_opencode_config();
            } else if let Some(command) = cli.command {
                let threshold = cli.level.unwrap_or(SafetyLevel::SafeWrite);
                run_cli(&command, threshold);
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
