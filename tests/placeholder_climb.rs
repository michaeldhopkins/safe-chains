//! A command `find` or `fd` runs for each match is judged where it really runs: a `..` after the
//! match can reach above the search path, and `-execdir` runs beside each match.

use std::process::{Command, Stdio};

fn approves(command: &str) -> bool {
    let base = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("tempdir");
    let home = base.path().join("home");
    let work = home.join("proj/w");
    std::fs::create_dir_all(home.join(".config")).expect("mkdir");
    std::fs::create_dir_all(work.join("sub")).expect("mkdir");
    std::fs::write(home.join(".config/safe-chains.toml"), "").expect("write config");
    let work = work.to_str().expect("utf-8 path");
    Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .args(["--cwd", work, "--root", work, command])
        .current_dir(work)
        .env_clear()
        .env("HOME", &home)
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run safe-chains")
        .code()
        == Some(0)
}

#[test]
fn a_climb_after_the_match_is_refused() {
    for command in [
        r"find . -maxdepth 0 -exec rm -rf {}/.. \;",
        r"find . -maxdepth 0 -exec touch {}/../../.zshrc \;",
        r#"find . -maxdepth 0 -exec sh -c "echo x > {}/../../.zshrc" \;"#,
        r"find sub -maxdepth 0 -exec rm -rf {}/../.. \;",
        r"find . -name x -exec cp {} {}/../../.zshrc \;",
        r"find . -maxdepth 0 -execdir rm -rf {}/.. \;",
        "fd -x rm -rf {//}/..",
        "fd . sub -x touch {}/../../../.zshrc",
    ] {
        assert!(!approves(command), "approved {command}");
    }
}

#[test]
fn execdir_is_judged_beside_each_match() {
    for command in [
        r"find ~/.claude/CLAUDE.md -maxdepth 0 -execdir tee settings.json \;",
        r"find ../../.claude -maxdepth 0 -execdir touch settings.json \;",
        r"find ~/.zshenv -execdir touch .zshrc \;",
        r#"find "$HOME/.claude/x" -execdir touch settings.json \;"#,
        r#"find "$(pwd)" -maxdepth 0 -execdir rm -rf w \;"#,
        r"find sub/x/.. -maxdepth 0 -execdir touch ../../.zshrc \;",
        r"find sub/x/y -maxdepth 0 -execdir touch ../../.zshrc \;",
        r"find sub/.. -maxdepth 0 -execdir rm -rf {} \;",
        r"find sub/x/../.. -maxdepth 0 -execdir rm -rf {} \;",
        r"find ./.. -maxdepth 0 -execdir touch {}/x \;",
    ] {
        assert!(!approves(command), "approved {command}");
    }
}

#[test]
fn ordinary_commands_on_each_match_still_work() {
    for command in [
        r"find . -name x -exec rm {} \;", r"find . -type f -exec grep -l foo {} +", r#"find "$(pwd)" -name x -exec cat {} \;"#,
        r"find . -name '*.txt' -execdir cat {} \;", r"find sub -name x -execdir touch y \;", r"find . -name x -execdir mv {} {}.bak \;",
        "fd -x cat {//}/{/}",
    ] {
        assert!(approves(command), "refused {command}");
    }
}
