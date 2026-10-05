//! Rules every workflow keeps, checked against the workflow files themselves so a new job or a
//! copied step cannot quietly drop one.

use std::fs;
use std::path::PathBuf;

fn workflows() -> Vec<(String, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".github/workflows");
    let mut found: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "yml"))
        .filter_map(|p| Some((p.file_name()?.to_string_lossy().into_owned(), fs::read_to_string(&p).ok()?)))
        .collect();
    found.sort();
    assert!(found.len() >= 3, "found {} workflows in {}", found.len(), dir.display());
    found
}

/// A `cargo deny check` limited to some checks, or `None` when the line runs the whole policy.
fn partial_deny(line: &str) -> Option<String> {
    let rest = line.split("cargo deny check").nth(1)?.trim();
    let first = rest.split_whitespace().next().unwrap_or("");
    (!first.is_empty() && !first.starts_with('-') && !first.starts_with('#')).then(|| rest.to_string())
}

/// `cargo deny check licenses` alone never consults RUSTSEC.
#[test]
fn cargo_deny_runs_the_whole_policy_including_advisories() {
    let all = workflows();
    let partial: Vec<_> = all
        .iter()
        .flat_map(|(name, text)| text.lines().filter_map(move |l| partial_deny(l).map(|p| format!("{name}: cargo deny check {p}"))))
        .collect();
    assert!(partial.is_empty(), "cargo deny limited to part of the policy: {partial:?}");
    let ci = &all.iter().find(|(n, _)| n == "ci.yml").expect("ci.yml").1;
    assert!(ci.lines().any(|l| l.trim() == "run: cargo deny check"), "ci.yml does not run `cargo deny check`");
    let deny = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("deny.toml")).expect("deny.toml");
    for section in ["[advisories]", "[bans]", "[licenses]"] {
        assert!(deny.contains(section), "deny.toml has no {section}");
    }
}

#[test]
fn a_partial_deny_is_recognised() {
    assert_eq!(partial_deny("        run: cargo deny check licenses"), Some("licenses".into()));
    assert_eq!(partial_deny("        run: cargo deny check advisories bans"), Some("advisories bans".into()));
    assert_eq!(partial_deny("        run: cargo deny check"), None);
    assert_eq!(partial_deny("        run: cargo deny check --hide-inclusion-graph"), None);
    assert_eq!(partial_deny("      - name: build"), None);
}

/// The jobs under `jobs:` that set no `timeout-minutes`. A hung job otherwise runs for GitHub's
/// six-hour default.
fn jobs_without_timeout(text: &str) -> Vec<String> {
    let mut missing = Vec::new();
    let mut in_jobs = false;
    let mut current: Option<(String, bool)> = None;
    for line in text.lines() {
        if !line.starts_with(' ') && !line.is_empty() && !line.starts_with('#') {
            in_jobs = line == "jobs:";
            continue;
        }
        if !in_jobs {
            continue;
        }
        let is_job = line.starts_with("  ") && !line.starts_with("   ") && line.trim_end().ends_with(':') && !line.trim().starts_with('#');
        if is_job {
            if let Some((name, false)) = current.take() {
                missing.push(name);
            }
            current = Some((line.trim().trim_end_matches(':').to_string(), false));
        } else if line.starts_with("    timeout-minutes:")
            && let Some((_, has)) = current.as_mut()
        {
            *has = true;
        }
    }
    if let Some((name, false)) = current {
        missing.push(name);
    }
    missing
}

#[test]
fn every_workflow_is_bounded_and_read_only_by_default() {
    let mut problems = Vec::new();
    for (name, text) in workflows() {
        let top: Vec<&str> = text.lines().collect();
        if !top.contains(&"concurrency:") {
            problems.push(format!("{name}: no top-level concurrency group"));
        }
        match top.iter().position(|l| *l == "permissions:") {
            Some(i) if top.get(i + 1).is_some_and(|l| l.trim() == "contents: read") => {}
            _ => problems.push(format!("{name}: top-level permissions are not `contents: read`")),
        }
        for job in jobs_without_timeout(&text) {
            problems.push(format!("{name}: job `{job}` has no timeout-minutes"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn a_job_without_a_timeout_is_found() {
    let text = "on:\n  push:\njobs:\n  a:\n    runs-on: x\n    timeout-minutes: 5\n    steps:\n      - run: y\n  b:\n    runs-on: x\n    steps:\n      - name: timeout-minutes: 3\n";
    assert_eq!(jobs_without_timeout(text), ["b"]);
    assert!(jobs_without_timeout("jobs:\n  a:\n    timeout-minutes: 1\n").is_empty());
}

#[test]
fn ci_builds_the_docs_with_warnings_denied() {
    let all = workflows();
    let ci = &all.iter().find(|(n, _)| n == "ci.yml").expect("ci.yml").1;
    assert!(ci.contains("cargo doc --locked --no-deps --all-features"), "ci.yml does not build the docs");
    assert!(ci.contains("RUSTDOCFLAGS: -D warnings"), "ci.yml builds the docs without RUSTDOCFLAGS=-D warnings");
}
