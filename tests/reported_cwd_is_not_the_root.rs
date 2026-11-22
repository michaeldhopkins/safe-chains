//! A hook that reports a working directory (and no project root) must not make that directory the
//! folder the command writes into. Claude Code's reported cwd is not where the command runs, so a
//! write is placed in an unknown folder and judged by name: a session reported at `~` or `~/.ssh`
//! never gets `cp` into `authorized_keys` approved, by any spelling.

use std::io::Write;
use std::process::{Command, Stdio};

fn decision(home: &std::path::Path, cwd: &std::path::Path, command: &str) -> String {
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
fn a_reported_home_or_ssh_folder_never_approves_a_write_to_authorized_keys() {
    let base = tempfile::tempdir().expect("tempdir");
    let home = base.path().join("home");
    let ssh = home.join(".ssh");
    std::fs::create_dir_all(&ssh).expect("mkdir");
    let control = decision(&home, &home, "ls");
    assert!(control.contains("\"allow\""), "a read is still approved: {control}");
    for cwd in [&home, &ssh] {
        for command in [
            "cp /tmp/k ~/.ssh/authorized_keys", "cp ./k ~/.ssh/authorized_keys", "cp ./k authorized_keys", "cp ./k .ssh/authorized_keys",
            "cp ./k out.txt", "echo x > authorized_keys", "cd .ssh && cp ./k authorized_keys", "time cd .ssh && cp ./k authorized_keys",
        ] {
            let out = decision(&home, cwd, command);
            assert!(!out.contains("\"allow\""), "`{command}` approved with cwd {}: {out}", cwd.display());
        }
    }
}
