//! Fuzz-target wiring ratchet.
//! Per-repo configuration lives in `tests/ratchets.toml`.

#[path = "support/isolate_home.rs"]
mod isolate_home;

use std::fs;
use std::path::Path;

#[rustfmt::skip]
struct Config {
    fuzz_min_targets: usize,
}

#[rustfmt::skip]
fn load_config(root: &Path) -> Config {
    let mut cfg = Config { fuzz_min_targets: 2 };
    let Ok(raw) = fs::read_to_string(root.join("tests/ratchets.toml")) else {
        return cfg;
    };
    let mut section = String::new();
    for raw_line in raw.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            continue;
        }
        if !section.is_empty() {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        if k.trim() == "fuzz_min_targets" {
            cfg.fuzz_min_targets = v.trim().parse().expect("valid fuzz_min_targets");
        }
    }
    cfg
}

#[rustfmt::skip]
fn declared_targets() -> Vec<String> {
    let src = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/Cargo.toml"))
        .expect("read fuzz/Cargo.toml");
    let mut out = Vec::new();
    let mut in_bin = false;
    for line in src.lines() {
        let t = line.trim();
        if t == "[[bin]]" {
            in_bin = true;
        } else if t.starts_with('[') {
            in_bin = false;
        } else if in_bin && let Some(rest) = t.strip_prefix("name = ") {
            out.push(rest.trim().trim_matches('"').to_string());
            in_bin = false;
        }
    }
    out.sort();
    out
}

#[rustfmt::skip]
fn workflow(name: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows").join(name))
        .unwrap_or_else(|e| panic!("read {name}: {e}"))
}

#[rustfmt::skip]
fn matrix_targets(name: &str) -> Vec<String> {
    let src = workflow(name);
    let line = src
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("target: ["))
        .unwrap_or_else(|| panic!("{name} declares no `target: [...]` matrix"));
    let inner = line.trim_start_matches("target: [").trim_end_matches(']');
    let mut out: Vec<String> =
        inner.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    out.sort();
    out
}

#[rustfmt::skip]
#[test]
fn every_fuzz_target_is_wired_into_the_per_push_replay() {
    let cfg = load_config(Path::new(env!("CARGO_MANIFEST_DIR")));
    let declared = declared_targets();
    assert!(
        declared.len() >= cfg.fuzz_min_targets,
        "only {} targets found — the Cargo.toml parse is wrong",
        declared.len()
    );
    assert_eq!(
        declared,
        matrix_targets("fuzz-replay.yml"),
        "fuzz/Cargo.toml and the per-push replay disagree."
    );
}

#[rustfmt::skip]
#[test]
fn fuzzing_runs_on_no_schedule_or_post_push_burst() {
    assert!(
        !workflow("fuzz-replay.yml").lines().any(|l| l.trim() == "schedule:"),
        "fuzz-replay.yml has a `schedule:` trigger."
    );
    let burst_workflow = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/fuzz.yml");
    assert!(
        !burst_workflow.exists(),
        "fuzz.yml must not exist: fuzz bursts run in the pre-release check before the push, never in CI after the push."
    );
}
