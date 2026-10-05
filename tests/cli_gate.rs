//! CLI-gate exit-code contract. In gate mode (`safe-chains "<cmd>" [--level L]`), a MALFORMED
//! invocation must FAIL CLOSED — a non-zero exit — never exit 0 ("allowed"). Regression for the
//! typo'd-flag fail-open: a clap parse error used to fall through to hook mode, which read empty
//! stdin and exited 0.
use std::io::Write;
use std::process::{Command, Stdio};

#[path = "support/hooks.rs"]
mod hooks;
use hooks::run_hook;

/// Run the binary in claude-hook mode (bare, JSON on stdin) and return its stdout.
///
/// `home` is required rather than inherited: the hook reads `~/.claude/settings.json` and
/// `~/.config/safe-chains.toml`, so a guard that let this default would be measuring whatever
/// config the person running the suite happens to have.
/// `cwd` is likewise explicit, so a guard comparing the hook against the CLI can stand both in the
/// same directory.
fn hook_stdout(payload: &str, cwd: &str, home: &std::path::Path) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .current_dir(cwd)
        .env("HOME", home)
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
///
/// Pinned to a HOME of its own. Without that it read whatever `~/.claude/settings.json` and
/// `~/.config/safe-chains.toml` the person running it happens to have, and a single `permissions.allow`
/// entry covering one of these commands turns the nudge into an approval — so the guard passed in
/// CI, where HOME is bare, and failed on the machine of anyone who had approved `grep` once. A test
/// that reads the developer's personal config is not testing safe-chains.
#[test]
fn overreach_nudge_names_the_working_directory() {
    let home = tempfile::tempdir().expect("tempdir");
    // A WRITE and a SWEEP. `cat /other/repo/x.rs` used to stand here and no longer overreaches at
    // all — reading a named file outside the workspace is ordinary now, so there is nothing to
    // nudge about. The nudge fires where reaching out is still refused, and both remaining shapes
    // must name the cwd (the mismatch cue) and the path they reached.
    for (command, reached) in [(r"echo x > /other/repo/x.rs", "/other/repo/x.rs"), (r"grep -r x /other/repo", "/other/repo")] {
        let payload = format!(r#"{{"tool_input":{{"command":"{command}"}},"cwd":"/work/here"}}"#);
        let out = hook_stdout(&payload, env!("CARGO_MANIFEST_DIR"), home.path());
        assert!(out.contains("/work/here"), "nudge must NAME the working directory: {out}");
        assert!(out.contains(reached), "nudge must name the reached path: {out}");
    }
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
    assert_eq!(std::fs::read_to_string(&cfg).expect("read"), original, "refusing must leave the file untouched");

    // The VALID case, so the refusal above cannot be satisfied by declining everything: it still
    // succeeds and still produces the entry — on STDOUT. `--suggest` writes nothing at all now, so
    // what distinguishes the two cases is the exit code and where the block appears, not whether a
    // file changed.
    let good = tmp.path().join("good");
    std::fs::create_dir(&good).expect("mkdir");
    let cfg = good.join(".safe-chains.toml");
    let original = "[[command]]\nname = \"existing\"\nmax_positional = 1\n";
    std::fs::write(&cfg, original).expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .args(["--suggest", "frobnicate build"])
        .current_dir(&good)
        .stdin(Stdio::null())
        .output()
        .expect("run");
    assert_eq!(out.status.code().unwrap_or(-1), 0, "a VALID config must still produce an entry");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("frobnicate"), "the generated entry must be printed:\n{text}");
    assert!(text.contains("[[trusted]]"), "the pin must be printed:\n{text}");
    assert_eq!(std::fs::read_to_string(&cfg).expect("read"), original, "--suggest must not modify the project config");
}

/// `--suggest` writes NOTHING — not even to a project that has no config yet.
///
/// It used to write the file and report "Added this to …", which is not what a flag called
/// `--suggest` says it does. The name is the part people read, and the obvious way to find out what
/// it says was to run it — which changed the project.
///
/// The fresh-project case is the one that would regress most quietly: with no existing file there
/// is nothing to preserve, so a write here breaks no assertion about surviving content. It has to
/// be checked by absence.
#[test]
fn suggest_creates_no_file_in_a_fresh_project() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let proj = tmp.path().join("proj");
    std::fs::create_dir_all(proj.join(".git")).expect("mkdir .git");

    let out = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .args(["--suggest", "frobnicate build"])
        .current_dir(&proj)
        .stdin(Stdio::null())
        .output()
        .expect("run");

    assert_eq!(out.status.code().unwrap_or(-1), 0, "suggesting is not a failure");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("frobnicate"), "the entry must still be printed:\n{text}");
    assert!(!proj.join(".safe-chains.toml").exists(), "--suggest created a config file; it is informational and must write nothing");
}

/// The levels must genuinely DISCRIMINATE, or the `level_monotonic` fuzz target is theatre.
///
/// That target asserts tightening never loosens a verdict — a property trivially satisfied if every
/// level returns the same answer. This pins the staircase it explores: each step admits strictly
/// more than the one below. If levels ever collapse into each other the fuzz target would keep
/// passing while guarding nothing, and this fails instead.
#[test]
fn levels_admit_strictly_more_as_they_loosen() {
    // (level, cat a file, write a file, remove a file, push to a remote)
    const STAIRCASE: &[(&str, [bool; 4])] = &[
        ("paranoid", [false, false, false, false]),
        ("reader", [true, false, false, false]),
        ("editor", [true, true, false, false]),
        ("developer", [true, true, true, false]),
        ("network-admin", [true, true, true, true]),
    ];
    const COMMANDS: [&str; 4] = ["cat ./a.txt", "echo hi > ./a.txt", "rm ./a.txt", "git push origin main"];

    let cwd = std::env::current_dir().expect("cwd");
    let root = cwd.display().to_string();
    for (level, expected) in STAIRCASE {
        for (command, want) in COMMANDS.iter().zip(expected) {
            let code = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
                .args(["--cwd", &root, "--root", &root, "--level", level, command])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("run")
                .code()
                .unwrap_or(-1);
            let allowed = code == 0;
            assert_eq!(allowed, *want, "level `{level}` on `{command}`: expected allowed={want}, got {allowed}");
        }
    }
}

/// The CLI and the hook must reach the SAME verdict for the same command in the same directory.
///
/// `safe-chains "<cmd>"` is the tool people run to ask what the hook decided — the `--help`
/// examples say exactly that, and none of them pass `--cwd`. So the bare form is the one everyone
/// measures with, and for a long time it was the lenient one: with no `--cwd` the `PathCtx` had
/// neither a cwd nor a root, `pathctx::resolve` joins a relative path only when BOTH are known, and
/// so the workspace boundary simply did not exist. A `cd` out of the project was invisible and
/// every relative path behind it classified worktree-local. `cd ~/Library/… && unzip -l x.zip`
/// came back ALLOW from the CLI and abstain from the hook, for the same command in the same
/// directory — and a reader who trusted the CLI concluded safe-chains had no opinion about paths
/// at all.
///
/// The guard is a PARITY property rather than a fixed expectation per command: it asserts the two
/// entry points agree, never that a particular command is allowed, so it keeps holding as
/// classifications change. The `cd`-prefixed rows are the discriminating ones (they are what the
/// boundary decides); the bare rows hold the other direction, that installing a cwd did not make
/// the CLI stricter than the hook on ordinary in-workspace work.
#[test]
fn the_cli_and_the_hook_agree_in_the_same_directory() {
    // Somewhere real to stand that is NOT a temp dir (temp has its own admit rules, which would
    // mask the boundary this is about).
    let workspace = env!("CARGO_MANIFEST_DIR");
    // A HOME of our own, for the same reason `claude_config_scope.rs` builds one: both entry points
    // read `~/.claude/settings.json` and `~/.config/safe-chains.toml`, so on the developer's own
    // machine this would be measuring their personal rules. The settings file is not empty on
    // purpose — the coverage and `Read()` bridges are themselves a thing the two entry points have
    // disagreed about before, so parity has to be asserted with them in play.
    let home = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(home.path().join(".claude")).expect("mkdir .claude");
    std::fs::create_dir_all(home.path().join(".config")).expect("mkdir .config");
    std::fs::write(home.path().join(".claude/settings.json"), r#"{"permissions":{"allow":["Bash(cat:*)","Read(//opt/vendor/**)"]}}"#)
        .expect("write settings.json");
    std::fs::write(home.path().join(".config/safe-chains.toml"), "").expect("write config");

    const OUTSIDE: &[&str] = &["/etc", "/usr/share/vendor", "~/Library/Preferences/calibre", "/Users/someone/other-project"];
    // A spread across the roles the boundary treats differently: a named read, an archive listing,
    // an unbounded sweep, a redirect write, a destroy, an in-place edit.
    const BODIES: &[&str] = &[
        "cat notes.txt", "unzip -l archive.zip", "tar -tf bundle.tar", "grep -r TODO .", "echo x > out.txt", "rm -rf build",
        "sed -i s/a/b/ notes.txt",
    ];

    let mut cases: Vec<String> = BODIES.iter().map(|b| (*b).to_string()).collect();
    for dir in OUTSIDE {
        for body in BODIES {
            cases.push(format!("cd {dir} && {body}"));
        }
    }

    let (mut allowed_seen, mut refused_seen) = (false, false);
    for command in &cases {
        let cli_allowed = Command::new(env!("CARGO_BIN_EXE_safe-chains"))
            .arg(command)
            .current_dir(workspace)
            .env("HOME", home.path())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("run safe-chains")
            .code()
            .unwrap_or(-1)
            == 0;

        let payload = serde_json::json!({
            "tool_name": "Bash",
            "tool_input": {"command": command},
            "cwd": workspace,
        })
        .to_string();
        let hook_allowed = hook_stdout(&payload, workspace, home.path()).contains(r#""permissionDecision":"allow""#);

        assert_eq!(
            cli_allowed, hook_allowed,
            "`{command}` in {workspace}: CLI says allowed={cli_allowed}, hook says \
             allowed={hook_allowed}. The CLI is how people ask what the hook decided; it has to \
             decide it the same way."
        );
        allowed_seen |= cli_allowed;
        refused_seen |= !cli_allowed;
    }

    // Non-vacuity: a corpus that is uniformly allowed (or uniformly refused) would agree trivially
    // and prove nothing about the boundary.
    assert!(allowed_seen, "corpus must contain a command both entry points ALLOW");
    assert!(refused_seen, "corpus must contain a command both entry points REFUSE");
}

/// `--explain` is a gate as well as a report: scripts read its exit code the same way they read
/// the bare gate's. Inverting it would report an approval as a refusal and the reverse.
#[test]
fn explain_exits_with_the_verdict() {
    let (_, _, allowed) = run_hook(&["--explain", "echo hi"], "");
    assert_eq!(allowed, 0, "--explain must exit 0 for an approved command");
    let (_, _, refused) = run_hook(&["--explain", "rm -rf /"], "");
    assert_eq!(refused, 1, "--explain must exit 1 for a refused command");
}

/// `--list-commands` prints the researched-command reference. An empty listing exits 0 too, so
/// the exit code alone says nothing.
#[test]
fn list_commands_prints_the_command_reference() {
    let (out, _, code) = run_hook(&["--list-commands"], "");
    assert_eq!(code, 0, "--list-commands exits 0");
    assert!(out.lines().any(|l| l == "### `git`"), "--list-commands must document git:\n{out}");
    assert!(out.lines().filter(|l| l.starts_with("### ")).count() > 100, "--list-commands documented almost nothing");
}
