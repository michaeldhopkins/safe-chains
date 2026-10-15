//! No grant opens a jj repository write at a trust root. `safe-chains config grant ~/.claude`
//! refuses, and the refusal says no grant can cover it; a grant written into the config by hand
//! must not let `jj -R ~/.claude new` through either, since a repository write reaches every file
//! beneath, the trust file included.

use std::path::PathBuf;
use std::process::{Command, Stdio};

struct Machine {
    _base: tempfile::TempDir,
    home: PathBuf,
    work: String,
}

impl Machine {
    fn with_grants(paths: &[&str]) -> Machine {
        let base = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("tempdir");
        let home = base.path().join("home");
        for dir in [".claude", ".config", "notes"] {
            std::fs::create_dir_all(home.join(dir)).expect("mkdir");
        }
        let work = base.path().join("work");
        std::fs::create_dir_all(&work).expect("mkdir work");
        let grants: String = paths.iter().map(|p| format!("[[grant]]\npath = \"{p}\"\nread = true\nwrite = true\n\n")).collect();
        std::fs::write(home.join(".config/safe-chains.toml"), grants).expect("write config");
        let work = work.to_str().expect("utf-8 path").to_string();
        Machine { _base: base, home, work }
    }

    fn approves_in(&self, dir: &str, command: &str) -> bool {
        Command::new(env!("CARGO_BIN_EXE_safe-chains"))
            .args(["--cwd", dir, "--root", dir, command])
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

    fn approves(&self, command: &str) -> bool {
        self.approves_in(&self.work, command)
    }
}

#[test]
fn a_hand_written_grant_does_not_open_a_repository_write_at_a_trust_root() {
    for grant in ["~/.claude", "~/.claude/", "~/.config", "~/.config/"] {
        let machine = Machine::with_grants(&[grant]);
        let root = grant.trim_end_matches('/');
        for command in [
            format!("jj -R {root} new"),
            format!("jj -R {root} describe -m x"),
            format!("jj -R {root} restore --from @- settings.json"),
            format!("jj --repository {root} new"),
            format!("jj --repository={root} new"),
            format!("jj --ignore-working-copy -R {root} new"),
            format!("jj -R {root} log"),
            format!("cd {root} && jj new"),
            format!("cd {root} && jj log"),
        ] {
            assert!(!machine.approves(&command), "grant {grant} approved {command}");
        }
        let read = format!("jj --ignore-working-copy -R {root} log");
        assert!(machine.approves(&read), "grant {grant}: {read} writes nothing and should be approved");
    }
}

#[test]
fn a_grant_still_opens_an_ordinary_repository() {
    let machine = Machine::with_grants(&["~/notes"]);
    assert!(machine.approves("jj -R ~/notes new"));
    assert!(machine.approves("cd ~/notes && jj new"));
    assert!(!Machine::with_grants(&[]).approves("jj -R ~/notes new"), "without a grant an outside repository must stay refused");
}

#[test]
fn a_session_working_at_the_trust_root_keeps_its_own_repository() {
    let machine = Machine::with_grants(&[]);
    let claude = machine.home.join(".claude");
    let claude = claude.to_str().expect("utf-8 path");
    assert!(machine.approves_in(claude, "jj new"));
    assert!(machine.approves_in(claude, "jj describe -m x"));
}

#[test]
fn a_trailing_slash_does_not_hide_the_trust_root() {
    let machine = Machine::with_grants(&["~/.claude", "~/.config"]);
    for root in ["~/.claude/", "~/.config/", "~/.claude//"] {
        for command in [format!("jj -R {root} restore"), format!("jj --repository={root} new"), format!("jj --repository {root} new")] {
            assert!(!machine.approves(&command), "approved {command}");
        }
    }
}

#[test]
fn a_session_rooted_outside_any_project_still_has_its_working_directory_judged() {
    let machine = Machine::with_grants(&[]);
    let home = machine.home.to_str().expect("utf-8 path").to_string();
    let agents = machine.home.join("Library/LaunchAgents");
    std::fs::create_dir_all(&agents).expect("mkdir");
    let agents = agents.to_str().expect("utf-8 path").to_string();
    let mut dirs = vec!["/", "/etc", home.as_str()];
    if cfg!(target_os = "macos") {
        dirs.extend(["/private/etc", agents.as_str()]);
    }
    for dir in dirs {
        for command in ["jj new", "jj restore", "jj describe -m x"] {
            assert!(!machine.approves_in(dir, command), "a session rooted at {dir} approved {command}");
        }
    }
}
