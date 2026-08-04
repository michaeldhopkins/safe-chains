//! `--suggest` writes a config, and WHICH file it writes is a trust question.
//!
//! Its output asks the reader for a trust decision — "add this pin to ~/.config/safe-chains.toml" —
//! so the file it offers has to be one the reader can predict from the command they typed. Two ways
//! that went wrong, one fixed here and one already fixed but ungarded:
//!
//! 1. The search for an existing `.safe-chains.toml` walked to the filesystem root, so a config in
//!    any ancestor captured the write. With a `~/.safe-chains.toml` present, every `--suggest` run
//!    anywhere beneath `$HOME` rewrote that file instead of a project one.
//! 2. Merging into a file that is not valid TOML produces a file that is still not valid TOML, which
//!    safe-chains can never load — so reporting success and handing over a pin for it sends the
//!    reader off to approve something that cannot work.
//!
//! Integration rather than unit tests because both are about what the PROCESS does to the
//! filesystem, and (1) is decided from the real `current_dir`.
use std::process::Command;

/// A command no registry entry can match, so `--suggest` reaches its generating path.
const UNKNOWN: &str = "frobnicatezzz --wibble";

fn suggest_in(dir: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .arg("--suggest")
        .arg(UNKNOWN)
        .current_dir(dir)
        .output()
        .expect("spawn safe-chains")
}

/// An ancestor config outside the project must be left alone: the write belongs to the project whose
/// commands were analysed, not to whatever `.safe-chains.toml` happens to sit above it.
#[test]
fn suggest_never_writes_a_config_above_the_project_root() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let outside = tmp.path().join("outside.toml.marker");
    let ancestor_cfg = tmp.path().join(".safe-chains.toml");
    std::fs::write(&ancestor_cfg, "# ancestor, must not be touched\n").expect("write ancestor");
    std::fs::write(&outside, "x").expect("marker");

    let proj = tmp.path().join("proj");
    let sub = proj.join("a").join("b");
    std::fs::create_dir_all(&sub).expect("mkdir");
    std::fs::create_dir_all(proj.join(".git")).expect("mkdir .git");

    let before = std::fs::read(&ancestor_cfg).expect("read ancestor");
    let out = suggest_in(&sub);
    let after = std::fs::read(&ancestor_cfg).expect("read ancestor after");

    assert_eq!(
        before,
        after,
        "--suggest rewrote a config ABOVE the project root; stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        proj.join(".safe-chains.toml").is_file(),
        "expected the config to be created at the project root; stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A project config that is not valid TOML must be refused and left byte-identical — never merged
/// into and never pinned.
#[test]
fn suggest_leaves_an_unparseable_config_byte_identical() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let proj = tmp.path().join("proj");
    std::fs::create_dir_all(proj.join(".git")).expect("mkdir .git");
    let cfg = proj.join(".safe-chains.toml");
    const MALFORMED: &[u8] = b"[[custom]\nname = \"unclosed\n";
    std::fs::write(&cfg, MALFORMED).expect("write malformed");

    let out = suggest_in(&proj);

    assert_eq!(
        std::fs::read(&cfg).expect("read back"),
        MALFORMED,
        "--suggest modified a config it could not parse"
    );
    assert!(
        !out.status.success(),
        "--suggest reported success over an unparseable config; stdout={}",
        String::from_utf8_lossy(&out.stdout)
    );
}
