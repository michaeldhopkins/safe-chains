//! The verdict baseline: every invocation the registry and the crate's example tests can name,
//! with the verdict it gets today, committed under `tests/fixtures/verdict_snapshot/` as one file
//! per first character.
//!
//! The TOML `examples_*` pin a few hundred invocations by hand. Everything else a node declares,
//! a flag nobody wrote an example for, could stop being approved and no test would notice. That
//! matters most when the registry's data is rebuilt from another source, which is what this file
//! exists to guard: a rebuilt registry must give every row here the same verdict, or say why.
//!
//! The verdicts are computed in a child process (this same test binary, re-run as the
//! `verdict_worker` test) with the user's own config switched off and HOME and TMPDIR fixed, so
//! the snapshot is the same on every machine and in CI. Setting the environment in this process
//! instead would need `unsafe` and would race the other tests' threads.
//!
//! Regenerate on purpose with `UPDATE_VERDICT_SNAPSHOT=1 cargo test --test verdict_snapshot -- --nocapture`,
//! and read the summary it prints before committing the result.

#[path = "verdict_snapshot/corpus.rs"]
mod corpus;
#[path = "verdict_snapshot/rust_examples.rs"]
mod rust_examples;
#[path = "verdict_snapshot/snapshot.rs"]
mod snapshot;
#[path = "verdict_snapshot/unit.rs"]
mod unit;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use snapshot::{Drift, Snapshot};

const SNAPSHOT: &str = "tests/fixtures/verdict_snapshot";
/// jj's default `snapshot.max-new-file-size`: a larger file is silently left out of the commit.
const MAX_FILE_BYTES: u64 = 1024 * 1024;
const UPDATE: &str = "UPDATE_VERDICT_SNAPSHOT";
/// `--nocapture`, or the summary of what the regeneration changed is swallowed by the harness.
const REGEN: &str = "UPDATE_VERDICT_SNAPSHOT=1 cargo test --test verdict_snapshot -- --nocapture";
const WORKER_IN: &str = "VERDICT_SNAPSHOT_WORKER_IN";
const WORKER_OUT: &str = "VERDICT_SNAPSHOT_WORKER_OUT";
const SHOWN: usize = 25;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every command name the documentation knows, the Rust handlers' included, with its aliases and
/// documented examples.
fn documented(out: &mut BTreeSet<String>) {
    for doc in safe_chains::docs::all_command_docs() {
        for name in std::iter::once(&doc.name).chain(&doc.aliases) {
            for rest in ["", " --help", " --version", " x"] {
                out.insert(format!("{name}{rest}"));
            }
        }
        out.extend(doc.examples.iter().cloned());
    }
}

fn corpus(root: &Path) -> BTreeSet<String> {
    let mut all: BTreeSet<String> = corpus::registry_corpus(root)
        .into_values()
        .flatten()
        .collect();
    all.extend(rust_examples::rust_examples(root));
    documented(&mut all);
    all.retain(|inv| !inv.trim().is_empty());
    all
}

/// Runs `verdict_worker` in a child with a fixed, config-free environment and returns the verdict
/// of each invocation.
fn verdicts(invocations: &BTreeSet<String>) -> Snapshot {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("in.tsv");
    let output = dir.path().join("out.tsv");
    let listed: Snapshot = invocations
        .iter()
        .map(|i| (i.clone(), "denied".to_string()))
        .collect();
    fs::write(&input, snapshot::render(&listed)).expect("write worker input");
    let exe = std::env::current_exe().expect("current_exe");
    let status = Command::new(exe)
        .args([
            "--exact",
            "verdict_worker",
            "--nocapture",
            "--test-threads=1",
        ])
        .env_clear()
        .env("HOME", "/nonexistent/verdict-snapshot-home")
        .env("TMPDIR", "/tmp")
        .env("SAFE_CHAINS_NO_LOCAL", "1")
        .env(WORKER_IN, &input)
        .env(WORKER_OUT, &output)
        .current_dir(dir.path())
        .status()
        .expect("spawn verdict worker");
    assert!(status.success(), "verdict worker failed: {status}");
    let out = snapshot::parse(&fs::read_to_string(&output).expect("worker output"))
        .expect("worker output parses");
    assert_eq!(
        out.len(),
        invocations.len(),
        "the worker answered for a different set of invocations"
    );
    out
}

/// The child half of [`verdicts`]. A no-op unless the parent set its input, so it passes
/// harmlessly when the suite runs it directly.
#[test]
fn verdict_worker() {
    let (Some(input), Some(output)) = (std::env::var_os(WORKER_IN), std::env::var_os(WORKER_OUT))
    else {
        return;
    };
    let text = fs::read_to_string(input).expect("worker input");
    let invocations: Vec<String> = snapshot::parse(&text)
        .expect("worker input parses")
        .into_keys()
        .collect();
    let threads = std::thread::available_parallelism().map_or(4, usize::from);
    let chunk = invocations.len().div_ceil(threads).max(1);
    let results: Snapshot = std::thread::scope(|s| {
        let handles: Vec<_> = invocations
            .chunks(chunk)
            .map(|part| {
                s.spawn(move || {
                    part.iter()
                        .map(|inv| {
                            (
                                inv.clone(),
                                snapshot::verdict_label(safe_chains::command_verdict(inv))
                                    .to_string(),
                            )
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("verdict thread"))
            .collect()
    });
    fs::write(output, snapshot::render(&results)).expect("write worker output");
}

fn snapshot_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "tsv"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

fn read_snapshot(dir: &Path) -> Snapshot {
    let mut all = Snapshot::new();
    for path in snapshot_files(dir) {
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let part = snapshot::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let bucket = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        for (inv, verdict) in part {
            assert_eq!(
                snapshot::bucket_of(&inv),
                bucket,
                "{}: {inv:?} belongs in another file",
                path.display()
            );
            all.insert(inv, verdict);
        }
    }
    all
}

fn write_snapshot(dir: &Path, rows: &Snapshot) {
    fs::create_dir_all(dir).expect("create snapshot dir");
    for stale in snapshot_files(dir) {
        fs::remove_file(&stale).expect("remove old snapshot file");
    }
    for (bucket, text) in snapshot::render_buckets(rows) {
        fs::write(dir.join(format!("{bucket}.tsv")), text).expect("write snapshot file");
    }
}

#[test]
fn verdicts_match_the_snapshot() {
    let root = root();
    let dir = root.join(SNAPSHOT);
    let update = std::env::var_os(UPDATE).is_some();
    let old = read_snapshot(&dir);
    assert!(
        update || !old.is_empty(),
        "{SNAPSHOT} is empty; create it with {REGEN}"
    );

    let generated = corpus(&root);
    assert!(
        generated.len() > 10_000,
        "the corpus shrank to {} invocations; the walk lost something",
        generated.len()
    );
    let mut asked = generated.clone();
    asked.extend(old.keys().cloned());
    let (all, recheck): (Snapshot, Snapshot) = verdicts(&asked)
        .into_iter()
        .partition(|(inv, _)| generated.contains(inv));
    let drift = Drift::between(&old, &all, &recheck);

    if update {
        write_snapshot(&dir, &all);
        eprintln!(
            "wrote {} rows to {SNAPSHOT}\n{}",
            all.len(),
            drift.summary(SHOWN)
        );
        return;
    }
    assert!(
        drift.is_empty(),
        "verdicts differ from {SNAPSHOT}:\n{}\nIf every change is intended, regenerate with \
         {REGEN} and commit the result.",
        drift.summary(SHOWN)
    );
}

#[test]
fn every_snapshot_file_stays_small_enough_for_jj_to_commit() {
    if std::env::var_os(UPDATE).is_some() {
        return;
    }
    let files = snapshot_files(&root().join(SNAPSHOT));
    assert!(files.len() > 10, "{SNAPSHOT} holds {} files", files.len());
    let big: Vec<_> = files
        .iter()
        .filter_map(|p| Some((p.display().to_string(), fs::metadata(p).ok()?.len())))
        .filter(|(_, len)| *len > MAX_FILE_BYTES / 2)
        .collect();
    assert!(
        big.is_empty(),
        "split these by their second character before they reach 1 MiB: {big:?}"
    );
}

#[test]
fn every_command_file_contributes_invocations() {
    let empty: Vec<_> = corpus::registry_corpus(&root())
        .into_iter()
        .filter(|(_, invocations)| invocations.is_empty())
        .map(|(path, _)| path.display().to_string())
        .collect();
    assert!(
        empty.is_empty(),
        "command files that generated nothing: {empty:?}"
    );
}
