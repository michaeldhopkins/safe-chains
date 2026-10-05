//! The toolchain and formatting gates stay wired.
//!
//! Before 2026-10-04 the crate had no `rustfmt.toml`, no format check, and CI ran whatever
//! `stable` was that day, so the first `cargo fmt` anyone ran produced a 17,000-line diff and a new
//! release could turn clippy red on code nobody touched. Now the toolchain is pinned to an exact
//! version in `rust-toolchain.toml`, every workflow on the stable channel installs exactly that
//! (`rustup toolchain install` reads the file), and every workflow that lints also checks
//! formatting. The fuzz workflows build on nightly with `cargo +nightly`, which is their own choice.

use std::fs;
use std::path::PathBuf;

const CHECK: &str = "cargo fmt --all --check";
const INSTALL: &str = "rustup toolchain install";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workflows() -> Vec<(String, String)> {
    let dir = root().join(".github/workflows");
    let mut found: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "yml"))
        .filter_map(|p| Some((p.file_name()?.to_string_lossy().into_owned(), fs::read_to_string(&p).ok()?)))
        .collect();
    found.sort();
    found
}

#[test]
fn every_workflow_that_lints_also_checks_formatting() {
    let linting: Vec<_> = workflows().into_iter().filter(|(_, text)| text.contains("cargo clippy")).collect();
    assert!(linting.len() >= 2, "expected ci.yml and release.yml to run clippy");
    let missing: Vec<_> = linting.iter().filter(|(_, text)| !text.contains(CHECK)).map(|(name, _)| name).collect();
    assert!(missing.is_empty(), "workflows that lint without `{CHECK}`: {missing:?}");
}

/// `fuzz/` is its own crate outside the workspace, so `cargo fmt --all` at the root never sees it.
#[test]
fn ci_also_checks_the_fuzz_crates_formatting() {
    let ci = fs::read_to_string(root().join(".github/workflows/ci.yml")).expect("ci.yml");
    assert!(ci.contains("cargo fmt --manifest-path fuzz/Cargo.toml --all --check"), "ci.yml does not check fuzz/'s formatting");
}

#[test]
fn no_workflow_floats_on_stable() {
    let all = workflows();
    assert!(all.len() >= 3, "found {} workflows", all.len());
    let floating: Vec<_> = all
        .iter()
        .filter(|(_, text)| text.contains("rust-toolchain@stable") || text.contains("toolchain: stable"))
        .map(|(n, _)| n)
        .collect();
    assert!(floating.is_empty(), "these install a floating stable instead of `{INSTALL}`: {floating:?}");
    let unpinned: Vec<_> = all
        .iter()
        .flat_map(|(name, text)| unpinned_jobs(text).into_iter().map(move |job| format!("{name}: {job}")))
        .collect();
    assert!(unpinned.is_empty(), "these jobs run cargo on the default toolchain without `{INSTALL}`: {unpinned:?}");
}

/// The jobs that run `cargo` without naming a toolchain (`cargo +nightly …`) and never install the
/// pinned one. Judged per job, not per workflow: a nightly job beside a pinned one installs nothing
/// the other's bare `cargo` could use.
fn unpinned_jobs(workflow: &str) -> Vec<String> {
    jobs(workflow)
        .into_iter()
        .filter(|(_, body)| !body.contains(INSTALL) && body.lines().any(runs_default_cargo))
        .map(|(name, _)| name)
        .collect()
}

/// Each job under `jobs:` with its lines, comments dropped.
fn jobs(workflow: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut in_jobs = false;
    for line in workflow.lines() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        if !line.starts_with(' ') && !line.trim().is_empty() {
            in_jobs = line.trim_end() == "jobs:";
            continue;
        }
        if !in_jobs {
            continue;
        }
        let key = line
            .strip_prefix("  ")
            .filter(|rest| !rest.starts_with(' '))
            .and_then(|rest| rest.trim_end().strip_suffix(':'));
        match (key, out.last_mut()) {
            (Some(name), _) => out.push((name.to_string(), String::new())),
            (None, Some((_, body))) => {
                body.push_str(line);
                body.push('\n');
            }
            (None, None) => {}
        }
    }
    out
}

/// A `cargo` invocation with no `+toolchain`, which runs whatever `rust-toolchain.toml` pins.
fn runs_default_cargo(line: &str) -> bool {
    line.match_indices("cargo ").any(|(i, _)| {
        let starts_word = line[..i].chars().next_back().is_none_or(|c| !(c.is_alphanumeric() || "-_@/".contains(c)));
        starts_word && !line[i + "cargo ".len()..].trim_start().starts_with('+')
    })
}

#[test]
fn the_per_job_check_sees_a_bare_cargo_in_a_nightly_job() {
    let workflow = "on: push\njobs:\n  fuzz:\n    steps:\n      - uses: dtolnay/rust-toolchain@nightly\n      \
                    - run: cargo +nightly fuzz build\n      - run: cargo run --bin gen\n  ci:\n    steps:\n      \
                    - run: rustup toolchain install\n      - run: cargo test\n  msrv:\n    steps:\n      \
                    # cargo build here is only a comment\n      - run: cargo +\"$MSRV\" check\n      \
                    - uses: taiki-e/install-action@cargo-deny\n";
    assert_eq!(jobs(workflow).iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), ["fuzz", "ci", "msrv"]);
    assert_eq!(unpinned_jobs(workflow), ["fuzz"]);
    assert!(unpinned_jobs(&workflow.replace("cargo run --bin gen", "cargo +nightly run --bin gen")).is_empty());
}

#[test]
fn the_toolchain_is_pinned_to_an_exact_release_with_both_linters() {
    let text = fs::read_to_string(root().join("rust-toolchain.toml")).expect("rust-toolchain.toml");
    let toolchain: toml::Table = toml::from_str(&text).expect("rust-toolchain.toml parses");
    let section = toolchain["toolchain"].as_table().expect("[toolchain]");
    let channel = section["channel"].as_str().expect("channel");
    let parts: Vec<_> = channel.split('.').collect();
    assert!(parts.len() == 3 && parts.iter().all(|p| p.parse::<u32>().is_ok()), "channel {channel:?} is not an exact x.y.z release");
    let components: Vec<_> = section["components"].as_array().expect("components").iter().filter_map(|c| c.as_str()).collect();
    for want in ["clippy", "rustfmt"] {
        assert!(components.contains(&want), "rust-toolchain.toml lacks the {want} component");
    }
}

#[test]
fn the_style_is_pinned_in_rustfmt_toml() {
    let text = fs::read_to_string(root().join("rustfmt.toml")).expect("rustfmt.toml");
    for key in ["style_edition", "max_width"] {
        assert!(text.lines().any(|l| l.starts_with(key)), "rustfmt.toml does not set {key}");
    }
}
