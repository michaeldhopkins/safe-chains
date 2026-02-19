use std::io;
use std::process;

use serde::Deserialize;
use serde_json::json;

use claude_safe_chains::is_safe_command;

#[derive(Deserialize)]
struct ToolInput {
    command: String,
}

#[derive(Deserialize)]
struct HookInput {
    tool_input: ToolInput,
}

fn main() {
    if std::env::args().any(|a| a == "--list-commands") {
        let docs = claude_safe_chains::docs::all_command_docs();
        print!("{}", claude_safe_chains::docs::render_markdown(&docs));
        return;
    }

    let input: HookInput = match serde_json::from_reader(io::stdin()) {
        Ok(v) => v,
        Err(_) => process::exit(0),
    };

    if !is_safe_command(&input.tool_input.command) {
        process::exit(0);
    }

    let output = json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "allow",
            "permissionDecisionReason": "All commands in chain are safe read-only utilities",
        }
    });

    serde_json::to_writer(io::stdout(), &output).ok();
}
