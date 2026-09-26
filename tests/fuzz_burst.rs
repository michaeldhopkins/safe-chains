//! `fuzz/burst.sh` against a stand-in libFuzzer binary, so its corpus bookkeeping is tested on
//! stable Rust without building a fuzz target.
//!
//! The stand-in behaves the way the real binary does where the script depends on it: in fuzz mode
//! it prints `INITED`, writes one find into its first directory and waits for SIGINT (exit 72); with
//! `-merge=1` it copies every input into the first directory under a content-derived name, which
//! is how libFuzzer renames what it keeps. The things under test are the ones a local run got
//! wrong: one shared `fuzz/new` merged each target's finds into the next target's corpus, and the
//! merge renamed the committed `seed-*` inputs, deleting tracked files from the working copy.
//!
//! The wait for SIGINT is perl, not a shell `trap`: burst.sh starts the fuzzer as a background job
//! of a non-interactive shell, which starts it with SIGINT ignored, and a POSIX shell cannot trap a
//! signal that was ignored on entry. libFuzzer installs its handler with sigaction, which can; so
//! does perl. A `trap` stand-in never exits and the test hangs.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const FAKE_LIBFUZZER: &str = r#"#!/bin/sh
echo "$*" >> "$FAKE_ARGS_LOG"
merge=0; dirs=""; prefix=""
for a in "$@"; do
  case "$a" in
    -merge=1) merge=1 ;;
    -artifact_prefix=*) prefix="${a#-artifact_prefix=}" ;;
    -*) ;;
    *) dirs="$dirs $a" ;;
  esac
done
set -- $dirs
out="$1"; shift
if [ "$merge" = 1 ]; then
  [ -n "${FAKE_MERGE_FAIL:-}" ] && exit 1
  for d in "$@"; do
    for f in "$d"/*; do
      [ -f "$f" ] && cp "$f" "$out/h$(cksum < "$f" | cut -d' ' -f1)"
    done
  done
  exit 0
fi
echo "find from $(basename "$out")" > "$out/find"
if [ -n "${FAKE_CRASH:-}" ]; then
  echo boom > "${prefix}crash-fake"
  exit 1
fi
exec perl -e '$SIG{INT} = sub { exit 72 }; print STDERR "INFO: INITED\n"; sleep 1 while 1'
"#;

/// Runs burst.sh in its own process group and kills the whole group after a deadline, so a
/// stand-in that never sees its SIGINT fails the test instead of hanging it and leaving the
/// stand-in running.
fn run_bounded(mut cmd: Command) -> i32 {
    let mut child = cmd
        .process_group(0)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("run fuzz/burst.sh");
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(status) = child.try_wait().expect("wait for fuzz/burst.sh") {
            return status.code().unwrap_or(-1);
        }
        if Instant::now() > deadline {
            let _ = Command::new("kill")
                .args(["-9", &format!("-{}", child.id())])
                .status();
            panic!("fuzz/burst.sh did not finish within 60s");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

struct Repo {
    root: PathBuf,
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Repo {
    fn new() -> Self {
        static N: AtomicUsize = AtomicUsize::new(0);
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "fuzz-burst-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("fuzz")).expect("burst test fixture");
        let bin = root.join("fake-libfuzzer");
        fs::write(&bin, FAKE_LIBFUZZER).expect("burst test fixture");
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("burst test fixture");
        Repo { root }
    }

    fn write(&self, rel: &str, contents: &str) {
        let p = self.root.join(rel);
        fs::create_dir_all(p.parent().expect("burst test fixture")).expect("burst test fixture");
        fs::write(p, contents).expect("burst test fixture");
    }

    fn burst(&self, target: &str, budget: &str, env: &[(&str, &str)]) -> (i32, String) {
        let output = self.root.join("github-output");
        let _ = fs::remove_file(&output);
        let mut cmd = Command::new("bash");
        cmd.arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/burst.sh"))
            .args(["./fake-libfuzzer", target, budget])
            .current_dir(&self.root)
            .env("GITHUB_OUTPUT", &output)
            .env("FAKE_ARGS_LOG", self.root.join("args.log"))
            .env_remove("FAKE_CRASH")
            .env_remove("FAKE_MERGE_FAIL");
        for (k, v) in env {
            cmd.env(k, v);
        }
        (
            run_bounded(cmd),
            fs::read_to_string(&output).unwrap_or_default(),
        )
    }

    /// The contents of every file in a target's corpus, sorted.
    fn corpus(&self, target: &str) -> Vec<String> {
        let mut out: Vec<String> = fs::read_dir(self.root.join("fuzz/corpus").join(target))
            .expect("burst test fixture")
            .map(|e| {
                fs::read_to_string(e.expect("burst test fixture").path())
                    .expect("burst test fixture")
            })
            .collect();
        out.sort();
        out
    }

    fn corpus_names(&self, target: &str) -> Vec<String> {
        fs::read_dir(self.root.join("fuzz/corpus").join(target))
            .expect("burst test fixture")
            .map(|e| {
                e.expect("burst test fixture")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }
}

#[test]
fn sequential_targets_keep_their_finds_apart() {
    let repo = Repo::new();
    assert_eq!(
        repo.burst("alpha", "0", &[]),
        (0, "merged=true\n".to_string())
    );
    assert_eq!(
        repo.burst("beta", "0", &[]),
        (0, "merged=true\n".to_string())
    );

    assert_eq!(repo.corpus("alpha"), ["find from alpha\n"]);
    assert_eq!(
        repo.corpus("beta"),
        ["find from beta\n"],
        "alpha's find leaked into beta's corpus"
    );
    assert!(
        !repo.root.join("fuzz/new/alpha").exists() && !repo.root.join("fuzz/new/beta").exists()
    );
}

#[test]
fn committed_seeds_keep_their_names_through_the_merge() {
    let repo = Repo::new();
    repo.write("fuzz/corpus/parse/seed-ls", "ls -la");
    repo.write("fuzz/corpus/parse/0123abcd", "git status");

    assert_eq!(repo.burst("parse", "0", &[]).0, 0);

    let names = repo.corpus_names("parse");
    assert!(
        names.contains(&"seed-ls".to_string()),
        "seed-ls was renamed or dropped: {names:?}"
    );
    assert!(
        !names.contains(&"0123abcd".to_string()),
        "the corpus was not replaced by the merge: {names:?}"
    );
    assert_eq!(
        fs::read_to_string(repo.root.join("fuzz/corpus/parse/seed-ls"))
            .expect("burst test fixture"),
        "ls -la"
    );
}

#[test]
fn a_crash_still_merges_and_keeps_the_fuzzer_status() {
    let repo = Repo::new();
    repo.write("fuzz/corpus/t/a", "prior");

    let (rc, output) = repo.burst("t", "0", &[("FAKE_CRASH", "1")]);

    assert_eq!(rc, 1);
    assert_eq!(output, "merged=true\n");
    assert!(repo.root.join("fuzz/artifacts/t/crash-fake").exists());
    assert_eq!(repo.corpus("t"), ["find from t\n", "prior"]);
}

#[test]
fn a_failed_merge_leaves_the_corpus_and_is_not_marked_for_saving() {
    let repo = Repo::new();
    repo.write("fuzz/corpus/t/a", "prior");

    let (rc, output) = repo.burst("t", "0", &[("FAKE_MERGE_FAIL", "1")]);

    assert_eq!(rc, 2);
    assert_eq!(output, "", "a failed merge must not set merged=true");
    assert_eq!(repo.corpus("t"), ["prior"]);
}

#[test]
fn the_target_dictionary_is_used_when_present() {
    let repo = Repo::new();
    repo.write("fuzz/dict/parse.dict", "\"git\"\n");

    repo.burst("parse", "0", &[]);
    repo.burst("other", "0", &[]);

    let log = fs::read_to_string(repo.root.join("args.log")).expect("burst test fixture");
    let fuzz_runs: Vec<&str> = log.lines().filter(|l| !l.contains("-merge=1")).collect();
    assert_eq!(fuzz_runs.len(), 2);
    assert!(
        fuzz_runs[0].contains("-dict=fuzz/dict/parse.dict"),
        "{}",
        fuzz_runs[0]
    );
    assert!(!fuzz_runs[1].contains("-dict="), "{}", fuzz_runs[1]);
}

#[test]
fn a_bad_budget_or_arity_is_a_usage_error() {
    let repo = Repo::new();
    assert_eq!(repo.burst("t", "3m", &[]).0, 64);
    assert_eq!(repo.burst("t", "", &[]).0, 64);

    let mut four = Command::new("bash");
    four.arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/burst.sh"))
        .args(["./fake-libfuzzer", "t", "60", "fuzz/dict/t.dict"])
        .current_dir(&repo.root);
    assert_eq!(
        run_bounded(four),
        64,
        "the old four-argument form must be refused, not ignored"
    );
    assert!(
        !repo.root.join("args.log").exists(),
        "the fuzzer ran despite a usage error"
    );
}

#[test]
fn a_zero_padded_budget_is_decimal_not_octal() {
    let repo = Repo::new();
    assert_eq!(repo.burst("t", "08", &[]).0, 0);
    let log = fs::read_to_string(repo.root.join("args.log")).expect("burst test fixture");
    assert!(log.contains("-max_total_time=3608"), "{log}");
}
