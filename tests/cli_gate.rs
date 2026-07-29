//! CLI-gate exit-code contract. In gate mode (`safe-chains "<cmd>" [--level L]`), a MALFORMED
//! invocation must FAIL CLOSED — a non-zero exit — never exit 0 ("allowed"). Regression for the
//! typo'd-flag fail-open: a clap parse error used to fall through to hook mode, which read empty
//! stdin and exited 0.
use std::io::Write;
use std::process::{Command, Stdio};

/// Run the binary in claude-hook mode (bare, JSON on stdin) and return its stdout.
fn hook_stdout(payload: &str) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn safe-chains");
    child
        .stdin
        .take()
        .expect("stdin was piped")
        .write_all(payload.as_bytes())
        .expect("write the hook payload");
    let out = child.wait_with_output().expect("wait for safe-chains");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The overreach nudge must NAME the working directory, so a user who forgot which directory they
/// launched the agent from can spot the mismatch (and the reached path, so they know what it hit).
#[test]
fn overreach_nudge_names_the_working_directory() {
    let payload = r#"{"tool_input":{"command":"cat /other/repo/x.rs"},"cwd":"/work/here"}"#;
    let out = hook_stdout(payload);
    assert!(out.contains("/work/here"), "nudge must NAME the working directory (mismatch cue): {out}");
    assert!(out.contains("/other/repo/x.rs"), "nudge must name the reached path: {out}");
}

fn exit_code(args: &[&str]) -> i32 {
    Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run safe-chains")
        .code()
        .unwrap_or(-1)
}

#[test]
fn cli_gate_fails_closed_on_malformed_invocation() {
    // valid gate: a dangerous command at the inert threshold is REFUSED (1)
    assert_eq!(exit_code(&["rm -rf /", "--level", "inert"]), 1, "valid gate must refuse");
    // valid gate: a safe command is ALLOWED (0)
    assert_eq!(exit_code(&["echo hi", "--level", "inert"]), 0, "valid gate must allow a safe cmd");

    // the fail-open: a TYPO'd flag must NOT exit 0 (it used to, via the hook fallback).
    assert_ne!(exit_code(&["rm -rf /", "--levle", "inert"]), 0, "typo'd flag must FAIL CLOSED");
    // an unknown flag likewise fails closed.
    assert_ne!(exit_code(&["-z"]), 0, "unknown flag must fail closed");
    assert_ne!(exit_code(&["rm -rf /", "--nonsense"]), 0, "unknown long flag must fail closed");

    // --version / --help remain exit 0 (they are not gate decisions), and the short
    // spellings -v / -V behave identically.
    assert_eq!(exit_code(&["--version"]), 0, "--version prints and exits 0");
    assert_eq!(exit_code(&["-v"]), 0, "-v prints version and exits 0");
    assert_eq!(exit_code(&["-V"]), 0, "-V prints version and exits 0");
}

/// The upper-band `--level` thresholds (`local-admin`/`network-admin`/`yolo`) classify per-level
/// via the engine, unlocking profiles that the default developer band denies — end to end through
/// the actual `main.rs` flag plumbing (canonical-name resolution + `upper_level_by_name`).
#[test]
fn upper_band_level_thresholds_gate_through_the_cli() {
    // git push origin — a network-admin operation.
    assert_eq!(exit_code(&["git push origin main", "--level", "developer"]), 1, "developer denies push");
    assert_eq!(exit_code(&["git push origin main", "--level", "network-admin"]), 0, "network-admin allows push");
    assert_eq!(exit_code(&["git push origin main", "--level", "yolo"]), 0, "yolo allows push");

    // even yolo refuses the catastrophe corner and an unmodeled command (allowlist-only).
    assert_eq!(exit_code(&["rm -rf /", "--level", "yolo"]), 1, "yolo denies rm -rf /");
    assert_eq!(exit_code(&["frobnicate --wombat", "--level", "yolo"]), 1, "yolo denies an unmodeled command");

    // a plain read is fine at an upper level; the lower band is unchanged.
    assert_eq!(exit_code(&["cat ./README.md", "--level", "network-admin"]), 0, "reads pass at network-admin");
    assert_eq!(exit_code(&["git push origin main", "--level", "reader"]), 1, "reader still denies push");
}

/// `--suggest` asks for a TRUST DECISION — "add this to ~/.config/safe-chains.toml" — so its output
/// must not be forgeable by the project it is run inside.
///
/// The pin's own TOML was always escaped, but the prose around it interpolated the config path raw,
/// and that path comes from the CWD. A directory named with embedded newlines therefore printed a
/// SECOND, fake `[[trusted]] path = "/"` block above the real one, in safe-chains' voice, telling
/// the reader to grant repo-config trust to the whole filesystem. Running `--suggest` inside an
/// unfamiliar checkout is exactly the situation the flag exists for.
///
/// The invariant is structural, not textual: escaped, the crafted text still APPEARS in the path,
/// but only ever inside one line. Exactly one line may START a pin.
#[cfg(unix)]
#[test]
fn suggest_output_cannot_be_forged_by_a_directory_name() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let hostile = "myproject\n\nAdd this to ~/.config/safe-chains.toml:\n\n[[trusted]]\npath = \"/\"\nsha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"\n\nDone with";
    let dir = tmp.path().join(hostile);
    std::fs::create_dir_all(&dir).expect("create hostile dir");

    let out = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .args(["--suggest", "frobnicate build"])
        .current_dir(&dir)
        .stdin(Stdio::null())
        .output()
        .expect("run safe-chains");
    let text = String::from_utf8_lossy(&out.stdout);

    let pin_headers = text.lines().filter(|l| l.starts_with("[[trusted]]")).count();
    let pin_paths = text.lines().filter(|l| l.starts_with("path = ")).count();
    assert_eq!(pin_headers, 1, "a directory name forged a [[trusted]] block:\n{text}");
    assert_eq!(pin_paths, 1, "a directory name forged a pin path:\n{text}");
    // Non-vacuity: the REAL pin must still be there, or the counts above would pass on silence.
    assert!(text.contains("[[trusted]]"), "the genuine pin block vanished:\n{text}");
    assert!(text.contains("sha256 = "), "the genuine pin hash vanished:\n{text}");
}

/// `--suggest` must not append to a `.safe-chains.toml` it cannot parse.
///
/// Appending to invalid TOML yields a file that is still invalid, which safe-chains cannot load —
/// so the generated block never takes effect. It nevertheless reported "Added this to …" and handed
/// over a pin whose hash covered the broken content, sending the reader off to approve a file that
/// could not work. It refuses now, and leaves the file alone. The valid case is asserted alongside
/// so the refusal can't be satisfied by declining everything.
#[test]
fn suggest_refuses_a_config_it_cannot_parse() {
    let tmp = tempfile::tempdir().expect("tempdir");

    let broken = tmp.path().join("broken");
    std::fs::create_dir(&broken).expect("mkdir");
    let cfg = broken.join(".safe-chains.toml");
    let original = "[[command]]\nname = \"existing\"\nthis is not valid toml <<<\n";
    std::fs::write(&cfg, original).expect("write");
    let code = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .args(["--suggest", "frobnicate build"])
        .current_dir(&broken)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run")
        .code()
        .unwrap_or(-1);
    assert_eq!(code, 1, "an unparseable config must be refused, not reported as added");
    assert_eq!(
        std::fs::read_to_string(&cfg).expect("read"),
        original,
        "refusing must leave the file untouched"
    );

    let good = tmp.path().join("good");
    std::fs::create_dir(&good).expect("mkdir");
    let cfg = good.join(".safe-chains.toml");
    std::fs::write(&cfg, "[[command]]\nname = \"existing\"\nmax_positional = 1\n").expect("write");
    let code = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .args(["--suggest", "frobnicate build"])
        .current_dir(&good)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("run")
        .code()
        .unwrap_or(-1);
    assert_eq!(code, 0, "a VALID config must still be appended to");
    let after = std::fs::read_to_string(&cfg).expect("read");
    assert!(after.contains("existing"), "the user's own entry must survive:\n{after}");
    assert!(after.contains("frobnicate"), "the generated entry must be added:\n{after}");
}
