//! `scripts/mutants-base.sh` picks the revision the in-diff mutants job diffs against.
//!
//! A push diffs against the tip it replaced only when that tip is a commit in the checkout and an
//! ancestor of HEAD; otherwise HEAD~1. A force-push that rewrote history left `before` missing
//! from the clone and `git diff <before>..` failed the job (2026-10-07). Each case here builds a real
//! repository, so the script's git calls are exercised rather than described.

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

    fn base(&self, event: &str, base_ref: &str, before: &str) -> (String, String) {
        let out = Command::new("bash")
            .arg(script())
            .args([event, base_ref, before])
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
fn a_pull_request_diffs_against_its_base_branch() {
    let repo = Repo::new();
    commit(repo.path(), "a");
    let (base, _) = repo.base("pull_request", "main", "");
    assert_eq!(base, "origin/main");
}

#[test]
fn a_fast_forward_push_diffs_against_the_tip_it_replaced() {
    let repo = Repo::new();
    commit(repo.path(), "a");
    let before = commit(repo.path(), "b");
    commit(repo.path(), "c");
    commit(repo.path(), "d");
    let (base, _) = repo.base("push", "", &before);
    assert_eq!(base, before);
}

#[test]
fn a_new_branch_falls_back_to_the_first_parent() {
    let repo = Repo::new();
    commit(repo.path(), "a");
    commit(repo.path(), "b");
    for before in [ZEROS, ""] {
        let (base, why) = repo.base("push", "", before);
        assert_eq!(base, "HEAD~1");
        assert!(why.contains("new branch"), "{why}");
    }
}

#[test]
fn a_force_push_whose_old_tip_is_gone_falls_back_to_the_first_parent() {
    let repo = Repo::new();
    commit(repo.path(), "a");
    commit(repo.path(), "b");
    let (base, why) = repo.base("push", "", "6a75e161f00dfacefeedbeefcafe0123456789ab");
    assert_eq!(base, "HEAD~1");
    assert!(why.contains("not in this checkout"), "{why}");
}

#[test]
fn a_force_push_whose_old_tip_is_off_head_falls_back_to_the_first_parent() {
    let repo = Repo::new();
    let root = commit(repo.path(), "a");
    let old_tip = commit(repo.path(), "b");
    git(repo.path(), &["reset", "-q", "--hard", &root]);
    commit(repo.path(), "c");
    let (base, why) = repo.base("push", "", &old_tip);
    assert_eq!(base, "HEAD~1");
    assert!(why.contains("not an ancestor"), "{why}");
}

#[test]
fn the_workflow_computes_its_base_with_the_script_on_full_history() {
    let yml =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/mutants.yml")).expect("read mutants.yml");
    let in_diff = &yml[yml.find("in-diff:").expect("in-diff job")..yml.find("slice:").expect("slice job")];
    assert!(in_diff.contains("fetch-depth: 0"), "in-diff checkout must fetch full history");
    assert!(
        in_diff.contains(r#"scripts/mutants-base.sh "$EVENT_NAME" "$BASE_REF" "$BEFORE""#),
        "Compute diff must take its base from scripts/mutants-base.sh"
    );
}
