//! `safe-chains mcp`: the CLI's checks and config changes as MCP tools, for an agent that calls
//! tools rather than running commands. Hand-rolled newline-delimited JSON-RPC 2.0 over stdio, one
//! request at a time.
//!
//! Every tool runs the `safe-chains` binary itself (`Runner`). That keeps the two surfaces equal by
//! construction, and it is also necessary: the classifier loads the user's rules once per process,
//! so a long-lived server answering in-process would go on using rules it had just changed.
//!
//! The editing tools never approve themselves. They are annotated destructive, so a client that
//! reads annotations asks before each call, and the server refuses them outright when it can see
//! that the client is set to run them without asking (`client_approval`).

mod args;
pub mod client_approval;
mod tools;

use std::io::{BufRead, Write};

use serde_json::{Value, json};

pub use tools::{Runner, Spawn, manifest};

/// The newest protocol revision this server has been checked against. A client that asks for
/// another gets its own echoed back, as the house servers do.
pub const PROTOCOL_VERSION: &str = "2025-11-25";

/// What the client is told about the server, once, at `initialize`. Under 2,000 characters, with
/// what it is and where to start in the first 500 (`tests/mcp_rules.rs` holds both).
pub const INSTRUCTIONS: &str = "safe-chains decides which shell commands an AI agent may run without \
asking a person. This server checks commands and changes the user's rules for that, kept in \
~/.config/safe-chains.toml. Start with check_command: it says whether a command would run without \
asking and, if not, what would change that.

check_command ends with one of three fixes. Add an entry, for a command safe-chains has no entry \
for (add_command_entry). Grant a path, when a known command reached a folder outside the approved \
places (grant_path). Or nothing: a built-in rule refused an option or a protected path, no rule can \
change that, and a person approves the command each time it runs.

The three editing tools (add_command_entry, grant_path, remove_rule) change what runs without a \
person's approval. Use them only when the user asked for that change, never to get your own \
command approved. A client should ask the user before every call to them, and the server refuses \
them when it can see the client approves them automatically. They never replace a built-in \
command, and never grant a credential store or the settings files safe-chains and coding agents \
read.

Each change applies from the next command on. list_rules shows every rule and the file it came \
from.";

/// Serve until stdin closes.
pub fn serve(runner: &dyn Runner) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if let Some(reply) = handle_line(runner, &line) {
            writeln!(out, "{reply}")?;
            out.flush()?;
        }
    }
    Ok(())
}

/// One message in, at most one reply out. A notification (no `id`) gets none.
pub fn handle_line(runner: &dyn Runner, line: &str) -> Option<String> {
    if line.trim().is_empty() {
        return None;
    }
    let message: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return Some(error(Value::Null, -32700, &format!("Parse error: {e}"))),
    };
    let Some(object) = message.as_object() else {
        return Some(error(Value::Null, -32600, "Invalid request: expected a JSON object"));
    };
    let id = object.get("id").cloned()?;
    let method = object.get("method").and_then(Value::as_str).unwrap_or_default();
    let params = object.get("params").cloned().unwrap_or(Value::Null);
    let mut result = match method {
        "initialize" => initialize(&params),
        "server/discover" => {
            json!({ "supportedVersions": SUPPORTED, "capabilities": { "tools": {} }, "serverInfo": server_info(), "instructions": INSTRUCTIONS })
        }
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tools::listing(), "ttlMs": 3_600_000, "cacheScope": "private" }),
        "tools/call" => tools::call(runner, &params),
        other => return Some(error(id, -32601, &format!("Method not found: {other}"))),
    };
    // Revision 2026-07-28 requires `resultType` on every result, and Claude Code shows no tools at
    // all from a server whose `tools/list` lacks it. Earlier revisions ignore the field.
    result["resultType"] = json!("complete");
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string())
}

fn initialize(params: &Value) -> Value {
    let asked = params.get("protocolVersion").and_then(Value::as_str);
    let version = asked.filter(|v| SUPPORTED.contains(v)).unwrap_or(PROTOCOL_VERSION);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": server_info(),
        "instructions": INSTRUCTIONS,
    })
}

fn error(id: Value, code: i64, message: &str) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }).to_string()
}

#[cfg(test)]
mod tests;

/// The revisions this server answers to, for `server/discover`.
const SUPPORTED: &[&str] = &["2025-06-18", "2025-11-25", "2026-07-28"];

fn server_info() -> Value {
    json!({ "name": "safe-chains", "version": env!("CARGO_PKG_VERSION") })
}
