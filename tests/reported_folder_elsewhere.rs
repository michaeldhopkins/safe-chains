//! A harness whose reported folder is not where its command runs must never let that folder decide
//! an approval. Codex reports its session cwd while `exec_command` runs in a `workdir` it leaves out;
//! Antigravity reports `workspacePaths` while `run_command` runs in `toolCall.args.Cwd`.

#[path = "support/isolate_home.rs"]
mod isolate_home;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn decide(harness: &str, home: &Path, cwd: &Path, payload: &serde_json::Value) -> bool {
    decide_with(harness, home, cwd, payload, &[])
}

fn decide_with(harness: &str, home: &Path, cwd: &Path, payload: &serde_json::Value, env: &[(&str, &Path)]) -> bool {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_safe-chains"));
    cmd.args(["hook", harness])
        .current_dir(cwd)
        .env_clear()
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for (key, value) in env {
        cmd.env(key, value);
    }
    let mut child = cmd.spawn().expect("spawn");
    child.stdin.take().expect("stdin").write_all(payload.to_string().as_bytes()).expect("write");
    String::from_utf8(child.wait_with_output().expect("wait").stdout)
        .expect("utf-8")
        .contains("\"allow\"")
}

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let base = tempfile::tempdir_in("/Users/Shared").expect("tempdir under a non-temp folder");
    let home = base.path().join("home");
    let proj = home.join("projects/proj");
    std::fs::create_dir_all(proj.join(".git")).expect("mkdir");
    std::fs::create_dir_all(proj.join("sub")).expect("mkdir");
    std::fs::create_dir_all(home.join("Documents")).expect("mkdir");
    std::fs::create_dir_all(home.join(".config")).expect("mkdir");
    std::fs::write(home.join(".config/safe-chains.toml"), "").expect("config");
    std::fs::write(home.join("Documents/tax.pdf"), "private").expect("file");
    std::fs::write(home.join(".zsh_history"), "history").expect("file");
    (base, home, proj)
}

/// A link committed in the reported folder (`src -> /usr/share/man/man1`) makes a path that climbs
/// out of the folder land somewhere harmless there. Codex runs the command in its `workdir`, where
/// `src` is an ordinary folder and the same spelling reaches `~/Documents`. Without the link the
/// climb is placed nowhere and prompts, so the reported folder alone turned a prompt into an allow.
#[test]
fn a_link_in_codex_reported_folder_does_not_place_a_climb_out_of_the_unknown_folder() {
    let (_base, home, proj) = fixture();
    std::os::unix::fs::symlink("/usr/share/man/man1", proj.join("src")).expect("link");
    let codex = |c: &str| {
        let payload = serde_json::json!({
            "hook_event_name": "PermissionRequest",
            "tool_name": "Bash",
            "tool_input": { "command": c },
            "cwd": proj,
        });
        decide("codex", &home, &proj, &payload)
    };
    assert!(!codex("cat sub/../../../Documents/tax.pdf"), "control: a climb out of the unknown folder prompts");
    let approved: Vec<&str> =
        ["cat src/../../../Documents/tax.pdf", "cd src && cat ../../../Documents/tax.pdf", "cat src/../../../.zsh_history"]
            .into_iter()
            .filter(|c| codex(c))
            .collect();
    assert!(approved.is_empty(), "a link in codex's reported folder placed a climb out of the unknown folder:\n{}", approved.join("\n"));
}

/// Antigravity runs `run_command` in `toolCall.args.Cwd`, not in `workspacePaths[0]`, so a relative
/// path judged in the workspace may name a file of another folder (here, home).
#[test]
fn antigravity_judges_a_command_where_its_cwd_argument_runs_it() {
    let (_base, home, proj) = fixture();
    let agy = |c: &str, cwd: &Path| {
        let payload = serde_json::json!({
            "toolCall": { "name": "run_command", "args": { "CommandLine": c, "Cwd": cwd } },
            "workspacePaths": [proj],
        });
        decide("antigravity", &home, &proj, &payload)
    };
    assert!(agy("rm -rf sub", &proj), "control: a delete in the workspace it runs in is approved");
    let approved: Vec<&str> = ["rm -rf Documents", "echo x > .zshrc"].into_iter().filter(|c| agy(c, &home)).collect();
    assert!(approved.is_empty(), "antigravity's workspace decided a command that runs in home:\n{}", approved.join("\n"));
}

/// Gemini CLI's `run_shell_command` takes `dir_path` (absolute, or relative to the workspace root)
/// and Qwen's takes `directory`; the command runs there, not in the reported cwd. A name the hook
/// cannot place leaves the folder unknown, so a relative path is never judged in the workspace.
#[test]
fn gemini_and_qwen_judge_a_command_where_its_directory_argument_runs_it() {
    let (_base, home, proj) = fixture();
    for (harness, field) in [("gemini", "dir_path"), ("qwen", "directory")] {
        let run = |c: &str, dir: Option<&Path>| {
            let mut input = serde_json::json!({ "command": c });
            if let Some(dir) = dir {
                input[field] = serde_json::json!(dir);
            }
            let payload = serde_json::json!({ "tool_name": "run_shell_command", "tool_input": input, "cwd": proj });
            decide(harness, &home, &proj, &payload)
        };
        assert!(run("rm -rf sub", None), "{harness} control: a delete in the reported folder is approved");
        assert!(run("rm -rf sub", Some(&proj)), "{harness} control: a named folder that is the workspace is approved");
        let approved: Vec<&str> = ["rm -rf Documents", "echo x > .zshrc"]
            .into_iter()
            .filter(|c| run(c, Some(&home)) || run(c, Some(Path::new("../.."))))
            .collect();
        assert!(approved.is_empty(), "{harness} judged a command in the reported folder that runs elsewhere:\n{}", approved.join("\n"));
    }
}

/// A directory the model names (`Cwd`, `dir_path`, `directory`) is where the command runs. It is
/// not the project. When the harness workspace is outside the known workspace folders, or is home
/// itself, `rm -rf src` in `~/projects/other` prompts. The same delete inside a valid workspace
/// is still approved.
#[test]
fn a_declared_directory_never_becomes_the_project_root() {
    let (base, home, proj) = fixture();
    let other = home.join("projects/other");
    std::fs::create_dir_all(other.join("src")).expect("mkdir");
    let elsewhere = base.path().join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("mkdir");

    let agy = |command: &str, cwd: &Path, workspace: &Path| {
        let payload = serde_json::json!({
            "toolCall": { "name": "run_command", "args": { "CommandLine": command, "Cwd": cwd } },
            "workspacePaths": [workspace],
        });
        decide("antigravity", &home, workspace, &payload)
    };
    assert!(agy("rm -rf sub", &proj, &proj), "control: a delete inside the valid workspace is approved");
    assert!(!agy("rm -rf src", &other, &elsewhere), "antigravity: a declared Cwd became the project root");

    for (harness, field, var) in [("gemini", "dir_path", "GEMINI_PROJECT_DIR"), ("qwen", "directory", "QWEN_PROJECT_DIR")] {
        let run = |command: &str, dir: &Path, project: &Path| {
            let payload = serde_json::json!({
                "tool_name": "run_shell_command",
                "tool_input": { "command": command, field: dir },
                "cwd": project,
            });
            decide_with(harness, &home, project, &payload, &[(var, project)])
        };
        assert!(run("rm -rf sub", &proj, &proj), "{harness} control: a delete inside the valid workspace is approved");
        for project in [&elsewhere, &home] {
            assert!(!run("rm -rf src", &other, project), "{harness}: a declared directory became the project root ({})", project.display());
        }
    }
}

/// A named directory that is itself a valid workspace is still not the project. Gemini's
/// `GEMINI_PROJECT_DIR` stays the project, and a relative command in the other workspace prompts.
#[test]
fn a_declared_directory_that_is_its_own_workspace_does_not_become_the_project() {
    let (_base, home, _proj) = fixture();
    let app = home.join("projects/app");
    let other = home.join("projects/other");
    std::fs::create_dir_all(app.join("src")).expect("app");
    std::fs::create_dir_all(app.join(".git")).expect("git");
    std::fs::create_dir_all(other.join("src")).expect("other");
    std::fs::create_dir_all(other.join(".git")).expect("git");
    let run = |command: &str| {
        let payload = serde_json::json!({
            "tool_name": "run_shell_command",
            "tool_input": { "command": command, "dir_path": &other },
            "cwd": &app,
        });
        decide_with("gemini", &home, &app, &payload, &[("GEMINI_PROJECT_DIR", app.as_path())])
    };
    assert!(!run("rm -rf src"), "rm -rf src in another workspace was approved");
    assert!(!run("cargo build"), "cargo build in another workspace was approved");
}
