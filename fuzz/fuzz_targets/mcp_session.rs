#![no_main]

//! MCP SESSION target: whatever a client sends, the server answers by the protocol.
//!
//! Each input line is one message. Properties:
//!
//! 1. No panic, on any bytes.
//! 2. A message with an `id` gets exactly one reply carrying that id, and the reply has exactly one
//!    of `result` or `error`. A notification gets none.
//! 3. A tool call never reaches the binary with arguments its schema refused: the stand-in runner
//!    records every run, and a refused call leaves it untouched.

use std::cell::Cell;
use std::path::{Path, PathBuf};

use libfuzzer_sys::fuzz_target;
use serde_json::Value;

struct Stub {
    runs: Cell<usize>,
}

impl safe_chains::mcp::Runner for Stub {
    fn run(&self, _args: &[String], _cwd: &Path) -> (Option<i32>, String) {
        self.runs.set(self.runs.get() + 1);
        (Some(0), "ok".to_string())
    }
    fn home(&self) -> PathBuf {
        PathBuf::from("/nonexistent-fuzz-home")
    }
    fn cwd(&self) -> PathBuf {
        PathBuf::from("/")
    }
}

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let stub = Stub { runs: Cell::new(0) };
    for line in text.lines() {
        let before = stub.runs.get();
        let reply = safe_chains::mcp::handle_line(&stub, line);
        let message: Option<Value> = serde_json::from_str(line).ok();
        let id = message.as_ref().and_then(|m| m.as_object()).and_then(|o| o.get("id").cloned());
        match (&message, &reply) {
            (Some(Value::Object(_)), None) => assert!(id.is_none(), "a request with an id got no reply: {line}"),
            (_, Some(reply)) => {
                let reply: Value = serde_json::from_str(reply).expect("every reply is JSON");
                assert_eq!(reply["jsonrpc"], "2.0");
                assert!(reply.get("result").is_some() != reply.get("error").is_some(), "exactly one of result or error: {reply}");
                if let Some(id) = &id {
                    assert_eq!(&reply["id"], id, "the reply carries the request's id");
                }
                let refused = reply["result"]["isError"] == true;
                if refused {
                    assert_eq!(stub.runs.get(), before, "a refused call still ran the binary: {line}");
                }
            }
            (_, None) => assert!(line.trim().is_empty() || message.is_some(), "a malformed line got no reply: {line:?}"),
        }
    }
});
