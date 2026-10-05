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
