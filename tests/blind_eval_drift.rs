//! Says when the MCP surface has moved far enough from the last blind evaluation that another round
//! is due (`eval/blind/README.md`).
//!
//! The surface is one line per tool (a hash of its name, description and schema), one for the
//! server instructions and one for the protocol revision, read from the spawned server. The last
//! round's lines are `eval/blind/surface.lock`. A round is due when there is none recorded in
//! `eval/blind/results.toml`, when the protocol revision changed, or when the tools and
//! instructions added, removed or changed since number at least max(3, a quarter of them).
//! After a round, rewrite the lock with `SAFE_CHAINS_WRITE_SURFACE_LOCK=1 cargo test --test
//! blind_eval_drift`.
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn surface() -> BTreeMap<String, String> {
    let home = tempfile::tempdir().expect("blind eval drift");
    let mut child = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .arg("mcp")
        .env_clear()
        .env("HOME", home.path())
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("start safe-chains mcp");
    let mut stdin = child.stdin.take().expect("blind eval drift");
    let mut stdout = BufReader::new(child.stdout.take().expect("blind eval drift"));
    let mut ask = |id: u64, method: &str| -> Value {
        writeln!(stdin, "{}", json!({"jsonrpc": "2.0", "id": id, "method": method, "params": {}})).expect("blind eval drift");
        let mut line = String::new();
        stdout.read_line(&mut line).expect("blind eval drift");
        serde_json::from_str(&line).expect("blind eval drift")
    };
    let init = ask(1, "initialize");
    let tools = ask(2, "tools/list");
    let _ = child.kill();
    let _ = child.wait();
    let hash = |text: &str| Sha256::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect::<String>();
    let mut items = BTreeMap::new();
    for tool in tools["result"]["tools"].as_array().expect("blind eval drift") {
        let name = tool["name"].as_str().expect("blind eval drift");
        items.insert(format!("tool {name}"), hash(&format!("{name}\n{}\n{}", tool["description"], tool["inputSchema"])));
    }
    items.insert("instructions".to_string(), hash(init["result"]["instructions"].as_str().expect("blind eval drift")));
    items.insert("protocol".to_string(), safe_chains::mcp::PROTOCOL_VERSION.to_string());
    items
}

fn render(items: &BTreeMap<String, String>) -> String {
    items.iter().map(|(k, v)| format!("{k} {v}\n")).collect()
}

fn parse(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|l| l.rsplit_once(' '))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// How many surface items differ between two fingerprints, counting a rename as two.
fn changed(before: &BTreeMap<String, String>, now: &BTreeMap<String, String>) -> usize {
    let keys: std::collections::BTreeSet<&String> = before.keys().chain(now.keys()).filter(|k| *k != "protocol").collect();
    keys.into_iter().filter(|k| before.get(*k) != now.get(*k)).count()
}

fn due(before: &BTreeMap<String, String>, now: &BTreeMap<String, String>) -> Option<String> {
    if before.get("protocol") != now.get("protocol") {
        return Some(format!("the protocol revision moved from {:?} to {:?}", before.get("protocol"), now.get("protocol")));
    }
    let size = now.keys().filter(|k| *k != "protocol").count();
    let limit = 3.max(size.div_ceil(4));
    let moved = changed(before, now);
    (moved >= limit).then(|| format!("{moved} of {size} surface items changed since the last round (a round is due at {limit})"))
}

#[test]
fn a_round_is_recorded_and_the_surface_has_not_drifted_far() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("eval/blind");
    let now = surface();
    if std::env::var_os("SAFE_CHAINS_WRITE_SURFACE_LOCK").is_some() {
        std::fs::write(root.join("surface.lock"), render(&now)).expect("blind eval drift");
    }
    let results = std::fs::read_to_string(root.join("results.toml")).unwrap_or_default();
    let rounds = toml::from_str::<toml::Value>(&results)
        .ok()
        .and_then(|v| v.get("round").and_then(|r| r.as_array().map(Vec::len)));
    assert!(rounds.unwrap_or(0) > 0, "no blind round is recorded in eval/blind/results.toml; run one (eval/blind/README.md)");
    let before = parse(&std::fs::read_to_string(root.join("surface.lock")).unwrap_or_default());
    if let Some(why) = due(&before, &now) {
        panic!("a blind evaluation round is due: {why}. Run one (eval/blind/README.md), record it, then rewrite surface.lock.");
    }
}

#[test]
fn the_drift_rule_counts_what_it_says() {
    let base: BTreeMap<String, String> = (0..8)
        .map(|i| (format!("tool t{i}"), "a".to_string()))
        .chain([("protocol".to_string(), "p".to_string())])
        .collect();
    let mut two = base.clone();
    two.insert("tool t0".into(), "b".into());
    two.insert("tool t1".into(), "b".into());
    assert!(due(&base, &two).is_none(), "two changes of eight are under max(3, 2)");
    let mut renamed = two.clone();
    renamed.remove("tool t2");
    renamed.insert("tool t9".into(), "a".into());
    assert!(due(&base, &renamed).is_some(), "a rename counts twice, so four changes are due");
    let mut protocol = base.clone();
    protocol.insert("protocol".into(), "q".into());
    assert!(due(&base, &protocol).is_some());
    assert!(due(&base, &BTreeMap::new()).is_some(), "no lock at all");
}
