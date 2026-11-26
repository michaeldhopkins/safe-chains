//! `git mv` moves files on disk, so its operands are judged where `mv`'s are: both the source and the
//! destination, by the one decision the engine makes for a transfer, with links and every spelling
//! of a path treated as `mv` treats them. Each case runs the real hook against a private home, as a
//! person's session would.

#[path = "support/isolate_home.rs"]
mod isolate_home;

use std::io::Write;
use std::process::{Command, Stdio};

struct Home {
    dir: tempfile::TempDir,
}

impl Home {
    fn new() -> Self {
        let dir = tempfile::tempdir_in("/Library/Caches").or_else(|_| tempfile::tempdir()).expect("tempdir");
        let home = dir.path().canonicalize().expect("canonical").join("home");
        for folder in [
            "projects/proj/src", "projects/proj/conf", "projects/proj/.git/hooks", "projects/proj/.codex/hooks", "projects/proj/sub/deep",
            "projects/other", "hooks", ".claude/hooks", ".config",
        ] {
            std::fs::create_dir_all(home.join(folder)).expect("mkdir");
        }
        for file in ["projects/proj/src/a.rs", "projects/proj/src/b.rs", "projects/proj/conf/settings.json", "projects/proj/README.md"] {
            std::fs::write(home.join(file), "x").expect("file");
        }
        for (link, target) in [("clink", ".codex"), ("glink", ".git"), ("plain", "src"), ("escape", "../../hooks")] {
            std::os::unix::fs::symlink(target, home.join("projects/proj").join(link)).expect("link");
        }
        Self { dir }
    }

    fn home(&self) -> std::path::PathBuf {
        self.dir.path().canonicalize().expect("canonical").join("home")
    }

    fn allows(&self, cwd: &str, command: &str) -> bool {
        let home = self.home();
        let sub = |s: &str| s.replace("@H", home.to_str().expect("utf8"));
        let payload = serde_json::json!({"tool_name": "Bash", "tool_input": {"command": sub(command)}, "cwd": sub(cwd)}).to_string();
        let mut child = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
            .args(["hook", "claude"])
            .current_dir(sub(cwd))
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", &home)
            .env("CLAUDE_PROJECT_DIR", home.join("projects/proj"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        child.stdin.as_mut().expect("stdin").write_all(payload.as_bytes()).expect("write");
        String::from_utf8_lossy(&child.wait_with_output().expect("wait").stdout).contains("\"allow\"")
    }
}

const PROJECT: &str = "@H/projects/proj";

#[test]
fn a_move_between_ordinary_files_inside_the_project_is_still_approved() {
    let h = Home::new();
    for command in [
        "git mv src/a.rs src/c.rs", "git mv src/a.rs conf/a.rs", "git mv README.md docs.md", "git mv -f src/a.rs src/b.rs",
        "git mv -v -k src/a.rs sub/deep", "git mv --dry-run src/a.rs src/c.rs", "git mv -- src/a.rs src/c.rs", "git mv --help",
        "git mv -h",
    ] {
        assert!(h.allows(PROJECT, command), "{command}");
    }
}

#[test]
fn the_reported_moves_into_trust_roots_and_out_of_the_project_are_refused() {
    let h = Home::new();
    for command in [
        "mkdir conf && echo x > conf/settings.json && git add conf && git mv conf .envrc",
        "git mv conf .git",
        "git mv conf .git/hooks/x",
        "git mv conf sub/.envrc",
        "git mv -f conf/settings.json .envrc",
        "git mv -f src/b.rs ../../settings.json",
        "git mv src ../../hooks/pre.sh",
        "git mv ../../x conf",
        "git mv src/a.rs ~/.claude/hooks/pre.sh",
        "git mv src/a.rs @H/hooks/pre.sh",
        "git mv ~/.claude/hooks/pre.sh src/a.rs",
    ] {
        assert!(!h.allows(PROJECT, command), "{command}");
    }
}

#[test]
fn either_operand_alone_is_enough_to_refuse() {
    let h = Home::new();
    for command in [
        "git mv ../../x src/x", "git mv src/x ../../x", "git mv src/a.rs conf/b.rs ../../x", "git mv ../../x src/a.rs conf",
        "git mv src/a.rs conf/b.rs .git", "git mv .git/x src/y",
    ] {
        assert!(!h.allows(PROJECT, command), "{command}");
    }
}

#[test]
fn a_directory_given_by_git_dash_c_is_where_the_operands_land() {
    let h = Home::new();
    for command in [
        "git -C .codex mv a b", "git -C .git mv a b", "git -C .. mv src/a.rs src/b.rs", "git -C ../other mv a b", "git -C @H/hooks mv a b",
        "git -C src mv ../../../hooks/pre.sh x", "git -C src mv a.rs ../../../hooks/pre.sh", "git -C src mv a.rs ../.codex/hooks.json",
    ] {
        assert!(!h.allows(PROJECT, command), "{command}");
    }
}

const SPELLINGS: &[&str] = &[
    "x", "./x", "src/x", "@H/projects/proj/x", "~/projects/proj/x", "~/projects/proj/./x", "src/../x", "@H/projects/proj/src/../x",
    ".claude/x", "./.claude/x", ".codex/x", "./.codex/x", ".codex/hooks/x", ".git/hooks/x", ".git/config", "clink/x", "./clink/x",
    "glink/x", "glink/hooks/x", "plain/x", "escape/x", "escape/pre.sh", "../x", "../../x", "../../hooks/x", "../other/x", "~/x",
    "@H/.claude/x", "@H/.claude/hooks/x", "~/.config/x", "@H/hooks/x", "@H/.claude/x", "/tmp/x", "/etc/x", ".envrc", "sub/.envrc",
    "$HOME/x", "$X/x",
];

/// The one decision: whatever `mv` refuses for an operand, `git mv` refuses too, in every spelling,
/// as source and as destination, whatever flag git mv is given (git has no flag that changes where a
/// path lands). Where `mv` accepts an operand `git mv` accepts it as well, so a move inside the project keeps working.
#[test]
fn git_mv_is_judged_exactly_as_mv_is_for_every_spelling_of_either_operand() {
    let h = Home::new();
    let mut compared = 0;
    for spelling in SPELLINGS {
        for (source, dest) in [("src/a.rs".to_string(), spelling.to_string()), (spelling.to_string(), "src/c.rs".to_string())] {
            let plain = h.allows(PROJECT, &format!("mv {source} {dest}"));
            for flags in ["", "-f ", "-n -v -k "] {
                let git = h.allows(PROJECT, &format!("git mv {flags}{source} {dest}"));
                assert_eq!(git, plain, "`git mv {flags}{source} {dest}` is {git}, `mv {source} {dest}` is {plain}");
                compared += 1;
            }
        }
    }
    assert!(compared > 200, "{compared}");
}

#[test]
fn a_tilde_or_physical_spelling_of_a_frozen_destination_is_refused_under_every_flag() {
    let h = Home::new();
    for flags in ["", "-f ", "-fv ", "-n ", "-- "] {
        for dest in [
            ".git", ".git/hooks/x", ".envrc", "sub/.envrc", "glink/x", "glink/hooks/x", "~/projects/proj/glink/hooks/x",
            "~/projects/proj/.git/hooks/x",
        ] {
            let command = format!("git mv {flags}conf {dest}");
            assert!(!h.allows(PROJECT, &command), "{command}");
        }
    }
}

#[test]
fn a_git_mv_with_no_pair_of_operands_moves_nothing_and_is_not_judged_as_a_move() {
    let h = Home::new();
    assert!(h.allows(PROJECT, "git mv --help"));
    assert!(h.allows(PROJECT, "git log mv"));
    assert!(h.allows(PROJECT, "git add --intent-to-add mv"));
}
