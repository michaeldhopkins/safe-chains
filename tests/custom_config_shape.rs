//! A user config that is well-formed TOML but is not a command definition must be REPORTED, not
//! panicked over.
//!
//! `load_toml` REPORTS every way a definition can be invalid instead of panicking on it, so a bad
//! config is skipped with a message and nothing is printed that looks like a crash.
//!
//! This is the property the `config_load` fuzz target found missing. Loading used to panic — on a
//! syntax error, on a document that is valid TOML but not a command definition (`[[command]]` with
//! no `name`), and on ~40 content assertions. Downstream `catch_unwind` caught all of it and the
//! classifier stayed correct, but catching cannot undo what panicking already did: the default hook
//! printed a panic and a backtrace note to stderr on EVERY invocation, naming an internal file and
//! line, for what is an ordinary typo — and under libFuzzer a caught panic is still an abort.
//!
//! An integration test because the property is about what the PROCESS prints and what exit status it
//! leaves; a unit test cannot see either.
use std::process::Command;

/// Well-formed TOML, each failing to BE a command definition in a different way.
const SHAPE_INVALID: &[(&str, &str)] = &[
    ("missing name", "[[command]]\nlevel = \"Inert\"\n"),
    ("name of the wrong type", "[[command]]\nname = 42\n"),
    ("sub with no name", "[[command]]\nname = \"x\"\n[[command.sub]]\nlevel = \"Inert\"\n"),
];

fn run_with_config(source: &str) -> std::process::Output {
    let home = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(home.path().join(".config")).expect("mkdir .config");
    std::fs::write(home.path().join(".config/safe-chains.toml"), source).expect("write config");
    let work = home.path().join("wk");
    std::fs::create_dir_all(&work).expect("mkdir wk");
    Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .arg("--root")
        .arg(&work)
        .arg("--cwd")
        .arg(&work)
        .arg("ls")
        .env("HOME", home.path())
        .output()
        .expect("run safe-chains")
}

#[test]
fn a_shape_invalid_config_is_reported_without_panicking() {
    for (what, source) in SHAPE_INVALID {
        let out = run_with_config(source);
        let err = String::from_utf8_lossy(&out.stderr);

        assert!(
            !err.contains("panicked at"),
            "{what}: a malformed user config printed a panic to stderr:\n{err}"
        );
        assert!(
            err.contains("ignoring"),
            "{what}: the skipped config was not reported at all:\n{err}"
        );
        // Skipping must leave the built-in registry deciding, not disable safe-chains.
        assert!(
            out.status.success(),
            "{what}: a safe command stopped being approved because a custom config was bad"
        );
    }
}

/// And the skip must stay FAIL-SAFE: a bad config may not widen anything.
#[test]
fn a_shape_invalid_config_does_not_widen_the_allowlist() {
    let home = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(home.path().join(".config")).expect("mkdir .config");
    std::fs::write(home.path().join(".config/safe-chains.toml"), SHAPE_INVALID[0].1)
        .expect("write config");
    let work = home.path().join("wk");
    std::fs::create_dir_all(&work).expect("mkdir wk");

    let out = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .arg("--root")
        .arg(&work)
        .arg("--cwd")
        .arg(&work)
        .arg("rm -rf /")
        .env("HOME", home.path())
        .output()
        .expect("run safe-chains");
    assert!(!out.status.success(), "a bad custom config must not make `rm -rf /` auto-approve");
}
