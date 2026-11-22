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
        format!("[[grant]]\npath = \"{}\"\nread = true\nwrite = true\n", granted.display()),
    )
    .expect("write config");

    for valid in [home.join("projects/app"), home.join(".worktrees/wt-1"), granted.clone()] {
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
        base.path().join("ungranted/app"),
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
fn round_four_unknown_root_and_readonly_grant_findings_are_all_refused() {
    let base = tempfile::tempdir().expect("tempdir");
    let tmp_base = tempfile::tempdir_in("/tmp").expect("tempdir in /tmp");
    let home = base.path().join("home");
    let ssh = home.join(".ssh");
    let downloads = home.join("Downloads");
    let claude_hooks = home.join(".claude/hooks");
    let repo_git = home.join("projects/repo/.git");
    let evil_dir = tmp_base.path().join("evil-dir");
    let ro_grant = base.path().join("ro-vendor");
    let rw_grant = base.path().join("rw-vendor");
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
