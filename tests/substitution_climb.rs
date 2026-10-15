//! A substitution whose locus is known still says nothing about where `..` after it leads, so a
//! path that climbs out of one is judged as anywhere.

use std::process::{Command, Stdio};

fn approves(command: &str) -> bool {
    let base = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("tempdir");
    let home = base.path().join("home");
    let work = home.join("proj/w");
    std::fs::create_dir_all(home.join(".config")).expect("mkdir");
    std::fs::create_dir_all(&work).expect("mkdir");
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
fn a_climb_after_a_known_substitution_is_refused() {
    for command in
        ["rm -rf \"$(pwd)/..\"", "echo x > \"$(pwd)/../../.zshrc\"", "ln -s ../.claude/settings.json \"$(pwd)/../../Documents/x\""]
    {
        assert!(!approves(command), "approved {command}");
    }
}

#[test]
fn a_known_substitution_inside_the_workspace_still_works() {
    for command in ["echo x > \"$(pwd)/out\"", "cat \"$(pwd)/a\""] {
        assert!(approves(command), "refused {command}");
    }
}

#[test]
fn a_cd_that_climbs_out_of_a_known_substitution_is_judged_as_anywhere() {
    for command in [
        "cd \"$(pwd)/../../.claude\" && touch settings.json",
        "cd \"$(pwd)/../..\" && echo x > .zshrc",
        "cd $(pwd)/.. && echo x > .zshrc",
        "cd \"$(pwd)\"/.. && echo x > .zshrc",
    ] {
        assert!(!approves(command), "approved {command}");
    }
    assert!(approves("cd \"$(pwd)\" && echo x > out"));
}

#[test]
fn a_climb_from_a_working_directory_left_by_a_known_substitution_is_judged_as_anywhere() {
    for command in [
        "cd \"$(pwd)\" && rm -rf ..",
        "cd \"$(pwd)/sub\" && rm -rf ../..",
        "cd \"$(pwd)\" && echo x > ../../.zshrc",
        "cd \"$(pwd)\" && cd ../../.claude && touch settings.json",
        "cd \"$(pwd)/sub\" && cd ../../../.claude && touch settings.json",
    ] {
        assert!(!approves(command), "approved {command}");
    }
    for command in ["cd \"$(pwd)/sub\" && touch ../x", "cd \"$(pwd)\" && cd sub && touch ../x"] {
        assert!(approves(command), "refused {command}");
    }
}
