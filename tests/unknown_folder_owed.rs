//! Every writer accounted for (docs/design/unknown-folder-writes.md §5): a command that writes
//! without naming a path must say what it writes in its folder (`writes_cwd`, or `executor =
//! "project"`), or an unknown folder refuses it whatever the level.
//!
//! The sweep runs every invocation the verdict snapshot records as a write, with the folder
//! unknown at `developer`, and collects each command whose write nothing accounts for. Those not
//! yet labelled are pinned in `tests/fixtures/unknown_folder_owed.txt`, which may only shrink: a
//! new command that writes must be labelled before it lands, and a labelled one must leave the
//! list. Labelling is research, never a guess: a command whose write could land on something
//! sensitive stays unlabelled, and so stays refused.
//!
//! Computed in a child process with the user's config off and HOME fixed, as the verdict snapshot
//! is, so the answer is the same on every machine.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use safe_chains::pathctx::anchor::FolderLevel;
use safe_chains::pathctx::folder::{self, Note};
use safe_chains::pathctx::{self, PathCtx};

const OWED: &str = "tests/fixtures/unknown_folder_owed.txt";
const SNAPSHOT: &str = "tests/fixtures/verdict_snapshot";
const CREATE: &str = "CREATE_UNKNOWN_FOLDER_OWED";
const WORKER_OUT: &str = "UNKNOWN_FOLDER_OWED_WORKER_OUT";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every invocation the snapshot records as a write.
fn writes() -> Vec<String> {
    let mut out = Vec::new();
    for entry in fs::read_dir(root().join(SNAPSHOT)).expect("snapshot dir").flatten() {
        let text = fs::read_to_string(entry.path()).expect("snapshot file");
        out.extend(text.lines().filter_map(|l| l.strip_suffix("\tsafe-write")).map(str::to_string));
    }
    out.sort();
    out
}

/// The command and subcommand of an unaccounted leaf: its first word and, when the second is not a
/// flag, that too.
fn key(command: &str) -> String {
    let mut words = command.split_whitespace();
    let first = words.next().unwrap_or_default();
    match words.next() {
        Some(second) if !second.starts_with(['-', '+']) && second.chars().all(|c| c.is_ascii_alphanumeric() || "_-:.".contains(c)) => {
            format!("{first} {second}")
        }
        _ => first.to_string(),
    }
}

fn unaccounted(invocation: &str) -> Vec<String> {
    let _ctx = pathctx::enter(PathCtx {
        cwd: Some(safe_chains::targets::UNKNOWN_WORKDIR.to_string()),
        root: Some("/work/projects/app".to_string()),
        session_id: None,
    });
    let _folder = folder::enter(FolderLevel::Developer);
    let _ = safe_chains::command_verdict(invocation);
    folder::notes()
        .into_iter()
        .filter_map(|n| match n {
            Note::Leaf { command, admitted: false, .. } => Some(key(&command)),
            _ => None,
        })
        .collect()
}

/// The child half: classify in a clean environment and write the keys found.
#[test]
fn owed_worker() {
    let Some(out) = std::env::var_os(WORKER_OUT) else { return };
    let invocations = writes();
    let threads = std::thread::available_parallelism().map_or(4, usize::from);
    let chunk = invocations.len().div_ceil(threads).max(1);
    let keys: BTreeSet<String> = std::thread::scope(|s| {
        let handles: Vec<_> = invocations
            .chunks(chunk)
            .map(|part| s.spawn(move || part.iter().flat_map(|i| unaccounted(i)).collect::<Vec<_>>()))
            .collect();
        handles.into_iter().flat_map(|h| h.join().expect("worker thread")).collect()
    });
    fs::write(out, keys.into_iter().collect::<Vec<_>>().join("\n") + "\n").expect("write worker output");
}

fn found() -> BTreeSet<String> {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join("owed.txt");
    let status = Command::new(std::env::current_exe().expect("current_exe"))
        .args(["--exact", "owed_worker", "--nocapture", "--test-threads=1"])
        .env_clear()
        .env("HOME", "/nonexistent/unknown-folder-owed-home")
        .env("TMPDIR", "/tmp")
        .env("SAFE_CHAINS_NO_LOCAL", "1")
        .env(WORKER_OUT, &out)
        .current_dir(dir.path())
        .status()
        .expect("spawn worker");
    assert!(status.success(), "worker failed: {status}");
    fs::read_to_string(&out)
        .expect("worker output")
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
fn every_unlabelled_writer_is_owed_and_the_list_only_shrinks() {
    let found = found();
    assert!(found.len() > 10, "the sweep found almost nothing ({}); it has gone blind", found.len());
    let path = root().join(OWED);
    if std::env::var_os(CREATE).is_some() && !path.exists() {
        fs::write(&path, found.iter().cloned().collect::<Vec<_>>().join("\n") + "\n").expect("write owed list");
    }
    let owed: BTreeSet<String> = fs::read_to_string(&path)
        .expect("owed list")
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    let new: Vec<&String> = found.difference(&owed).collect();
    let done: Vec<&String> = owed.difference(&found).collect();
    assert!(
        new.is_empty(),
        "these write in their folder without saying what, so an unknown folder refuses them; label each \
         `writes_cwd` (output with output_dirs, source, or none) or `executor = \"project\"` after researching \
         what it writes:\n{}",
        new.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
    );
    assert!(
        done.is_empty(),
        "these are accounted for now; remove them from {OWED}:\n{}",
        done.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn the_key_is_the_command_and_its_subcommand() {
    assert_eq!(key("cargo build --release"), "cargo build");
    assert_eq!(key("git -C x commit"), "git");
    assert_eq!(key("cargo +nightly fmt"), "cargo");
    assert_eq!(key("pkill"), "pkill");
    assert_eq!(key("rake db:migrate"), "rake db:migrate");
    assert_eq!(key("tee out/x.txt"), "tee");
}
