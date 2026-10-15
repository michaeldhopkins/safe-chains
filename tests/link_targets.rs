//! A link makes its target reachable under the link's name, so making one is judged as a write to
//! the target, wherever the link's own name sits.

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
        for dir in [".claude", ".config", "notes", "Library/LaunchAgents"] {
            std::fs::create_dir_all(home.join(dir)).expect("mkdir");
        }
        let work = base.path().join("work");
        std::fs::create_dir_all(work.join("sub")).expect("mkdir work");
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

const THROUGH_THE_LINK: &[&str] = &["echo '{}' > s", "cp x s", "mv x s", "echo '{}' | tee s", "sed -i '' 's/a/b/' s", "sed -i 's/a/b/' s"];

#[test]
fn a_link_to_a_trust_file_is_refused_with_whatever_writes_through_it() {
    for grants in [&[][..], &["~/.claude", "~/.config"]] {
        let machine = Machine::with_grants(grants);
        for link in [
            "ln -s ~/.claude/settings.json s", "ln -sf ~/.claude/settings.json s", "ln --symbolic ~/.claude/settings.json s",
            "ln ~/.claude/settings.json s", "link ~/.claude/settings.json s", "ln -s ~/.config/safe-chains.toml s",
        ] {
            assert!(!machine.approves(link), "grants {grants:?} approved {link}");
            for write in THROUGH_THE_LINK {
                let command = format!("{link} && {write}");
                assert!(!machine.approves(&command), "grants {grants:?} approved {command}");
            }
        }
    }
}

#[test]
fn a_link_to_a_trust_root_directory_is_refused_even_when_granted() {
    let machine = Machine::with_grants(&["~/.claude", "~/.config"]);
    for command in [
        "ln -s ~/.claude l && echo '{}' > l/settings.json",
        "ln -s ~/.config l && cp x l/safe-chains.toml",
        "ln -s ~/.claude sub/",
        "ln -s ~/.claude -t sub",
    ] {
        assert!(!machine.approves(command), "approved {command}");
    }
}

#[test]
fn a_link_to_an_autostart_directory_is_refused() {
    let machine = Machine::with_grants(&[]);
    for command in ["ln -s ~/Library/LaunchAgents la && cp x la/agent.plist", "ln -s /etc/passwd p && echo x >> p"] {
        assert!(!machine.approves(command), "approved {command}");
    }
}

#[test]
fn a_link_out_of_the_workspace_needs_a_grant_on_its_target() {
    assert!(!Machine::with_grants(&[]).approves("ln -s ~/notes/a b"));
    assert!(Machine::with_grants(&["~/notes"]).approves("ln -s ~/notes/a b"));
}

#[test]
fn a_relative_target_resolves_from_the_link_not_the_working_directory() {
    let machine = Machine::with_grants(&["~/.claude"]);
    for command in [
        "ln -s settings.json ~/.claude/l", "ln -s ./settings.json ~/.claude/l", "ln -s ../settings.json ~/.claude/sub/l",
        "ln -s ./sub/../settings.json ~/.claude/l",
    ] {
        assert!(!machine.approves(command), "approved {command}");
    }
    assert!(machine.approves("ln -s notes.md ~/.claude/l"));
}

#[test]
fn links_inside_the_workspace_are_still_approved() {
    let machine = Machine::with_grants(&[]);
    for command in [
        "ln -s ./target ./link", "ln -s target link", "ln -s target sub/link", "ln -s ../target sub/link", "ln -s ../target sub/",
        "ln -sr sub/x link", "ln -s x y z sub", "ln -s -t sub ../x", "ln a b", "ln -s target link && echo hi > link",
    ] {
        assert!(machine.approves(command), "refused {command}");
    }
}

#[test]
fn a_flag_value_is_not_read_as_the_symbolic_or_relative_flag() {
    let machine = Machine::with_grants(&["~/.claude", "~/notes"]);
    std::fs::create_dir_all(std::path::Path::new(&machine.work).join("sub/deep")).expect("mkdir");
    for command in [
        "gln -Ss ../home/.claude/settings.json sub/deep/x",
        "gln -S -s ../home/.claude/settings.json sub/deep/x",
        "gln -Ss ../home/.claude/settings.json sub/",
        "gln -tsub ../home/.claude/settings.json",
        r"find . -name y -exec ln -Ss ../home/.claude/settings.json sub/z \;",
        "gln -s -Sr ~/.claude/settings.json ~/notes/x",
        "gln -s -Sr ../.claude/settings.json ~/notes/x",
        "gln -s -S -r ../.claude/settings.json ~/notes/x",
    ] {
        assert!(!machine.approves(command), "approved {command}");
    }
}

#[test]
fn a_flag_after_an_operand_is_judged_both_ways() {
    let machine = Machine::with_grants(&["~/.claude", "~/notes"]);
    std::fs::create_dir_all(std::path::Path::new(&machine.work).join("sub/deep")).expect("mkdir");
    for command in [
        "ln ../home/.claude/settings.json -s sub/",
        "ln ../home/.claude/settings.json -s sub/deep/",
        "ln ../home/.claude/settings.json -s sub/deep/x",
        "POSIXLY_CORRECT=1 gln ../home/.claude/settings.json -s sub/",
        "ln -s ../.claude/settings.json -r ~/notes",
        "ln -s ../.claude/settings.json -r ~/notes/",
    ] {
        assert!(!machine.approves(command), "approved {command}");
    }
}
