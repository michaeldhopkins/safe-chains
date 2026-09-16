//! Spawning the binary in hook mode, shared by the hook integration tests.
//!
//! Pulled out of `integration_hooks.rs` for the file-length gate: that file is pinned, so new code
//! goes elsewhere — and the spawn plumbing was never what that file is about.
//!
//! In `tests/support/` rather than `tests/`, because cargo builds every top-level `tests/*.rs` as
//! its own test binary and a helper module is not a test.

use std::io::Write;
use std::process::{Command, Stdio};

pub fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_safe-chains")
}

/// Run the binary with `args`, piping `stdin_payload` in. Returns (stdout, stderr, exit code).
pub fn run_hook(args: &[&str], stdin_payload: &str) -> (String, String, i32) {
    let mut child = Command::new(binary())
        .args(args)
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
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}
