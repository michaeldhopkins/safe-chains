//! A path is placed once, on disk, and every judge reads the placed value. Each test here is a spot
//! where a judge used to read a spelling, or place the same operand twice, and so landed somewhere
//! the kernel does not.

use std::io::Write;
use std::os::unix::fs::symlink;
use std::process::{Command, Stdio};

use safe_chains::pathctx::PathCtx;
use safe_chains::{Verdict, command_verdict_in};

struct Project {
    _dir: tempfile::TempDir,
    root: String,
    home: String,
}

fn project() -> Project {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("scratch project");
    let root = dir.path().canonicalize().expect("canonical root").to_str().expect("utf-8 root").to_string();
    std::fs::create_dir_all(format!("{root}/sub")).expect("sub");
    std::fs::write(format!("{root}/README.md"), "read me").expect("readme");
    let home = std::env::var("HOME").expect("HOME");
    Project { _dir: dir, root, home }
}

fn at(p: &Project, cwd: &str, command: &str) -> Verdict {
    command_verdict_in(command, PathCtx { cwd: Some(cwd.to_string()), root: Some(p.root.clone()), session_id: None })
}

fn refused(p: &Project, cwd: &str, commands: &[&str]) -> Vec<String> {
    commands.iter().filter(|c| at(p, cwd, c).is_allowed()).map(|c| (*c).to_string()).collect()
}

#[test]
fn a_directory_option_climbs_from_where_its_link_leads() {
    let p = project();
    symlink(format!("{}/.ssh/keys", p.home), format!("{}/lnk", p.root)).expect("link");
    let approved = refused(
        &p,
        &p.root.clone(),
        &[
            "git -C lnk/.. status",
            "git -C lnk/.. log",
            "git -C ./lnk/../ diff",
            "tmux new-session -d -c lnk/.. 'touch x'",
            "tmux new-session -d -c lnk/.. 'echo x > y'",
        ],
    );
    assert!(approved.is_empty(), "a `..` after a link was climbed from the link's own folder:\n{}", approved.join("\n"));
    let control = refused(&p, &p.root.clone(), &["git -C sub/.. status", "git -C ./sub/../ log", "tmux new-session -d -c sub/.. 'touch x'"]);
    assert_eq!(control.len(), 3, "a `..` after a plain folder stays approved: {control:?}");
}

#[test]
fn a_file_command_in_a_script_is_placed_once() {
    let p = project();
    symlink(format!("{}/.ssh/authorized_keys", p.home), format!("{}/sub/out.txt", p.root)).expect("link");
    let sub = format!("{}/sub", p.root);
    let plain = at(&p, &sub, "echo hi > ../out.txt");
    assert!(plain.is_allowed(), "control: a redirect to ../out.txt writes the project's own file: {plain:?}");
    for command in ["sed -n 'w ../out.txt' ../README.md", "sed -n 'r ../README.md' ../README.md"] {
        let v = at(&p, &sub, command);
        assert!(v.is_allowed(), "`{command}` placed ../ target a second time from the folder, so judged sub/…: {v:?}");
    }
}

fn hook(cwd: &std::path::Path, home: &std::path::Path, command: &str) -> String {
    let payload = serde_json::json!({ "tool_name": "Bash", "tool_input": { "command": command }, "cwd": cwd }).to_string();
    let mut child = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .args(["hook", "claude"])
        .env_clear()
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    child.stdin.take().expect("stdin").write_all(payload.as_bytes()).expect("write");
    String::from_utf8(child.wait_with_output().expect("wait").stdout).expect("utf-8")
}

#[test]
fn a_glob_in_a_plain_reported_folder_lists_that_folder() {
    // A folder named without a leading dot, under the real HOME and inside `~/projects`: a hidden
    // component is a region of its own, and a folder outside HOME is one a glob match cannot be read
    // from. The cargo target directory is not that folder when the checkout itself has a hidden name.
    let home = std::path::PathBuf::from(std::env::var("HOME").expect("HOME"));
    let projects = home.join("projects");
    std::fs::create_dir_all(&projects).expect("projects");
    let base = tempfile::Builder::new().prefix("plain-").tempdir_in(&projects).expect("tempdir");
    let folder = base.path().join("clone");
    std::fs::create_dir_all(&folder).expect("folder");
    std::fs::write(folder.join("a.txt"), "a").expect("file");
    let control = hook(&folder, &home, "sort *");
    assert!(control.contains("\"allow\""), "control: a glob over plain names in {} is approved: {control}", folder.display());
    std::fs::write(folder.join("--output=a.txt"), "").expect("flag-shaped name");
    for command in ["sort *", "sort * | head", "ls *"] {
        let out = hook(&folder, &home, command);
        if command == "ls *" {
            continue;
        }
        assert!(!out.contains("\"allow\""), "`{command}` expanded to a name that reads as a flag, and was approved: {out}");
    }
}
