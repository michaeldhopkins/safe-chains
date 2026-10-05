//! The formatting gate stays wired. Before 2026-10-04 the crate had no `rustfmt.toml` and no CI
//! step, and the first `cargo fmt` anyone ran produced a 17,000-line diff. This holds the config in
//! place and keeps the check in every workflow that runs clippy, so the two gates cannot drift
//! apart.

use std::fs;
use std::path::PathBuf;

const CHECK: &str = "cargo fmt --all --check";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The workflows that lint, and whether each also checks formatting and installs rustfmt.
fn linting_workflows() -> Vec<(String, bool, bool)> {
    let dir = root().join(".github/workflows");
    let mut found: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "yml"))
        .filter_map(|p| {
            let text = fs::read_to_string(&p).ok()?;
            text.contains("cargo clippy").then(|| {
                let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                (name, text.contains(CHECK), text.contains("rustfmt"))
            })
        })
        .collect();
    found.sort();
    found
}

#[test]
fn every_workflow_that_lints_also_checks_formatting() {
    let workflows = linting_workflows();
    assert!(workflows.len() >= 2, "expected ci.yml and release.yml to run clippy: {workflows:?}");
    let missing: Vec<_> = workflows.iter().filter(|(_, check, component)| !(*check && *component)).collect();
    assert!(missing.is_empty(), "workflows that lint without `{CHECK}` and the rustfmt component: {missing:?}");
}

#[test]
fn the_style_is_pinned_in_rustfmt_toml() {
    let text = fs::read_to_string(root().join("rustfmt.toml")).expect("rustfmt.toml");
    for key in ["style_edition", "max_width"] {
        assert!(text.lines().any(|l| l.starts_with(key)), "rustfmt.toml does not set {key}");
    }
}
