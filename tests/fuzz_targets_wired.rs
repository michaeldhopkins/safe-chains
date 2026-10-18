// @template-sha256: 33fb733897ab1149b4ccc7ed2e8ed80617f35b085b0575e9534f6fab394148b3
//! Fuzz-target wiring ratchet.
//! Per-repo configuration lives in `tests/ratchets.toml`.

use std::fs;
use std::path::Path;

#[rustfmt::skip]
struct Config {
    fuzz_min_targets: usize,
    fuzz_check_uploaded_binaries: bool,
}

#[rustfmt::skip]
fn load_config(root: &Path) -> Config {
    let mut cfg = Config { fuzz_min_targets: 2, fuzz_check_uploaded_binaries: false };
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
        match k.trim() {
            "fuzz_min_targets" => {
                cfg.fuzz_min_targets = v.trim().parse().expect("valid fuzz_min_targets");
            }
            "fuzz_check_uploaded_binaries" => {
                cfg.fuzz_check_uploaded_binaries = v.trim() == "true";
            }
            _ => {}
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
fn corpus_cache_keys(src: &str) -> Vec<String> {
    src.lines()
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("key: ").or_else(|| l.strip_prefix("restore-keys: ")))
        .filter(|v| v.starts_with("fuzz-corpus-"))
        .map(str::to_string)
        .collect()
}

#[rustfmt::skip]
const MATRIX_CORPUS_PREFIX: &str = "fuzz-corpus-${{ matrix.target }}-";

#[rustfmt::skip]
#[test]
fn every_fuzz_target_is_wired_into_the_burst() {
    let cfg = load_config(Path::new(env!("CARGO_MANIFEST_DIR")));
    let declared = declared_targets();
    assert!(
        declared.len() >= cfg.fuzz_min_targets,
        "only {} targets found — the Cargo.toml parse is wrong",
        declared.len()
    );
    assert_eq!(
        declared,
        matrix_targets("fuzz.yml"),
        "fuzz/Cargo.toml and the burst disagree. A target missing from the burst never explores, \
         so its corpus never grows and the per-push replay has nothing to replay."
    );
}

#[rustfmt::skip]
#[test]
fn the_replay_restores_the_corpus_the_burst_saves() {
    let burst = corpus_cache_keys(&workflow("fuzz.yml"));
    let replay = corpus_cache_keys(&workflow("fuzz-replay.yml"));
    let saved: Vec<&String> = burst
        .iter()
        .filter(|k| k.starts_with(MATRIX_CORPUS_PREFIX) && k.len() > MATRIX_CORPUS_PREFIX.len())
        .collect();
    assert!(
        !saved.is_empty(),
        "fuzz.yml saves no per-target corpus under `{MATRIX_CORPUS_PREFIX}<unique>`: {burst:?}"
    );
    assert!(
        replay.iter().any(|k| k == MATRIX_CORPUS_PREFIX),
        "fuzz-replay.yml does not restore by the prefix `{MATRIX_CORPUS_PREFIX}` the burst saves under \
         ({saved:?}). Every replay would then find an empty corpus and pass green, gating nothing. Found: {replay:?}"
    );
}

#[rustfmt::skip]
#[test]
fn fuzzing_runs_on_no_schedule() {
    for name in ["fuzz.yml", "fuzz-replay.yml"] {
        assert!(
            !workflow(name).lines().any(|l| l.trim() == "schedule:"),
            "{name} has a `schedule:` trigger. Exploration runs as the burst on each push to main; \
             a deeper run is a workflow_dispatch."
        );
    }
}

#[rustfmt::skip]
#[test]
fn every_fuzz_target_is_wired_into_the_per_push_replay() {
    let declared = declared_targets();
    assert_eq!(
        declared,
        matrix_targets("fuzz-replay.yml"),
        "fuzz/Cargo.toml and the per-push replay disagree."
    );
}

#[rustfmt::skip]
#[test]
fn the_burst_saves_only_a_merged_corpus_and_fails_on_a_broken_burst() {
    let src = workflow("fuzz.yml");
    assert!(
        src.contains("steps.fuzz.outputs.merged == 'true'"),
        "fuzz.yml must save the corpus only when fuzz/burst.sh reports `merged=true`"
    );
    assert!(
        src.contains("FUZZ_OUTCOME: ${{ steps.fuzz.outcome }}")
            && src.contains(r#"[ "$FUZZ_OUTCOME" != success ]"#),
        "fuzz.yml's crash check must also fail on a non-success burst outcome"
    );
    let call = src
        .lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("run: bash fuzz/burst.sh "))
        .map(str::to_string)
        .expect("fuzz.yml does not run `bash fuzz/burst.sh`");
    assert_eq!(
        call.replace("${{ matrix.target }}", "T").split_whitespace().count(),
        3,
        "fuzz/burst.sh takes <binary> <target> <budget> and refuses anything else: `{call}`"
    );
}

#[rustfmt::skip]
#[test]
fn the_burst_uploads_every_target_it_fuzzes() {
    let cfg = load_config(Path::new(env!("CARGO_MANIFEST_DIR")));
    if !cfg.fuzz_check_uploaded_binaries {
        return;
    }
    let src = workflow("fuzz.yml");
    let mut uploaded: Vec<String> = src
        .lines()
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("fuzz/target/${{ env.TRIPLE }}/release/"))
        .map(str::to_string)
        .collect();
    uploaded.sort();
    assert_eq!(
        declared_targets(),
        uploaded,
        "fuzz.yml's build job uploads a different set of binaries than fuzz/Cargo.toml declares."
    );
}
