//! The MCP ratchet's rules, checked on what the spawned server actually lists and answers.
//!
//! The rules and their reasons are the house MCP ratchet. Each one here is something a weak model
//! got wrong against another server: a description cut off, an untyped argument retried in a loop,
//! a silently dropped argument, a result too long to be shown at all.
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

struct Server {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next: u64,
}

impl Server {
    fn start(home: &Path, cwd: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
            .arg("mcp")
            .env_clear()
            .env("HOME", home)
            .env("PATH", "/usr/bin:/bin")
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("start safe-chains mcp");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        let mut server = Server { child, stdin, stdout, next: 1 };
        let init = server.request(
            "initialize",
            json!({"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}),
        );
        assert!(init["result"]["serverInfo"]["name"] == "safe-chains", "{init}");
        server.notify("notifications/initialized");
        server
    }

    fn notify(&mut self, method: &str) {
        writeln!(self.stdin, "{}", json!({"jsonrpc": "2.0", "method": method})).expect("write");
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        writeln!(self.stdin, "{}", json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})).expect("write");
        self.stdin.flush().expect("flush");
        let mut line = String::new();
        self.stdout.read_line(&mut line).expect("read a reply");
        let reply: Value = serde_json::from_str(&line).unwrap_or_else(|e| panic!("stdout carried a non-protocol line ({e}): {line}"));
        assert_eq!(reply["id"], id, "replies arrive in order, one per request: {reply}");
        reply
    }

    fn call(&mut self, name: &str, arguments: Value) -> (bool, String) {
        let reply = self.request("tools/call", json!({"name": name, "arguments": arguments}));
        let result = &reply["result"];
        (result["isError"] == true, result["content"][0]["text"].as_str().unwrap_or_default().to_string())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn tools(server: &mut Server) -> Vec<Value> {
    server.request("tools/list", json!({}))["result"]["tools"].as_array().cloned().unwrap_or_default()
}

#[test]
fn the_listing_obeys_the_rules() {
    let home = tempfile::tempdir().unwrap();
    let mut server = Server::start(home.path(), home.path());
    let listed = tools(&mut server);
    assert!(!listed.is_empty(), "{} tools", listed.len());
    assert_eq!(tools(&mut server), listed, "the listing must be deterministic");

    let mut names = std::collections::HashSet::new();
    for tool in &listed {
        let name = tool["name"].as_str().unwrap();
        assert!(names.insert(name), "{name} listed twice");
        assert!(name.len() <= 40 && name.contains('_'), "{name}");
        assert!(name.chars().all(|c| c.is_ascii_lowercase() || c == '_'), "{name} is not snake_case");
        let description = tool["description"].as_str().unwrap();
        assert!(description.len() <= 1_000, "{name}: {} characters", description.len());
        let first = description.split(". ").next().unwrap();
        assert!(first.len() <= 160, "{name}: first sentence is {} characters", first.len());
        let schema = &tool["inputSchema"];
        assert_eq!(schema["additionalProperties"], false, "{name}");
        for (property, spec) in schema["properties"].as_object().unwrap() {
            assert!(spec["type"].is_string(), "{name}.{property} has no type");
        }
        let annotations = &tool["annotations"];
        assert!(annotations["readOnlyHint"].is_boolean(), "{name} has no readOnlyHint");
        if annotations["readOnlyHint"] == false {
            assert!(annotations["destructiveHint"].is_boolean(), "{name} writes and has no destructiveHint");
        }
    }
}

#[test]
fn the_instructions_fit_and_say_where_to_start() {
    let home = tempfile::tempdir().unwrap();
    let mut server = Server::start(home.path(), home.path());
    let init = server.request("initialize", json!({"protocolVersion": "2025-11-25"}));
    let instructions = init["result"]["instructions"].as_str().unwrap();
    assert!(instructions.len() <= 2_000, "{} characters", instructions.len());
    assert!(instructions[..500].contains("Start with check_command"), "{instructions}");
}

#[test]
fn bad_arguments_teach() {
    let home = tempfile::tempdir().unwrap();
    let mut server = Server::start(home.path(), home.path());
    let (error, text) = server.call("grant_path", json!({"path": "~/x", "access": "read", "recursive": true}));
    assert!(error && text.contains("recursive") && text.contains("path, access") || text.contains("access, path"), "{text}");
    let (error, text) = server.call("check_command", json!({"command": 42}));
    assert!(error && text.contains("type") && !text.contains("needs"), "{text}");
    assert!(!home.path().join(".config/safe-chains.toml").exists(), "a refused call wrote the config");
}

#[test]
fn every_tool_round_trips_through_the_real_binary() {
    let home = tempfile::tempdir().unwrap();
    let mut server = Server::start(home.path(), home.path());

    let (error, text) = server.call("check_command", json!({"command": "zzmcptool sync"}));
    assert!(!error && text.contains("call add_command_entry with command \"zzmcptool sync\""), "{text}");

    let (error, text) = server.call("add_command_entry", json!({"command": "zzmcptool sync", "level": "safe-read"}));
    assert!(!error, "{text}");
    let (_, text) = server.call("check_command", json!({"command": "zzmcptool sync"}));
    assert!(text.contains("auto-approves"), "the entry did not take effect for the next check: {text}");

    let (error, text) = server.call("grant_path", json!({"path": "/opt/zzmcp", "access": "write"}));
    assert!(!error, "{text}");
    let (_, listed) = server.call("list_rules", json!({}));
    assert!(listed.contains("command zzmcptool") && listed.contains("grant /opt/zzmcp: write"), "{listed}");

    let (error, text) = server.call("grant_path", json!({"path": "~/.ssh", "access": "read"}));
    assert!(error && text.contains("credentials"), "{text}");

    for (kind, name) in [("command", "zzmcptool"), ("grant", "/opt/zzmcp")] {
        let (error, text) = server.call("remove_rule", json!({"kind": kind, "name": name}));
        assert!(!error, "{kind} {name}: {text}");
    }
    let config = std::fs::read_to_string(home.path().join(".config/safe-chains.toml")).unwrap();
    assert!(!config.contains("zzmcp"), "{config}");
}

#[test]
fn edits_stop_when_the_client_approves_them_unasked() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".claude")).unwrap();
    std::fs::write(home.path().join(".claude/settings.json"), r#"{"permissions":{"allow":["mcp__safe-chains"]}}"#).unwrap();
    let mut server = Server::start(home.path(), home.path());
    let (error, text) = server.call("grant_path", json!({"path": "/opt/zzmcp", "access": "read"}));
    assert!(error && text.contains("mcp__safe-chains"), "{text}");
    assert!(!home.path().join(".config/safe-chains.toml").exists());
}

#[test]
fn a_long_listing_is_cut_and_says_how_much() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".config")).unwrap();
    let many: String = (0..800).map(|i| format!("[[grant]]\npath = \"/opt/zzfolder{i}\"\nread = true\n\n")).collect();
    std::fs::write(home.path().join(".config/safe-chains.toml"), many).unwrap();
    let mut server = Server::start(home.path(), home.path());
    let (_, text) = server.call("list_rules", json!({}));
    assert!(text.len() <= 8_200, "{} characters", text.len());
    assert!(text.contains("more lines not shown"), "{}", &text[text.len().saturating_sub(300)..]);
}

/// `tests/mcp_tools.toml` is what the allowlist check reads and what makes a changed tool show in
/// review. Rewrite it with `SAFE_CHAINS_WRITE_MCP_MANIFEST=1 cargo test --test mcp_rules`.
#[test]
fn the_tool_manifest_is_what_the_server_lists() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/mcp_tools.toml");
    let expected = safe_chains::mcp::manifest();
    if std::env::var_os("SAFE_CHAINS_WRITE_MCP_MANIFEST").is_some() {
        std::fs::write(&path, &expected).unwrap();
    }
    let found = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(found, expected, "tests/mcp_tools.toml is out of date; rewrite it with SAFE_CHAINS_WRITE_MCP_MANIFEST=1");

    let home = tempfile::tempdir().unwrap();
    let mut server = Server::start(home.path(), home.path());
    for tool in tools(&mut server) {
        let name = tool["name"].as_str().unwrap();
        let read_only = tool["annotations"]["readOnlyHint"] == true;
        assert!(found.contains(&format!("[{name}]\nread_only = {read_only}\n")), "{name} is listed differently from the manifest");
    }
}

/// Revision 2026-07-28 requires `resultType` on every result and a cache lifetime on a list. Without
/// them Claude Code connected and listed no tools at all: the first blind run had nothing to call.
#[test]
fn every_result_carries_what_the_current_revision_requires() {
    let home = tempfile::tempdir().unwrap();
    let mut server = Server::start(home.path(), home.path());
    for (method, params) in [
        ("initialize", json!({"protocolVersion": "2026-07-28"})),
        ("ping", json!({})),
        ("server/discover", json!({})),
        ("tools/list", json!({})),
        ("tools/call", json!({"name": "list_rules", "arguments": {}})),
    ] {
        let reply = server.request(method, params);
        assert_eq!(reply["result"]["resultType"], "complete", "{method}: {reply}");
    }
    let list = server.request("tools/list", json!({}));
    assert!(list["result"]["ttlMs"].is_u64() && list["result"]["cacheScope"] == "private", "{list}");
}
