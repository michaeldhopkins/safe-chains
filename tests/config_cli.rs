//! `safe-chains config` and the fix `--explain` ends with, through the real binary.
//!
//! Every run gets an empty HOME of its own, so nothing here reads or changes the config of the
//! person running the suite.
use std::path::Path;
use std::process::Command;

struct Run {
    ok: bool,
    stdout: String,
    stderr: String,
}

fn safe_chains(home: &Path, cwd: &Path, args: &[&str], surface: Option<&str>) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_safe-chains"));
    command.env_clear().env("HOME", home).env("PATH", "/usr/bin:/bin").current_dir(cwd).args(args);
    if let Some(surface) = surface {
        command.env("SAFE_CHAINS_SURFACE", surface);
    }
    let out = command.output().expect("run safe-chains");
    Run {
        ok: out.status.success(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn config_file(home: &Path) -> String {
    std::fs::read_to_string(home.join(".config/safe-chains.toml")).unwrap_or_default()
}

#[test]
fn add_list_and_remove_an_entry() {
    let home = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let (home, cwd) = (home.path(), cwd.path());

    let before = safe_chains(home, cwd, &["zzcfgtool status --env prod"], None);
    assert!(!before.ok, "an unknown command must start refused");

    let added = safe_chains(home, cwd, &["config", "add", "zzcfgtool", "status", "--env", "prod"], None);
    assert!(added.ok, "{}", added.stderr);
    assert!(added.stdout.contains("It approves `zzcfgtool status`"), "{}", added.stdout);
    assert!(config_file(home).contains("name = \"zzcfgtool\""), "{}", config_file(home));

    assert!(safe_chains(home, cwd, &["zzcfgtool status --env prod"], None).ok, "the entry did not take effect");
    assert!(!safe_chains(home, cwd, &["zzcfgtool destroy"], None).ok, "the entry approved another subcommand");

    let listed = safe_chains(home, cwd, &["config", "list"], None);
    assert!(listed.ok && listed.stdout.contains("command zzcfgtool: subcommands status"), "{}", listed.stdout);

    let again = safe_chains(home, cwd, &["config", "add", "zzcfgtool other"], None);
    assert!(!again.ok && again.stderr.contains("already has an entry"), "{}", again.stderr);

    let removed = safe_chains(home, cwd, &["config", "remove", "command", "zzcfgtool"], None);
    assert!(removed.ok, "{}", removed.stderr);
    assert!(!config_file(home).contains("zzcfgtool"));
    assert!(!safe_chains(home, cwd, &["zzcfgtool status --env prod"], None).ok);
}

#[test]
fn grant_keeps_the_file_and_refuses_protected_paths() {
    let home = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let (home, cwd) = (home.path(), cwd.path());
    std::fs::create_dir_all(home.join(".config")).unwrap();
    let original = "# mine\nlevel = \"developer\"  # keep\n";
    std::fs::write(home.join(".config/safe-chains.toml"), original).unwrap();

    let granted = safe_chains(home, cwd, &["config", "grant", "/opt/zzcfg", "--write"], None);
    assert!(granted.ok, "{}", granted.stderr);
    assert!(config_file(home).starts_with(original), "{}", config_file(home));
    assert!(safe_chains(home, cwd, &["tee /opt/zzcfg/out.txt"], None).ok, "the grant did not take effect");

    for (path, says) in [("~/.ssh", "holds credentials"), ("/", "too much"), ("/etc/sudoers", "log in")] {
        let refused = safe_chains(home, cwd, &["config", "grant", path, "--read"], None);
        assert!(!refused.ok && refused.stderr.contains(says), "{path}: {}", refused.stderr);
        assert!(refused.stderr.contains("Nothing was changed."));
    }
    let no_access = safe_chains(home, cwd, &["config", "grant", "/opt/x"], None);
    assert!(!no_access.ok && no_access.stderr.contains("--read, --write, or both"));

    let removed = safe_chains(home, cwd, &["config", "remove", "grant", "/opt/zzcfg"], None);
    assert!(removed.ok, "{}", removed.stderr);
    assert_eq!(config_file(home), original);
}

#[test]
fn a_file_that_does_not_load_is_left_alone() {
    let home = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".config")).unwrap();
    std::fs::write(home.path().join(".config/safe-chains.toml"), "[[[ broken").unwrap();
    let run = safe_chains(home.path(), cwd.path(), &["config", "grant", "/opt/x", "--read"], None);
    assert!(!run.ok && run.stderr.contains("does not load"), "{}", run.stderr);
    assert_eq!(config_file(home.path()), "[[[ broken");
}

#[test]
fn explain_ends_with_the_fix() {
    let home = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let (home, cwd) = (home.path(), cwd.path());
    for (command, fix) in [
        ("zzcfgtool sync", "Fix: add an entry for `zzcfgtool`:\n  safe-chains config add 'zzcfgtool sync'"),
        ("tee /opt/zzfix/out.txt", "Fix: grant write access to `/opt/zzfix`:\n  safe-chains config grant /opt/zzfix --write"),
        ("git push --force", "Fix: none in safe-chains' config. `git` is built in"),
    ] {
        let run = safe_chains(home, cwd, &["--explain", command], None);
        assert!(!run.ok, "{command} must be refused");
        assert!(run.stdout.contains(fix), "{command}:\n{}", run.stdout);
        assert!(run.stdout.trim_end().lines().last().is_some_and(|l| !l.is_empty()), "{}", run.stdout);
    }
    let approved = safe_chains(home, cwd, &["--explain", "ls"], None);
    assert!(approved.ok && !approved.stdout.contains("Fix:"), "{}", approved.stdout);
}

#[test]
fn over_mcp_the_next_step_names_a_tool() {
    let home = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let run = safe_chains(home.path(), cwd.path(), &["--explain", "zzcfgtool sync"], Some("mcp"));
    assert!(run.stdout.contains("call add_command_entry with command \"zzcfgtool sync\""), "{}", run.stdout);
    assert!(!run.stdout.contains("safe-chains config add"), "{}", run.stdout);
}
