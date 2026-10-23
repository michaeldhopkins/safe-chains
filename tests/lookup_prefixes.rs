//! macOS resolves `/.nofollow/p` and `/.resolve/<flags>/p` as `/p`, so each prefix is judged as the
//! path after it.

#![cfg(target_os = "macos")]

use safe_chains::pathctx::PathCtx;
use safe_chains::{Verdict, command_verdict_in};

fn at(cmd: &str) -> Verdict {
    command_verdict_in(
        cmd,
        PathCtx { cwd: Some("/work/proj".into()), root: Some("/work/proj".into()), session_id: None },
    )
}

fn paths() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/me".into());
    let mut out: Vec<String> = [
        ".claude.json",
        ".codex/auth.json",
        "Library/Keychains/login.keychain-db",
        ".s3cfg",
        ".aws/credentials",
        ".zshrc",
        "notes.txt",
    ]
    .iter()
    .map(|p| format!("{home}/{p}"))
    .collect();
    out.extend(
        ["/.vol/1/2", "/Users/someone/notes.txt", "/private/etc/master.passwd", "/etc/hosts", "/usr/share/dict/words", "/work/proj/README.md"]
            .map(String::from),
    );
    out
}

#[test]
fn a_lookup_prefix_reads_as_the_path_after_it() {
    for path in paths() {
        let plain = at(&format!("cat {path}")).is_allowed();
        for prefix in ["/.nofollow", "/.resolve/1", "/.resolve/0", "/.nofollow/.nofollow", "/.resolve/1/.nofollow", "//.nofollow/."] {
            let cmd = format!("cat {prefix}{path}");
            assert_eq!(at(&cmd).is_allowed(), plain, "{cmd} differs from cat {path}");
        }
    }
}

#[test]
fn a_credential_store_behind_a_lookup_prefix_is_refused() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/me".into());
    for cmd in [
        format!("cat /.nofollow{home}/.claude.json"),
        format!("cat /.resolve/1{home}/.codex/auth.json"),
        format!("cp /.nofollow{home}/.claude.json ./x"),
        "cat /.nofollow/.vol/1/2".to_string(),
        "cat /.nofollow/Users/someone/notes.txt".to_string(),
    ] {
        assert!(!at(&cmd).is_allowed(), "{cmd} was approved");
    }
    assert!(at("cat /work/proj/README.md").is_allowed());
}
