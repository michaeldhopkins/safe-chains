//! `scripts/mutants-run.sh` decides the mutants job's verdict from `mutants.out/missed.txt` and
//! keeps a run of hung mutants inside a time budget. A stand-in `cargo` plays cargo-mutants.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const MISS: &str = "src/x.rs:1:1: replace f with ()\n";

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/mutants-run.sh")
}

struct Case {
    rc: i32,
    missed: Option<&'static str>,
    sleep: u32,
    ignore_term: bool,
    budget: String,
    passes: bool,
    args: Vec<&'static str>,
    out: &'static str,
    output_env: Option<&'static str>,
}

impl Case {
    fn new(rc: i32, missed: &'static str) -> Self {
        Case {
            rc,
            missed: Some(missed),
            sleep: 0,
            ignore_term: false,
            budget: "60".into(),
            passes: false,
            args: Vec::new(),
            out: ".",
            output_env: None,
        }
    }

    fn hanging(mut self, budget: &str) -> Self {
        self.sleep = 30;
        self.budget = budget.into();
        self
    }
}

struct Run {
    code: i32,
    status: Option<String>,
}

fn run(case: &Case) -> Run {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("bin");
    std::fs::create_dir(&bin).expect("bin dir");
    let cargo = bin.join("cargo");
    // Like cargo-mutants: `--output` outranks CARGO_MUTANTS_OUTPUT and the default (STUB_OUT stands
    // for a config `output`, `--dir` or `--manifest-path`), and a second `--output` is a usage error.
    std::fs::write(
        &cargo,
        "#!/bin/bash\n\
         out=\"${CARGO_MUTANTS_OUTPUT:-$STUB_OUT}\" given=0 previous=\n\
         for arg in \"$@\"; do\n\
           [ \"$arg\" = -- ] && break\n\
           [ \"$previous\" = --output ] && { out=\"$arg\"; given=$((given + 1)); }\n\
           previous=\"$arg\"\n\
         done\n\
         [ \"$given\" -gt 1 ] && exit 2\n\
         [ -n \"$STUB_WRITE\" ] && { mkdir -p \"$out/mutants.out\"; printf '%s' \"$STUB_MISSED\" > \"$out/mutants.out/missed.txt\"; }\n\
         [ -n \"$STUB_IGNORE_TERM\" ] && trap '' TERM\n\
         [ \"$STUB_SLEEP\" -gt 0 ] && { for ((i = 0; i < STUB_SLEEP; i++)); do sleep 1; done; }\n\
         exit \"$STUB_RC\"\n",
    )
    .expect("write stub");
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default());
    let mut cmd = Command::new("/bin/bash");
    cmd.arg(script())
        .args(&case.args)
        .current_dir(dir.path())
        .env_remove("MUTANTS_STATUS")
        .env_remove("MUTANTS_BUDGET_PASSES")
        .env_remove("CARGO_MUTANTS_OUTPUT")
        .env("PATH", path)
        .env("STUB_RC", case.rc.to_string())
        .env("STUB_MISSED", case.missed.unwrap_or(""))
        .env("STUB_SLEEP", case.sleep.to_string())
        .env("STUB_OUT", case.out)
        .env("MUTANTS_BUDGET", &case.budget)
        .env("MUTANTS_GRACE", "2");
    if case.missed.is_some() {
        cmd.env("STUB_WRITE", "1");
    }
    if case.ignore_term {
        cmd.env("STUB_IGNORE_TERM", "1");
    }
    if case.passes {
        cmd.env("MUTANTS_BUDGET_PASSES", "1");
    }
    if let Some(output) = case.output_env {
        cmd.env("CARGO_MUTANTS_OUTPUT", output);
    }
    let out = cmd.output().expect("run script");
    Run {
        code: out.status.code().unwrap_or(-1),
        status: std::fs::read_to_string(dir.path().join("mutants-run-status.txt"))
            .ok()
            .map(|s| s.trim().to_string()),
    }
}

#[test]
fn a_clean_run_passes() {
    let r = run(&Case::new(0, ""));
    assert_eq!((r.code, r.status.as_deref()), (0, Some("complete")));
}

#[test]
fn a_run_with_no_results_directory_passes_when_cargo_mutants_does() {
    let mut case = Case::new(0, "");
    case.missed = None;
    assert_eq!(run(&case).code, 0);
}

#[test]
fn timeouts_without_a_miss_pass() {
    let r = run(&Case::new(3, ""));
    assert_eq!((r.code, r.status.as_deref()), (0, Some("complete")));
}

#[test]
fn timeouts_do_not_hide_a_missed_mutant() {
    assert_eq!(run(&Case::new(3, MISS)).code, 2);
}

#[test]
fn a_missed_mutant_fails_whatever_the_exit_status() {
    assert_eq!(run(&Case::new(2, MISS)).code, 2);
    assert_eq!(run(&Case::new(0, MISS)).code, 2);
}

#[test]
fn exit_two_with_nothing_missed_is_a_fault() {
    assert_eq!(run(&Case::new(2, "")).code, 1);
}

#[test]
fn harness_faults_keep_their_code() {
    for rc in [1, 4, 5, 6, 70] {
        assert_eq!(run(&Case::new(rc, "")).code, rc);
    }
}

#[test]
fn a_budget_that_is_not_a_number_is_refused() {
    let mut case = Case::new(0, "");
    case.budget = "17m".into();
    assert_eq!(run(&case).code, 70);
}

#[test]
fn running_out_of_budget_is_not_a_pass_unless_the_caller_says_so() {
    let started = std::time::Instant::now();
    let r = run(&Case::new(0, "").hanging("1"));
    assert!(started.elapsed().as_secs() < 25, "the budget did not stop the run");
    assert_eq!((r.code, r.status.as_deref()), (124, Some("budget")));
}

#[test]
fn running_out_of_budget_passes_a_slice_with_nothing_missed() {
    let mut case = Case::new(0, "").hanging("1");
    case.passes = true;
    let r = run(&case);
    assert_eq!((r.code, r.status.as_deref()), (0, Some("budget")));
}

#[test]
fn running_out_of_budget_still_reports_what_was_missed() {
    let mut case = Case::new(0, MISS).hanging("1");
    case.passes = true;
    let r = run(&case);
    assert_eq!((r.code, r.status.as_deref()), (2, Some("budget")));
}

#[test]
fn a_process_that_ignores_term_is_killed() {
    let started = std::time::Instant::now();
    let mut case = Case::new(0, "").hanging("1");
    case.ignore_term = true;
    case.passes = true;
    let r = run(&case);
    assert!(started.elapsed().as_secs() < 25, "TERM was never escalated to KILL");
    assert_eq!((r.code, r.status.as_deref()), (0, Some("budget")));
}

#[test]
fn a_missed_mutant_fails_wherever_cargo_mutants_would_write_by_default() {
    let mut case = Case::new(3, MISS);
    case.out = "elsewhere";
    assert_eq!(run(&case).code, 2, "a config output, --dir or --manifest-path");
    case.output_env = Some("elsewhere");
    assert_eq!(run(&case).code, 2, "CARGO_MUTANTS_OUTPUT");
}

#[test]
fn a_caller_output_directory_is_a_usage_error_not_a_pass() {
    let mut case = Case::new(3, MISS);
    case.args = vec!["--output", "out"];
    assert_eq!(run(&case).code, 1);
}

#[test]
fn the_workflow_arguments_and_anything_after_a_double_dash_are_passed_through() {
    for args in [
        vec!["--no-shuffle", "-vV", "-j2", "--in-diff", "mutants.diff"],
        vec!["--no-shuffle", "-vV", "-j2", "--shard", "1/8"],
        vec!["-j2", "-Fparse_word", "-Dmutants.diff", "--", "--output", "x"],
    ] {
        let mut case = Case::new(0, "");
        case.args = args.clone();
        assert_eq!(run(&case).code, 0, "{args:?}");
    }
}
