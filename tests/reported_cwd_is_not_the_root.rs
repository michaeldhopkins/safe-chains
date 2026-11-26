//! A hook that reports a working directory (and no project root) must not make that directory the
//! folder the command writes into. Claude Code's reported cwd is not where the command runs, so a
//! write is placed in an unknown folder and judged by name: a session reported at `~` or `~/.ssh`
//! never gets `cp` into `authorized_keys` approved, by any spelling.

#[path = "support/isolate_home.rs"]
mod isolate_home;

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

#[test]
fn review_findings_on_reported_cwd_are_all_refused() {
    let base = tempfile::tempdir().expect("tempdir");
    let home = base.path().join("home");
    let ssh = home.join(".ssh");
    let claude = home.join(".claude");
    let config = home.join(".config");
    let gcloud_cfg = home.join(".config/gcloud/configurations");
    let keychains_sub = home.join("Library/Keychains/sub");
    for dir in [&ssh, &claude, &config, &gcloud_cfg, &keychains_sub] {
        std::fs::create_dir_all(dir).expect("mkdir");
    }

    let cases: Vec<(std::path::PathBuf, String)> = vec![
        (ssh.clone(), "cd ~/.ssh && cp /tmp/k authorized_keys".into()),
        (ssh.clone(), format!("cp /tmp/k {}/authorized_keys", ssh.display())),
        (ssh.clone(), format!("echo x > {}/authorized_keys", ssh.display())),
        (claude.clone(), "cd ~/.claude && echo evil > CLAUDE.md".into()),
        (claude.clone(), format!("echo evil > {}/CLAUDE.md", claude.display())),
        (claude.clone(), format!("echo evil > {}/hooks/pre.sh", claude.display())),
        (config.clone(), "cd ~/.config && echo evil > zsh/.zshrc".into()),
        (config.clone(), format!("echo evil > {}/mise/config.toml", config.display())),
        (gcloud_cfg, "cd ~/.config/gcloud/configurations && echo evil > config_default".into()),
        (keychains_sub.clone(), format!("cat {}/login.keychain-db", keychains_sub.display())),
        (home.join("repo/.git"), "echo evil > hooks/pre-commit".into()),
        (home.join("repo/.git"), "echo evil > config".into()),
        (home.join("repo/.git/hooks"), "echo evil > pre-commit".into()),
        (home.join("repo/.jj/repo"), "echo evil > config.toml".into()),
        (home.join("projects/repo/.git"), "echo evil > hooks/pre-commit".into()),
        (home.join("projects/repo/.git"), "echo evil > config".into()),
        (home.join("projects/repo/.git/hooks"), "echo evil > pre-commit".into()),
        (home.join("projects/repo/.jj/repo"), "echo evil > config.toml".into()),
        (home.join(".claude/hooks"), "echo evil > pre.sh".into()),
        (home.join(".claude/skills"), "echo evil > SKILL.md".into()),
        (home.join(".config/zsh"), "echo evil > .zshrc".into()),
        (home.join(".config/systemd/user"), "echo evil > pwn.service".into()),
        (home.join(".config/fish/conf.d"), "echo evil > pwn.fish".into()),
        (home.join(".config/nvim/plugin"), "echo evil > pwn.lua".into()),
        (home.join(".oh-my-zsh/custom"), "echo evil > pwn.zsh".into()),
        (home.join(".cargo/bin"), "cp /tmp/x rustc".into()),
        (home.join("Library/Application Support/Code/User"), "echo evil > settings.json".into()),
        (home.clone(), "mise set FOO=bar".into()),
    ];

    for (cwd, command) in cases {
        std::fs::create_dir_all(&cwd).expect("mkdir case cwd");
        let out = decision(&home, &cwd, &command);
        assert!(!out.contains("\"allow\""), "`{command}` approved with cwd {}: {out}", cwd.display());
    }
}

#[test]
fn known_workspaces_and_user_grants_become_the_root_while_protected_or_outside_dirs_do_not() {
    let base = tempfile::tempdir().expect("tempdir");
    let home = base.path().join("home");
    let granted = base.path().join("granted-work/app");
    std::fs::create_dir_all(home.join(".config")).expect("mkdir .config");
    std::fs::create_dir_all(&granted).expect("mkdir granted");
    std::fs::write(
        home.join(".config/safe-chains.toml"),
        format!(
            "workspace_roots = [\"~/agent-workspaces\", \"~/scripts\", \"~/work/.agents\"]\n[[grant]]\npath = \"{}\"\nread = true\nwrite = true\n",
            granted.display()
        ),
    )
    .expect("write config");

    for valid in [
        home.join("projects/app"),
        home.join(".worktrees/wt-1"),
        home.join("agent-workspaces"),
        home.join("agent-workspaces/agent-1"),
        home.join("scripts"),
        home.join("scripts/job"),
        home.join("work/.agents/agent-1"),
        granted.clone(),
    ] {
        std::fs::create_dir_all(&valid).expect("mkdir valid");
        for cmd in ["echo hi > out.txt", "mise set FOO=bar", "ls", "cargo --version"] {
            let out = decision(&home, &valid, cmd);
            assert!(out.contains("\"allow\""), "`{cmd}` should approve in known workspace {}: {out}", valid.display());
        }
    }

    for rejected in [
        home.clone(),
        home.join(".worktrees"),
        home.join("projects/app/.hidden"),
        home.join(".worktrees/wt-1/.git"),
        home.join("work/.agents"),
        home.join("work/.agents/agent-1/.git"),
        home.join("agent-workspaces/agent-1/.hidden"),
        home.join("scripts/job/.git"),
        home.join("ungranted/app"),
    ] {
        std::fs::create_dir_all(&rejected).expect("mkdir rejected");
        for write_cmd in ["echo hi > out.txt", "mise set FOO=bar"] {
            let out = decision(&home, &rejected, write_cmd);
            assert!(!out.contains("\"allow\""), "`{write_cmd}` must not approve in rejected root {}: {out}", rejected.display());
        }
        let read_out = decision(&home, &rejected, "ls");
        assert!(read_out.contains("\"allow\""), "`ls` should still approve in {}: {read_out}", rejected.display());
    }
}

#[test]
fn a_configured_workspace_root_is_a_root_only_when_the_config_names_it() {
    let base = tempfile::tempdir().expect("tempdir");
    let home = base.path().join("home");
    std::fs::create_dir_all(home.join(".config")).expect("mkdir .config");
    let folders = [home.join("agent-workspaces/agent-1"), home.join("scripts/job"), home.join("work/.agents/agent-1")];
    for folder in &folders {
        std::fs::create_dir_all(folder).expect("mkdir");
        let out = decision(&home, folder, "echo hi > out.txt");
        assert!(!out.contains("\"allow\""), "unnamed root approved a write in {}: {out}", folder.display());
    }
    std::fs::write(
        home.join(".config/safe-chains.toml"),
        "workspace_roots = [\"~/agent-workspaces\", \"~/scripts\", \"~/work/.agents\"]\n",
    )
    .expect("write config");
    for folder in &folders {
        for cmd in ["echo hi > out.txt", "mise set FOO=bar", "ls", "cargo --version"] {
            let out = decision(&home, folder, cmd);
            assert!(out.contains("\"allow\""), "`{cmd}` should approve in configured root {}: {out}", folder.display());
        }
    }
    let parent = home.join("work/.agents");
    std::fs::create_dir_all(&parent).expect("mkdir parent");
    let parent_out = decision(&home, &parent, "echo hi > out.txt");
    assert!(!parent_out.contains("\"allow\""), "the dotted root itself approved a write: {parent_out}");
    let hidden = home.join("scripts/job/.git");
    std::fs::create_dir_all(&hidden).expect("mkdir hidden");
    let hidden_out = decision(&home, &hidden, "echo hi > out.txt");
    assert!(!hidden_out.contains("\"allow\""), "a dot component below the root approved a write: {hidden_out}");
}

#[test]
fn round_four_unknown_root_and_readonly_grant_findings_are_all_refused() {
    let base = tempfile::tempdir().expect("tempdir");
    let tmp_base = tempfile::tempdir_in("/tmp").expect("tempdir in /tmp");
    let home = base.path().join("home");
    let ssh = home.join(".ssh");
    let downloads = home.join("Downloads");
    let claude_hooks = home.join(".claude/hooks");
    let repo_git = home.join("projects/repo/.git");
    let evil_dir = tmp_base.path().join("evil-dir");
    let ro_grant = home.join("ro-vendor");
    let rw_grant = home.join("rw-vendor");
    let dot_under_grant = rw_grant.join(".secret/sub");
    for dir in [&ssh, &downloads, &claude_hooks, &repo_git, &evil_dir, &ro_grant, &dot_under_grant, &home.join(".config")] {
        std::fs::create_dir_all(dir).expect("mkdir");
    }
    std::fs::write(ssh.join("id_rsa"), "secret-key").expect("write id_rsa");
    std::fs::write(home.join("Gemfile"), "source 'https://rubygems.org'").expect("write Gemfile");
    std::os::unix::fs::symlink(&ssh, evil_dir.join("leak")).expect("symlink to .ssh");
    std::fs::write(
        home.join(".config/safe-chains.toml"),
        format!(
            "[[grant]]\npath = \"{}\"\nread = true\nwrite = false\n\n[[grant]]\npath = \"{}\"\nread = true\nwrite = true\n",
            ro_grant.display(),
            rw_grant.display()
        ),
    )
    .expect("write config");

    let cases: Vec<(&str, std::path::PathBuf, String)> = vec![
        ("4.A", evil_dir.clone(), format!("find -L {} -exec cat {{}} +", evil_dir.display())),
        ("4.A", evil_dir.clone(), format!("cp -RL {} {}/out", evil_dir.display(), tmp_base.path().display())),
        ("4.A", evil_dir.clone(), "find -L . -exec cat {} +".into()),
        ("4.A", evil_dir.clone(), "rg -L needle .".into()),
        ("4.A", evil_dir.clone(), "tar -czhf - .".into()),
        ("4.B", evil_dir.clone(), format!("git -C {} status", evil_dir.display())),
        ("4.B", evil_dir.clone(), "git status".into()),
        ("4.B", evil_dir.clone(), format!("cd {} && hg status", evil_dir.display())),
        ("4.B", evil_dir.clone(), "bundle list".into()),
        ("4.B", home.clone(), "bundle list".into()),
        ("4.B", downloads.clone(), "bundle list".into()),
        ("4.C", evil_dir.clone(), "pytest".into()),
        ("4.C", evil_dir.clone(), "npm test".into()),
        ("4.C", evil_dir.clone(), "cargo test".into()),
        ("4.C", evil_dir, "cargo --version".into()),
        ("4.C", ssh, "pytest".into()),
        ("4.C", downloads, "npm test".into()),
        ("4.C", repo_git, "cargo test".into()),
        ("4.D", ro_grant.clone(), "echo evil > file.txt".into()),
        ("4.D", ro_grant, "rm -rf sub".into()),
        ("4.D", dot_under_grant, "echo evil > file.txt".into()),
        ("2.F", claude_hooks, "cd ../.. && mkdir -p tmp && echo evil > tmp/pwn.sh".into()),
    ];

    let mut approved = Vec::new();
    for (tag, cwd, command) in cases {
        let out = decision(&home, &cwd, &command);
        if out.contains("\"allow\"") {
            approved.push(format!("{tag}: `{command}` approved with cwd {}", cwd.display()));
        }
    }
    assert!(approved.is_empty(), "findings still approved:\n{}", approved.join("\n"));
}

#[test]
fn a_hook_that_reports_no_folder_judges_the_command_in_an_unknown_one() {
    let base = tempfile::tempdir().expect("tempdir");
    let home = base.path().join("home");
    std::fs::create_dir_all(&home).expect("mkdir");
    let run = |command: &str| {
        let payload = serde_json::json!({ "tool_name": "Bash", "tool_input": { "command": command } }).to_string();
        let mut child = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
            .args(["hook", "claude"])
            .current_dir(&home)
            .env_clear()
            .env("HOME", &home)
            .env("PATH", "/usr/bin:/bin")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn");
        child.stdin.take().expect("stdin").write_all(payload.as_bytes()).expect("write");
        String::from_utf8(child.wait_with_output().expect("wait").stdout).expect("utf-8")
    };
    assert!(run("ls").contains("\"allow\""), "a read is still approved");
    let mut approved = Vec::new();
    for command in ["pytest", "cargo test", "git status", "bundle list", "echo evil > out.txt", "mise set FOO=bar"] {
        if run(command).contains("\"allow\"") {
            approved.push(command);
        }
    }
    assert!(approved.is_empty(), "approved with no reported folder:\n{}", approved.join("\n"));
}

/// A folder that is not a known workspace is judged as an unknown one, but the command still runs
/// in the real folder the hook reported. A link there is followed as the kernel would follow it, so
/// a file committed as a link to `~/.ssh/id_rsa` is read as the key, not as a file of the folder.
#[test]
fn a_link_in_a_reported_folder_that_is_not_a_workspace_is_followed() {
    let base = tempfile::tempdir().expect("tempdir");
    let home = base.path().join("home");
    let ssh = home.join(".ssh");
    let folder = home.join("Downloads/clone");
    for dir in [&ssh, &folder] {
        std::fs::create_dir_all(dir).expect("mkdir");
    }
    std::fs::write(ssh.join("id_rsa"), "secret-key").expect("write id_rsa");
    std::fs::write(folder.join("plain.txt"), "words").expect("write plain");
    std::os::unix::fs::symlink(ssh.join("id_rsa"), folder.join("notes.txt")).expect("link to the key");
    std::os::unix::fs::symlink(&ssh, folder.join("leak")).expect("link to .ssh");
    std::os::unix::fs::symlink(ssh.join("not-yet"), folder.join("dangling")).expect("dangling link into .ssh");
    std::fs::create_dir_all(folder.join("sub")).expect("mkdir sub");
    std::os::unix::fs::symlink(folder.join("plain.txt"), folder.join("alias.txt")).expect("link inside the folder");

    for read in ["cat plain.txt", "ls", "cat alias.txt"] {
        let out = decision(&home, &folder, read);
        assert!(out.contains("\"allow\""), "`{read}` is an ordinary read and stays approved: {out}");
    }
    let mut approved = Vec::new();
    for command in [
        "cat notes.txt", "cat ./notes.txt", "wc -c notes.txt", "base64 notes.txt", "cp notes.txt /tmp/out", "cat leak/id_rsa",
        "cat dangling", "cat sub/../notes.txt", "cd sub && cat ../notes.txt", "cd leak && cat id_rsa", "cd leak && cp id_rsa /tmp/out",
        "echo x >> notes.txt",
    ] {
        if decision(&home, &folder, command).contains("\"allow\"") {
            approved.push(command);
        }
    }
    assert!(approved.is_empty(), "read through a link out of the folder:\n{}", approved.join("\n"));
}

fn decision_as(harness: &str, home: &std::path::Path, cwd: &std::path::Path, command: &str) -> String {
    let payload = match harness {
        "grok" => serde_json::json!({ "toolName": "run_terminal_command", "toolInput": { "command": command }, "cwd": cwd, "workspaceRoot": cwd }),
        "antigravity" => serde_json::json!({ "toolCall": { "name": "run_command", "args": { "CommandLine": command, "Cwd": cwd } }, "workspacePaths": [cwd] }),
        "codex" => serde_json::json!({ "hook_event_name": "PermissionRequest", "tool_name": "Bash", "tool_input": { "command": command }, "cwd": cwd }),
        _ => serde_json::json!({ "tool_name": "Bash", "tool_input": { "command": command }, "cwd": cwd }),
    }
    .to_string();
    let mut child = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .args(["hook", harness])
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

const HARNESSES: [&str; 4] = ["claude", "grok", "antigravity", "codex"];

/// The reported folder can itself be a link: a session that ran `cd keys`, where `keys` leads to
/// `~/.ssh`, reports the folder by the link's name. Every relative read there lands in `~/.ssh`.
#[test]
fn a_reported_folder_that_is_a_link_to_a_credential_store_reads_as_that_store() {
    let base = tempfile::tempdir().expect("tempdir");
    let home = base.path().join("home");
    let ssh = home.join(".ssh");
    let outside = base.path().join("outside");
    for dir in [&ssh, &outside] {
        std::fs::create_dir_all(dir).expect("mkdir");
    }
    std::fs::write(ssh.join("config"), "Host x").expect("write config");
    std::fs::write(ssh.join("id_rsa"), "secret-key").expect("write id_rsa");
    let keys = outside.join("keys");
    std::os::unix::fs::symlink(&ssh, &keys).expect("link to .ssh");
    let through_home = outside.join("h");
    std::os::unix::fs::symlink(&home, &through_home).expect("link to home");

    let mut approved = Vec::new();
    for harness in HARNESSES {
        for (cwd, command) in [
            (keys.clone(), "cat config"),
            (keys.clone(), "cat ./config"),
            (keys.clone(), "grep -r x"),
            (keys.clone(), "grep -r x ."),
            (keys.clone(), "head *"),
            (keys.join("."), "cat config"),
            (through_home.join(".ssh"), "cat config"),
            (through_home.clone(), "cat .ssh/config"),
            (through_home.clone(), "cd .ssh && cat config"),
        ] {
            if decision_as(harness, &home, &cwd, command).contains("\"allow\"") {
                approved.push(format!("{harness}: `{command}` in {}", cwd.display()));
            }
        }
    }
    assert!(approved.is_empty(), "read a credential store through the reported folder:\n{}", approved.join("\n"));
}

/// A scratch folder outside every workspace, such as one under `/tmp`, is a real place on disk. A
/// relative path there is judged as the absolute path it names, so ordinary work in it approves as it
/// would when spelled out, while a link out of it, its repository metadata and its own code are still
/// judged where they lead.
#[test]
fn a_scratch_folder_outside_a_workspace_is_judged_as_the_place_it_is() {
    let home_base = tempfile::tempdir().expect("home tempdir");
    let home = home_base.path().join("home");
    let ssh = home.join(".ssh");
    std::fs::create_dir_all(&ssh).expect("mkdir .ssh");
    std::fs::write(ssh.join("id_rsa"), "secret-key").expect("write id_rsa");
    let scratch = tempfile::Builder::new().prefix("sc-scratch").tempdir_in("/tmp").expect("scratch under /tmp");
    let folder = std::path::PathBuf::from("/tmp").join(scratch.path().file_name().expect("name"));
    for dir in ["sub", ".git/hooks"] {
        std::fs::create_dir_all(folder.join(dir)).expect("mkdir");
    }
    for file in ["a.txt", "x.txt", "file.txt"] {
        std::fs::write(folder.join(file), "a").expect("write");
    }
    std::os::unix::fs::symlink(ssh.join("id_rsa"), folder.join("key2")).expect("link to a key");
    std::os::unix::fs::symlink(&ssh, folder.join("leak")).expect("link to .ssh");

    let mut refused = Vec::new();
    let mut approved = Vec::new();
    for harness in ["claude", "grok", "antigravity"] {
        for command in [
            "touch new.txt", "mkdir -p sub/dir", "rm -f x.txt", "cp a.txt b.txt", "mv a.txt b.txt", "echo hi > out.txt",
            "sed -i '' s/a/b/ file.txt", "cd sub && touch y.txt", "ls *.txt", "cat x.txt",
        ] {
            if !decision_as(harness, &home, &folder, command).contains("\"allow\"") {
                refused.push(format!("{harness}: `{command}`"));
            }
        }
        for command in [
            "cat key2", "cat leak/id_rsa", "cd leak && cat id_rsa", "echo k > leak/authorized_keys", "cd leak && echo k > authorized_keys",
            "cp key2 copy.txt", "echo x > .git/hooks/pre-commit", "cd .git/hooks && echo x > pre-commit", "rm -rf .git", "git status",
            "jj log", "cargo build", "cargo test", "npm test", "bundle exec rake", "mise set FOO=bar", "cat ~/.ssh/id_rsa",
        ] {
            if decision_as(harness, &home, &folder, command).contains("\"allow\"") {
                approved.push(format!("{harness}: `{command}`"));
            }
        }
    }
    for command in ["cat key2", "echo k > leak/authorized_keys", "echo x > .git/hooks/pre-commit"] {
        if decision_as("codex", &home, &folder, command).contains("\"allow\"") {
            approved.push(format!("codex: `{command}`"));
        }
    }
    assert!(approved.is_empty(), "approved in a scratch folder:\n{}", approved.join("\n"));
    assert!(refused.is_empty(), "ordinary work refused in a scratch folder:\n{}", refused.join("\n"));
}
