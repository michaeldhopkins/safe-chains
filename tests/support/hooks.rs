//! Spawning the binary in hook mode, shared by the hook integration tests.
//!
//! Pulled out of `integration_hooks.rs` when the file-length gate went in: that file is pinned, so
//! new code goes elsewhere — and the spawn plumbing was never what that file is about.
//!
//! In `tests/support/` rather than `tests/`, because cargo builds every top-level `tests/*.rs` as
//! its own test binary and a helper module is not a test.

use std::io::Write;
use std::process::{Command, Stdio};

pub fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_safe-chains")
}

/// A bare `$HOME` shared by every `run_hook` test: no `~/.claude/settings.json`, no
/// `~/.config/safe-chains.toml`.
///
/// Without it these tests inherited the ambient home and so measured the config of whoever ran the
/// suite. A single `permissions.allow` entry covering `grep` turned three overreach guards into
/// approvals — `grep -r x /etc` came back "allow" instead of the nudge — so they passed in CI,
/// where HOME is bare, and failed on a developer machine.
pub fn bare_home() -> &'static std::path::Path {
    use std::sync::OnceLock;
    static HOME: OnceLock<tempfile::TempDir> = OnceLock::new();
    HOME.get_or_init(|| tempfile::tempdir().expect("tempdir")).path()
}

/// Run the binary with `args`, piping `stdin_payload` in. Returns (stdout, stderr, exit code).
pub fn run_hook(args: &[&str], stdin_payload: &str) -> (String, String, i32) {
    let mut child = Command::new(binary())
        .args(args)
        .env("HOME", bare_home())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn safe-chains");
    match child.stdin.as_mut().expect("stdin was piped").write_all(stdin_payload.as_bytes()) {
        Ok(()) => {}
        // Short-circuit error paths (e.g. unknown subcommand) exit before reading stdin, which
        // races with our write and surfaces as BrokenPipe. The test is asserting on stdout/exit
        // code, not on a successful stdin handshake — tolerate the race.
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
        Err(e) => panic!("write to safe-chains stdin failed: {e}"),
    }
    let out = child.wait_with_output().expect("wait");
    (String::from_utf8_lossy(&out.stdout).into_owned(), String::from_utf8_lossy(&out.stderr).into_owned(), out.status.code().unwrap_or(-1))
}
