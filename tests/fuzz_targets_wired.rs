//! Every fuzz target runs in BOTH halves of the fuzzing program, and nothing drifts.
//!
//! The program has two halves and a target needs both. The burst on each push to `main`
//! (`fuzz.yml`) EXPLORES — a few minutes of mutation per target, merged into that target's saved
//! corpus. The per-push replay (`fuzz-replay.yml`) is the REGRESSION gate — it re-runs that
//! corpus, deterministically, in minutes. A target wired only into the burst still finds bugs, but
//! nothing gates a regression in it; a target wired only into the replay never explores, so its
//! corpus never grows and the replay has nothing to say.
//!
//! Drift here is silent in both directions, which is why this is a test rather than a convention.
//! `gate_prefilter` was added to `fuzz/Cargo.toml` and the exploring matrix and NOT to the replay,
//! and nothing complained — the workflows are YAML that no compiler reads. The same is true of the
//! cache key that joins them: if the burst saved under a prefix the replay does not restore, every
//! replay would find an empty corpus and pass green, gating nothing.
//!
//! The authority is `fuzz/Cargo.toml`: a target exists when it has a `[[bin]]`. Both workflows must
//! list exactly that set.

/// Target names from `fuzz/Cargo.toml` — every `[[bin]]`'s `name`.
fn declared_targets() -> Vec<String> {
    let src =
        std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/Cargo.toml")).expect("read fuzz/Cargo.toml");

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
fn matrix_targets(name: &str) -> Vec<String> {
    let src = workflow(name);

    let line = src
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("target: ["))
        .unwrap_or_else(|| panic!("{name} declares no `target: [...]` matrix"));

    let inner = line.trim_start_matches("target: [").trim_end_matches(']');
    let mut out: Vec<String> = inner.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    out.sort();
    out
}

fn workflow(name: &str) -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows").join(name))
        .unwrap_or_else(|e| panic!("read {name}: {e}"))
}

/// Every `key:` / `restore-keys:` value in a workflow that names a corpus cache.
fn corpus_cache_keys(src: &str) -> Vec<String> {
    src.lines()
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("key: ").or_else(|| l.strip_prefix("restore-keys: ")))
        .filter(|v| v.starts_with("fuzz-corpus-"))
        .map(str::to_string)
        .collect()
}

const MATRIX_CORPUS_PREFIX: &str = "fuzz-corpus-${{ matrix.target }}-";

#[test]
fn every_fuzz_target_is_wired_into_the_burst() {
    let declared = declared_targets();
    assert!(declared.len() >= 8, "only {} targets found — the Cargo.toml parse is wrong", declared.len());
    assert_eq!(
        declared,
        matrix_targets("fuzz.yml"),
        "fuzz/Cargo.toml and the burst disagree. A target missing from the burst never explores, \
         so its corpus never grows and the per-push replay has nothing to replay."
    );
}

#[test]
fn the_replay_restores_the_corpus_the_burst_saves() {
    let burst = corpus_cache_keys(&workflow("fuzz.yml"));
    let replay = corpus_cache_keys(&workflow("fuzz-replay.yml"));

    let saved: Vec<&String> = burst
        .iter()
        .filter(|k| k.starts_with(MATRIX_CORPUS_PREFIX) && k.len() > MATRIX_CORPUS_PREFIX.len())
        .collect();
    assert!(!saved.is_empty(), "fuzz.yml saves no per-target corpus under `{MATRIX_CORPUS_PREFIX}<unique>`: {burst:?}");
    assert!(
        replay.iter().any(|k| k == MATRIX_CORPUS_PREFIX),
        "fuzz-replay.yml does not restore by the prefix `{MATRIX_CORPUS_PREFIX}` the burst saves under \
         ({saved:?}). Every replay would then find an empty corpus and pass green, gating nothing. Found: {replay:?}"
    );
}

#[test]
fn fuzzing_runs_on_no_schedule() {
    for name in ["fuzz.yml", "fuzz-replay.yml"] {
        assert!(
            !workflow(name).lines().any(|l| l.trim() == "schedule:"),
            "{name} has a `schedule:` trigger. The nightly was retired on 2026-09-26: its finds all came \
             in each target's first days, and its later red runs were job timeouts. Exploration runs \
             as the burst on each push to main; a deeper run is a workflow_dispatch."
        );
    }
}

#[test]
fn every_fuzz_target_is_wired_into_the_per_push_replay() {
    let declared = declared_targets();
    assert_eq!(
        declared,
        matrix_targets("fuzz-replay.yml"),
        "fuzz/Cargo.toml and the per-push replay disagree. A target missing from the replay still \
         finds bugs in the burst, but nothing gates a regression in it — which is the whole reason \
         the replay exists."
    );
}

#[test]
fn the_burst_saves_only_a_merged_corpus_and_fails_on_a_broken_burst() {
    let src = workflow("fuzz.yml");
    assert!(
        src.contains("steps.fuzz.outputs.merged == 'true'"),
        "fuzz.yml must save the corpus only when fuzz/burst.sh reports `merged=true`; a save after a \
         failed merge makes a half-merged corpus canonical for every later replay"
    );
    assert!(
        src.contains("FUZZ_OUTCOME: ${{ steps.fuzz.outcome }}") && src.contains(r#"[ "$FUZZ_OUTCOME" != success ]"#),
        "fuzz.yml's crash check must also fail on a non-success burst outcome: the burst step is \
         continue-on-error, so a burst that broke without writing an artifact would otherwise be green"
    );
    let call = src
        .lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("run: bash fuzz/burst.sh "))
        .map(str::to_string)
        .expect("fuzz.yml does not run `bash fuzz/burst.sh`");
    assert_eq!(
        // Each argument is double-quoted (`${{ matrix.target }}` has spaces inside it), so count
        // the quoted spans; anything outside them is an unquoted extra argument.
        call.split('"').skip(1).step_by(2).count() + call.split('"').step_by(2).filter(|s| !s.trim().is_empty()).count(),
        3,
        "fuzz/burst.sh takes <binary> <target> <budget> and refuses anything else: `{call}`"
    );
}
