//! The hook against envelopes captured from real grok releases, under `tests/fixtures/grok/<version>/`.
//!
//! Each case holds the envelope grok sent, the reply the hook gave and its exit code, and
//! `outcomes.toml` records what grok then did. The test replays each envelope through the built
//! binary and requires the same reply, so a parser or contract change that would answer a real
//! grok differently fails here. `scripts/capture-grok-fixtures.sh` captures a new version.

#[path = "support/hooks.rs"]
mod hooks;

use std::path::{Path, PathBuf};

const VERSIONS: &[&str] = &["1.0.62"];

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/grok")
}

fn replay(version: &str) -> usize {
    let dir = fixtures().join(version);
    let mut cases = 0;
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir).expect("fixture dir").map(|e| e.expect("entry").path()).collect();
    entries.sort();
    for envelope_path in entries.iter().filter(|p| p.to_string_lossy().ends_with(".envelope.json")) {
        let stem = envelope_path.to_string_lossy().trim_end_matches(".envelope.json").to_string();
        let envelope = std::fs::read_to_string(envelope_path).expect("envelope");
        let want: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(format!("{stem}.reply.json")).expect("reply")).expect("reply json");
        let want_exit: i32 = std::fs::read_to_string(format!("{stem}.exit")).expect("exit").trim().parse().expect("exit code");
        let compact: serde_json::Value = serde_json::from_str(&envelope).expect("envelope json");
        let (stdout, _stderr, code) = hooks::run_hook(&["hook", "grok"], &compact.to_string());
        let got: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_default();
        assert_eq!(got, want, "{stem}: reply differs");
        assert_eq!(code, want_exit, "{stem}: exit code differs");
        cases += 1;
    }
    cases
}

#[test]
fn grok_1_0_62() {
    assert_eq!(replay("1.0.62"), 3, "expected the default, auto and bypassPermissions cases");
}

#[test]
fn every_fixture_version_has_a_test() {
    let mut found: Vec<String> = std::fs::read_dir(fixtures())
        .expect("fixtures/grok")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();
    let mut listed: Vec<String> = VERSIONS.iter().map(|v| (*v).to_string()).collect();
    listed.sort();
    assert_eq!(found, listed, "each tests/fixtures/grok/<version> needs a named test and a VERSIONS entry");
}
