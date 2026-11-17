//! A `cd` can fail, or never apply to the shell that reads what follows it. Whatever runs after one
//! that may not have taken effect is judged from the folder before the `cd` as well as from the
//! folder after it. The fixture's `1` is a link to a folder outside the project, so
//! `echo evil >> 1/.zshrc` writes outside it from here but inside it from `./q`, where no `1` exists.

use std::path::PathBuf;
use std::process::{Command, Stdio};

struct Machine {
    _base: tempfile::TempDir,
    home: PathBuf,
    work: String,
}

impl Machine {
    fn new() -> Machine {
        let base = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("tempdir");
        let home = base.path().join("home");
        std::fs::create_dir_all(&home).expect("mkdir home");
        let work = base.path().join("work");
        std::fs::create_dir_all(work.join("sub")).expect("mkdir work");
        std::os::unix::fs::symlink(&home, work.join("1")).expect("link");
        std::os::unix::fs::symlink(&home, work.join("sub").join("2")).expect("link");
        let work = work.to_str().expect("utf-8 path").to_string();
        Machine { _base: base, home, work }
    }

    fn approves(&self, command: &str) -> bool {
        Command::new(env!("CARGO_BIN_EXE_safe-chains"))
            .args(["--cwd", &self.work, "--root", &self.work, command])
            .current_dir(&self.work)
            .env_clear()
            .env("HOME", &self.home)
            .env("PATH", "/usr/bin:/bin")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("run safe-chains")
            .code()
            == Some(0)
    }
}

const WRITE: &str = "echo evil >> 1/.zshrc";

#[test]
fn the_fixture_write_is_refused_on_its_own() {
    assert!(!Machine::new().approves(WRITE));
}

#[test]
fn what_runs_when_the_cd_may_have_failed_is_judged_from_both_folders() {
    let machine = Machine::new();
    for command in [
        format!("cd ./nope || {WRITE}"),
        format!("cd ./q\u{a0}&& true || {WRITE}"),
        format!("cd ./q && true & {WRITE}"),
        format!("cd ./q; {WRITE}"),
        format!("cd ./q\n{WRITE}"),
        format!("cd ./q || {WRITE}"),
        format!("cd ./q && true || {WRITE}"),
        format!("cd ./q && ls && true || {WRITE}"),
        format!("false || cd ./q && true || {WRITE}"),
        format!("cd ./q & {WRITE}"),
        format!("cd ./q; true && {WRITE}"),
        format!("cd ./q && true; {WRITE}"),
        format!("(cd ./q; {WRITE})"),
        format!("{{ cd ./q || {WRITE}; }}"),
        format!("if cd ./q; then true; else {WRITE}; fi"),
        format!("cd -- ./q || {WRITE}"),
        format!("! cd ./q && {WRITE}"),
        format!("time cd ./q; {WRITE}"),
        format!("time -p cd ./q || {WRITE}"),
        format!("! time cd ./q && {WRITE}"),
        format!("cd ./q || ! cd ./r && {WRITE}"),
        format!("cd ./q a || {WRITE}"),
    ] {
        assert!(!machine.approves(&command), "`{}` was approved", command.escape_debug());
    }
}

#[test]
fn an_ordinary_cd_chain_keeps_its_verdict() {
    let machine = Machine::new();
    for command in [
        "cd sub && ls", "cd sub && echo x > f", "cd sub && ls && echo x > f", "cd sub; ls", "cd sub\nls", "cd sub && ls || echo x > f",
        "cd ./sub && true || echo x > f", "cd sub && ls | wc -l", "cd .. && cd sub && ls", "ls || cd sub", "false || cd sub && ls",
        "cd sub && ls & echo x > f",
    ] {
        assert!(machine.approves(command), "`{command}` should stay approved");
    }
}

#[test]
fn what_only_runs_after_a_cd_that_worked_is_judged_from_the_new_folder() {
    let machine = Machine::new();
    assert!(machine.approves(&format!("cd sub && {WRITE}")), "no `1` under sub, so the write is a project file");
    assert!(machine.approves(&format!("cd ./q && (true || {WRITE})")), "only reached once the cd worked");
    assert!(!machine.approves("cd sub && echo evil >> ../1/.zshrc"));
}

#[test]
fn nested_groups_each_holding_a_cd_are_judged_in_bounded_time() {
    let mut command = "cd a || cd b; true".to_string();
    for _ in 0..24 {
        command = format!("cd a || cd b; {{ {command}; }}");
    }
    let started = std::time::Instant::now();
    Machine::new().approves(&command);
    assert!(started.elapsed() < std::time::Duration::from_secs(20), "took {:?}", started.elapsed());
}

#[test]
fn a_timed_cd_moves_the_shell_like_a_plain_one() {
    let machine = Machine::new();
    assert!(machine.approves("echo evil >> 2/.zshrc"), "no `2` at the top, so a project file");
    for command in
        ["time cd sub && echo evil >> 2/.zshrc", "time -p cd sub && echo evil >> 2/.zshrc", "time -p -- cd sub; echo evil >> 2/.zshrc"]
    {
        assert!(!machine.approves(command), "`{command}` was approved");
    }
}
