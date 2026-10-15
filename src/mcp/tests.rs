use std::cell::RefCell;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::*;

/// Records what the server would run, and answers as the CLI would.
struct Stub {
    home: tempfile::TempDir,
    cwd: tempfile::TempDir,
    ran: RefCell<Vec<Vec<String>>>,
    answer: (bool, String),
}

impl Stub {
    fn new(answer: (bool, &str)) -> Self {
        Stub {
            home: tempfile::tempdir().unwrap(),
            cwd: tempfile::tempdir().unwrap(),
            ran: RefCell::new(Vec::new()),
            answer: (answer.0, answer.1.to_string()),
        }
    }
}

impl Runner for Stub {
    fn run(&self, args: &[String], _cwd: &Path) -> (Option<i32>, String) {
        self.ran.borrow_mut().push(args.to_vec());
        (Some(i32::from(!self.answer.0)), self.answer.1.clone())
    }
    fn home(&self) -> PathBuf {
        self.home.path().to_path_buf()
    }
    fn cwd(&self) -> PathBuf {
        self.cwd.path().to_path_buf()
    }
}

fn call(runner: &Stub, name: &str, arguments: Value) -> Value {
    let line = json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": {"name": name, "arguments": arguments}}).to_string();
    let reply: Value = serde_json::from_str(&handle_line(runner, &line).unwrap()).unwrap();
    reply["result"].clone()
}

fn text(result: &Value) -> String {
    result["content"][0]["text"].as_str().unwrap_or_default().to_string()
}

#[test]
fn each_tool_runs_the_matching_cli_command() {
    let cases = [
        ("list_rules", json!({}), vec!["config", "list"]),
        ("add_command_entry", json!({"command": "mytool sync"}), vec!["config", "add", "--level", "safe-write", "--", "mytool sync"]),
        ("add_command_entry", json!({"command": "mytool", "level": "inert"}), vec!["config", "add", "--level", "inert", "--", "mytool"]),
        ("grant_path", json!({"path": "~/notes", "access": "read"}), vec!["config", "grant", "--read", "--", "~/notes"]),
        ("grant_path", json!({"path": "~/notes", "access": "read_write"}), vec!["config", "grant", "--read", "--write", "--", "~/notes"]),
        ("remove_rule", json!({"kind": "grant", "name": "~/notes"}), vec!["config", "remove", "grant", "--", "~/notes"]),
    ];
    for (tool, args, argv) in cases {
        let stub = Stub::new((true, "done"));
        let result = call(&stub, tool, args);
        assert_eq!(result["isError"], false, "{tool}: {result}");
        assert_eq!(stub.ran.borrow()[0], argv, "{tool}");
    }
}

#[test]
fn check_command_passes_the_directory_and_the_command_untouched() {
    let stub = Stub::new((false, "safe-chains: did not auto-approve this command."));
    let dir = stub.cwd.path().to_string_lossy().into_owned();
    let result = call(&stub, "check_command", json!({"command": "-rf weird", "cwd": dir}));
    assert_eq!(result["isError"], false, "a refusal is an answer, not an error");
    assert_eq!(stub.ran.borrow()[0], ["--explain", "--cwd", &dir, "--", "-rf weird"]);
    let missing = call(&stub, "check_command", json!({"command": "ls", "cwd": "/no/such/dir"}));
    assert_eq!(missing["isError"], false, "a missing directory is checked from the server's own");
    assert!(text(&missing).starts_with("(cwd /no/such/dir does not exist, so this was checked from"), "{}", text(&missing));
    assert_eq!(stub.ran.borrow().last().unwrap()[2], stub.cwd.path().to_string_lossy());
}

#[test]
fn a_failed_edit_is_an_error_result() {
    let stub = Stub::new((false, "`gh` is built into safe-chains. Nothing was changed."));
    let result = call(&stub, "add_command_entry", json!({"command": "gh pr merge"}));
    assert_eq!(result["isError"], true);
    assert!(text(&result).contains("Nothing was changed"));
}

#[test]
fn edits_are_refused_when_the_client_approves_them_unasked() {
    let stub = Stub::new((true, "done"));
    std::fs::create_dir_all(stub.home.path().join(".claude")).unwrap();
    std::fs::write(stub.home.path().join(".claude/settings.json"), r#"{"permissions":{"allow":["mcp__safe-chains__*"]}}"#).unwrap();
    for (tool, args) in [
        ("add_command_entry", json!({"command": "mytool"})),
        ("grant_path", json!({"path": "~/x", "access": "read"})),
        ("remove_rule", json!({"kind": "command", "name": "mytool"})),
    ] {
        let result = call(&stub, tool, args);
        assert_eq!(result["isError"], true, "{tool}");
        assert!(text(&result).contains("~/.claude/settings.json"), "{tool}: {}", text(&result));
    }
    assert!(stub.ran.borrow().is_empty(), "an edit ran anyway");
    let read = call(&stub, "list_rules", json!({}));
    assert_eq!(read["isError"], false, "reading is never refused");
}

#[test]
fn bad_arguments_never_reach_the_cli() {
    let stub = Stub::new((true, "done"));
    for (tool, args, says) in [
        ("grant_path", json!({"path": "~/x"}), "needs access"),
        ("grant_path", json!({"path": "~/x", "access": "all"}), "must be one of read, write, read_write"),
        ("grant_path", json!({"path": 3, "access": "read"}), "wrong type"),
        ("remove_rule", json!({"kind": "command", "name": "x", "force": true}), "does not take force"),
        ("nope", json!({}), "There is no tool"),
    ] {
        let result = call(&stub, tool, args);
        assert_eq!(result["isError"], true, "{tool}");
        assert!(text(&result).contains(says), "{tool}: {}", text(&result));
    }
    assert!(stub.ran.borrow().is_empty());
}

#[test]
fn a_long_result_is_cut_with_a_count() {
    let long: String = (0..2000).map(|i| format!("  grant ~/folder{i}: read\n")).collect();
    let stub = Stub::new((true, &long));
    let out = text(&call(&stub, "list_rules", json!({})));
    assert!(out.len() <= 8_200, "{}", out.len());
    assert!(out.contains("more lines not shown"), "{}", &out[out.len() - 200..]);
}

#[test]
fn the_protocol_answers_and_stays_quiet_on_notifications() {
    let stub = Stub::new((true, ""));
    let init: Value = serde_json::from_str(
        &handle_line(&stub, r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#).unwrap(),
    )
    .unwrap();
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert!(init["result"]["instructions"].as_str().unwrap().contains("Start with check_command"));
    assert!(handle_line(&stub, r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
    let unknown: Value = serde_json::from_str(&handle_line(&stub, r#"{"jsonrpc":"2.0","id":2,"method":"nope"}"#).unwrap()).unwrap();
    assert_eq!(unknown["error"]["code"], -32601);
    let garbage: Value = serde_json::from_str(&handle_line(&stub, "{not json").unwrap()).unwrap();
    assert_eq!(garbage["error"]["code"], -32700);
    assert!(handle_line(&stub, "   ").is_none());
}

#[test]
fn the_instructions_fit_and_lead_with_where_to_start() {
    assert!(INSTRUCTIONS.len() < 2_000, "{}", INSTRUCTIONS.len());
    assert!(INSTRUCTIONS[..500].contains("Start with check_command"));
}

#[test]
fn a_failed_read_is_an_error_and_an_unknown_revision_gets_ours() {
    let stub = Stub::new((false, "boom"));
    assert_eq!(call(&stub, "list_rules", json!({}))["isError"], true);
    let init: Value = serde_json::from_str(
        &handle_line(&stub, r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"1999-01-01"}}"#).unwrap(),
    )
    .unwrap();
    assert_eq!(init["result"]["protocolVersion"], PROTOCOL_VERSION);
}

#[test]
fn a_request_id_beyond_exact_float_range_is_echoed_as_the_client_sent_it() {
    let stub = Stub::new((true, "ok"));
    let line = r#"{"jsonrpc":"2.0","id":466666666666666666666666666666666666666666666666666666666666666666666666666666666666,"method":"tools/list"}"#;
    let sent: Value = serde_json::from_str(line).unwrap();
    let reply: Value = serde_json::from_str(&handle_line(&stub, line).expect("a request gets a reply")).unwrap();
    assert_eq!(reply["id"], sent["id"]);
}
