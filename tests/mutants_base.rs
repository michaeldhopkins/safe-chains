//! `scripts/mutants-base.sh` picks the revision the pre-release `--in-diff` mutants check diffs
//! against.
//!
//! It diffs against the given base commit (the last covered tree or replaced tip) only when that
//! commit is in the checkout and an ancestor of HEAD; otherwise HEAD~1. Rewritten history can leave
//! the old base missing from the clone or off HEAD's ancestry, where `git diff <base>..` would fail
//! (2026-10-07). Each case here builds a real repository, so the script's git calls are exercised
//! rather than described.

#[path = "support/isolate_home.rs"]
mod isolate_home;

use std::path::{Path, PathBuf};
use std::process::Command;

const ZEROS: &str = "0000000000000000000000000000000000000000";

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/mutants-base.sh")
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com", "-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("run git");
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).expect("utf8").trim().to_string()
}

fn commit(dir: &Path, name: &str) -> String {
    std::fs::write(dir.join(name), name).expect("write");
    git(dir, &["add", name]);
    git(dir, &["commit", "-q", "-m", name]);
    git(dir, &["rev-parse", "HEAD"])
}

struct Repo {
    dir: tempfile::TempDir,
}

impl Repo {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        git(dir.path(), &["init", "-q", "-b", "main"]);
        Repo { dir }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn base(&self, before: &str) -> (String, String) {
        let out = Command::new("bash")
            .arg(script())
            .arg(before)
            .current_dir(self.path())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .expect("run script");
        assert!(out.status.success(), "script failed: {}", String::from_utf8_lossy(&out.stderr));
        (String::from_utf8(out.stdout).expect("utf8").trim().to_string(), String::from_utf8(out.stderr).expect("utf8"))
    }
}

#[test]
fn an_ancestor_base_diffs_against_that_commit() {
    let repo = Repo::new();
    commit(repo.path(), "a");
    let before = commit(repo.path(), "b");
    commit(repo.path(), "c");
    commit(repo.path(), "d");
    let (base, _) = repo.base(&before);
    assert_eq!(base, before);
}

#[test]
fn an_empty_or_zero_base_falls_back_to_the_first_parent() {
    let repo = Repo::new();
    commit(repo.path(), "a");
    commit(repo.path(), "b");
    for before in [ZEROS, ""] {
        let (base, why) = repo.base(before);
        assert_eq!(base, "HEAD~1");
        assert!(why.contains("no previous base"), "{why}");
    }
}

#[test]
fn a_missing_base_commit_falls_back_to_the_first_parent() {
    let repo = Repo::new();
    commit(repo.path(), "a");
    commit(repo.path(), "b");
    let (base, why) = repo.base("6a75e161f00dfacefeedbeefcafe0123456789ab");
    assert_eq!(base, "HEAD~1");
    assert!(why.contains("not in this checkout"), "{why}");
}

#[test]
fn a_base_off_head_falls_back_to_the_first_parent() {
    let repo = Repo::new();
    let root = commit(repo.path(), "a");
    let old_tip = commit(repo.path(), "b");
    git(repo.path(), &["reset", "-q", "--hard", &root]);
    commit(repo.path(), "c");
    let (base, why) = repo.base(&old_tip);
    assert_eq!(base, "HEAD~1");
    assert!(why.contains("not an ancestor"), "{why}");
}

#[test]
fn mutation_testing_runs_before_release_not_in_a_ci_workflow() {
    let yml = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/mutants.yml");
    assert!(
        !yml.exists(),
        "mutants.yml must not exist: mutation testing runs in the pre-release check before the push, never in CI after the push."
    );
}
