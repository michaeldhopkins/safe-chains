use std::io;
use std::process;

use serde::Deserialize;
use serde_json::json;

use safe_chains::{is_safe, is_safe_command};

#[derive(Deserialize)]
struct ToolInput {
    command: String,
}

#[derive(Deserialize)]
struct HookInput {
    tool_input: ToolInput,
}

enum Mode {
    ListCommands,
    Cli(String),
    ClaudeHook,
}

fn detect_mode(args: &[String]) -> Mode {
    if args.iter().any(|a| a == "--list-commands") {
        return Mode::ListCommands;
    }
    match args.iter().find(|a| !a.starts_with('-')) {
        Some(command) => Mode::Cli(command.clone()),
        None => Mode::ClaudeHook,
    }
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

    let patterns = safe_chains::settings::ApprovedPatterns::load();
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
    let args: Vec<String> = std::env::args().skip(1).collect();
    match detect_mode(&args) {
        Mode::ListCommands => print_docs(),
        Mode::Cli(command) => run_cli(&command),
        Mode::ClaudeHook => run_claude_hook(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_list_commands_mode() {
        let args = vec!["--list-commands".to_string()];
        assert!(matches!(detect_mode(&args), Mode::ListCommands));
    }

    #[test]
    fn detect_cli_mode() {
        let args = vec!["ls -la".to_string()];
        assert!(matches!(detect_mode(&args), Mode::Cli(cmd) if cmd == "ls -la"));
    }

    #[test]
    fn detect_cli_mode_skips_flags() {
        let args = vec!["--verbose".to_string(), "ls -la".to_string()];
        assert!(matches!(detect_mode(&args), Mode::Cli(cmd) if cmd == "ls -la"));
    }

    #[test]
    fn detect_claude_hook_mode() {
        let args: Vec<String> = vec![];
        assert!(matches!(detect_mode(&args), Mode::ClaudeHook));
    }

    #[test]
    fn detect_claude_hook_mode_with_only_flags() {
        let args = vec!["--verbose".to_string()];
        assert!(matches!(detect_mode(&args), Mode::ClaudeHook));
    }
}
