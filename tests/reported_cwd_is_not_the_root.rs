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
        for cmd in ["echo hi > out.txt", "mise set FOO=bar", "ls"] {
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
