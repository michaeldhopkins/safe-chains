//! `safe-chains mcp`: serve the MCP tools on stdin and stdout until the client hangs up.

use std::process;

pub fn run_mcp() -> ! {
    match safe_chains::mcp::serve(&safe_chains::mcp::Spawn) {
        Ok(()) => process::exit(0),
        Err(e) => {
            eprintln!("safe-chains mcp: {e}");
            process::exit(1);
        }
    }
}
