//! Every fuzz target runs in BOTH halves of the fuzzing program, and nothing drifts.
//!
//! The program has two halves and a target needs both. The nightly EXPLORES — hours of mutation
//! discovering new paths. The per-push replay is the REGRESSION gate — it re-runs the corpus the
//! nightly accumulated, deterministically, in minutes. A target wired only into the nightly still
//! finds bugs, but a regression in it stays invisible until the next morning; a target wired only
//! into the replay never explores, so its corpus never grows and the replay has nothing to say.
//!
//! Drift here is silent in both directions, which is why this is a test rather than a convention.
//! `gate_prefilter` was added to `fuzz/Cargo.toml` and the nightly matrix and NOT to the replay,
//! and nothing complained — the workflows are YAML that no compiler reads.
//!
//! The authority is `fuzz/Cargo.toml`: a target exists when it has a `[[bin]]`. Both workflows must
//! list exactly that set.

/// Target names from `fuzz/Cargo.toml` — every `[[bin]]`'s `name`.
fn declared_targets() -> Vec<String> {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/Cargo.toml"),
    )
    .expect("read fuzz/Cargo.toml");

    let mut out = Vec::new();
    let mut in_bin = false;
    for line in src.lines() {
        let t = line.trim();
        if t == "[[bin]]" {
            in_bin = true;
            continue;
        }
        if t.starts_with('[') {
            in_bin = false;
            continue;
        }
        if in_bin && let Some(rest) = t.strip_prefix("name = ") {
            out.push(rest.trim().trim_matches('"').to_string());
            in_bin = false;
        }
    }
    out.sort();
    out
}

/// The `target: [a, b, c]` matrix list from a workflow. Takes the first one, which is the only one
/// either workflow has.
fn matrix_targets(workflow: &str) -> Vec<String> {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows").join(workflow),
    )
    .unwrap_or_else(|e| panic!("read {workflow}: {e}"));

    let line = src
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("target: ["))
        .unwrap_or_else(|| panic!("{workflow} declares no `target: [...]` matrix"));

    let inner = line
        .trim_start_matches("target: [")
        .trim_end_matches(']');
    let mut out: Vec<String> =
        inner.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    out.sort();
    out
}

/// The nightly runs `parse` in its own sharded job rather than the matrix, because it is the one
/// target big enough to need sharding. It is still a target and still must be replayed, so the
/// comparison adds it back rather than exempting it.
fn nightly_targets() -> Vec<String> {
    let mut out = matrix_targets("fuzz.yml");
    out.push("parse".to_string());
    out.sort();
    out
}

#[test]
fn every_fuzz_target_is_wired_into_the_nightly() {
    let declared = declared_targets();
    assert!(declared.len() >= 8, "only {} targets found — the Cargo.toml parse is wrong", declared.len());
    assert_eq!(
        declared,
        nightly_targets(),
        "fuzz/Cargo.toml and the nightly disagree. A target missing from the nightly never explores, \
         so its corpus never grows and the per-push replay has nothing to replay."
    );
}

#[test]
fn every_fuzz_target_is_wired_into_the_per_push_replay() {
    let declared = declared_targets();
    assert_eq!(
        declared,
        matrix_targets("fuzz-replay.yml"),
        "fuzz/Cargo.toml and the per-push replay disagree. A target missing from the replay still \
         finds bugs overnight, but a regression in it stays invisible until the next morning — \
         which is the whole reason the replay exists."
    );
}
