//! Codex's reported cwd is not where its command runs, so a command there is judged in an unknown
//! folder. A directory option (`tmux -c .`, `git -C .`, `find -execdir`) and a glob (`ls *`) name that
//! same folder and must be judged the same way: neither may turn the reported cwd into a known one.

use std::io::Write;
use std::process::{Command, Stdio};

fn approves(home: &std::path::Path, cwd: &std::path::Path, command: &str) -> bool {
    let payload = serde_json::json!({
        "hook_event_name": "PermissionRequest",
        "tool_name": "Bash",
        "tool_input": { "command": command },
        "cwd": cwd,
    })
    .to_string();
    let mut child = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .args(["hook", "codex"])
        .current_dir(cwd)
        .env_clear()
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");
    child.stdin.take().expect("stdin").write_all(payload.as_bytes()).expect("write");
    String::from_utf8(child.wait_with_output().expect("wait").stdout)
        .expect("utf-8")
        .contains("\"allow\"")
}

#[test]
fn a_directory_option_on_codex_reported_cwd_is_judged_like_the_plain_command() {
    let base = tempfile::tempdir_in("/Users/Shared").expect("tempdir under a non-temp folder");
    let home = base.path().join("home");
    let proj = home.join("projects/proj");
    std::fs::create_dir_all(proj.join(".git")).expect("mkdir");
    std::fs::create_dir_all(proj.join("sub")).expect("mkdir");
    std::fs::create_dir_all(home.join(".config")).expect("mkdir");
    std::fs::write(home.join(".config/safe-chains.toml"), "").expect("config");
    assert!(!approves(&home, &proj, "rm -rf Documents"), "control: codex's reported cwd is not trusted for a recursive delete");
    let approved: Vec<&str> = [
        r#"tmux new-window -c . "rm -rf Documents""#,
        r#"tmux new-session -d -c . "rm -rf Documents""#,
        r#"tmux new-window -c sub "rm -rf ../Documents""#,
    ]
    .into_iter()
    .filter(|c| approves(&home, &proj, c))
    .collect();
    assert!(approved.is_empty(), "codex's reported cwd became a known folder through a directory option:\n{}", approved.join("\n"));
}

fn fixture(project: bool) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let base = tempfile::tempdir_in("/Users/Shared").expect("tempdir under a non-temp folder");
    let home = base.path().join("home");
    let folder = home.join("projects/proj");
    std::fs::create_dir_all(folder.join("sub")).expect("mkdir");
    if project {
        std::fs::create_dir_all(folder.join(".git")).expect("mkdir");
    }
    std::fs::create_dir_all(home.join("Documents")).expect("mkdir");
    std::fs::create_dir_all(home.join(".config")).expect("mkdir");
    std::fs::write(home.join(".config/safe-chains.toml"), "").expect("config");
    std::fs::write(folder.join("a.txt"), "a").expect("file");
    std::fs::write(folder.join("sub/b.txt"), "b").expect("file");
    (base, home, folder)
}

#[test]
fn a_glob_read_on_codex_reported_cwd_is_not_listed_there() {
    for project in [true, false] {
        let (_base, home, folder) = fixture(project);
        for dir in [folder.clone(), folder.join("sub")] {
            let approved: Vec<&str> = ["ls *", "ls -la *", "file *", "du -sh *", "stat *"]
                .into_iter()
                .filter(|c| approves(&home, &dir, c))
                .collect();
            assert!(approved.is_empty(), "a glob was listed in codex's reported folder ({}):\n{}", dir.display(), approved.join("\n"));
        }
    }
}

#[test]
fn a_directory_option_on_codex_reported_plain_folder_is_judged_like_the_plain_command() {
    let (_base, home, folder) = fixture(false);
    let approved: Vec<&str> = [
        r#"tmux new-window -c . "rm -rf Documents""#,
        r#"tmux new-window -c . "rm -rf *""#,
        r#"tmux new-window -c sub "rm -rf ../Documents""#,
        r#"find . -execdir rm -rf Documents ;"#,
    ]
    .into_iter()
    .filter(|c| approves(&home, &folder, c))
    .collect();
    assert!(approved.is_empty(), "codex's reported plain folder became a known folder:\n{}", approved.join("\n"));
}

#[test]
fn the_claude_harness_still_knows_its_folder() {
    let (_base, home, folder) = fixture(false);
    let claude = |c: &str| {
        let payload = serde_json::json!({"tool_name":"Bash","tool_input":{"command":c},"cwd":folder,"session_id":"x"}).to_string();
        let mut child = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
            .args(["hook", "claude"])
            .current_dir(&folder)
            .env_clear()
            .env("HOME", &home)
            .env("PATH", "/usr/bin:/bin")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        child.stdin.take().expect("stdin").write_all(payload.as_bytes()).expect("write");
        String::from_utf8(child.wait_with_output().expect("wait").stdout)
            .expect("utf-8")
            .contains("\"allow\"")
    };
    assert!(claude("ls *"), "control: a glob in claude's own folder is listed there");
}
