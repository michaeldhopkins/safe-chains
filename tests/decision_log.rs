//! The decision log's contract, exercised through a real process with a real `$HOME`.
//!
//! An integration test rather than a unit one for the same reason `claude_config_scope` is: the log
//! resolves `$HOME` at write time and creates a file with a mode, so the properties that matter —
//! that a logging fault cannot change a verdict, that the two modes differ in exactly one way, and
//! that "off" creates nothing — only exist in a real process.
use std::io::Write;
use std::process::{Command, Stdio};

/// One command per outcome class, so every guard below sweeps all four rather than the easy one.
const ALLOWED: &str = "ls -la";
const UNKNOWN: &str = "definitelynotarealtool --frobnicate";
const RECOGNIZED_DENIED: &str = "aws dynamodb put-item --table-name t --item {}";
const UNPARSEABLE: &str = "echo \"unterminated";

fn all_classes() -> [&'static str; 4] {
    [ALLOWED, UNKNOWN, RECOGNIZED_DENIED, UNPARSEABLE]
}

struct Run {
    stdout: String,
    exit: i32,
}

fn hook(command: &str, home: &std::path::Path, flags: &[&str]) -> Run {
    hook_env(command, Some(home), flags)
}

fn hook_env(command: &str, home: Option<&std::path::Path>, flags: &[&str]) -> Run {
    // Built with `json!` rather than `format!`: one of these commands is deliberately unparseable
    // SHELL, which means it carries a quote, and hand-interpolating it produced a payload whose
    // command differed from the constant every assertion compares against.
    let payload = serde_json::json!({
        "tool_name": "Bash",
        "tool_input": { "command": command },
        "cwd": "/work",
    })
    .to_string();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_safe-chains"));
    for f in flags {
        cmd.arg(f);
    }
    cmd.arg("hook").arg("claude");
    match home {
        Some(h) => {
            cmd.env("HOME", h);
        }
        None => {
            cmd.env_remove("HOME");
        }
    }
    let mut child = cmd
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
    Run {
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        exit: out.status.code().unwrap_or(-1),
    }
}

fn tmp_home(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("safe-chains-log-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the temp home");
    dir
}

fn log_lines(home: &std::path::Path) -> Vec<serde_json::Value> {
    let path = home.join(".local/state/safe-chains/log.jsonl");
    let Ok(text) = std::fs::read_to_string(&path) else { return Vec::new() };
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("line is not JSON: {e}\n{l}")))
        .collect()
}

/// THE guard. A `PreToolUse` hook that crashes fails OPEN — the harness runs the command — so a
/// logging fault must be incapable of reaching the decision. Every failure mode the module claims
/// to swallow is exercised against the no-logging baseline, and the whole response must be
/// byte-identical: same stdout, same exit code.
#[test]
fn logging_never_changes_the_decision() {
    let good = tmp_home("nochange-good");
    // A `$HOME` that cannot be written into, and one that does not exist at all.
    let readonly = tmp_home("nochange-ro");
    let mut perms = std::fs::metadata(&readonly).expect("stat").permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o555);
    }
    std::fs::set_permissions(&readonly, perms).expect("make the home read-only");
    let missing = std::env::temp_dir().join("safe-chains-log-does-not-exist-anywhere");
    let _ = std::fs::remove_dir_all(&missing);
    // A home where the directory tree is fine but the log PATH itself cannot be opened, because
    // something else already occupies it as a directory. Without this the guard was VACUOUS for the
    // whole write half: `create_dir_all` returns first on a read-only home, so the open, the write
    // and the rotate were never reached — replacing the open with an `expect` left it green.
    let blocked = tmp_home("nochange-blocked");
    std::fs::create_dir_all(blocked.join(".local/state/safe-chains/log.jsonl"))
        .expect("occupy the log path with a directory");

    for command in all_classes() {
        let baseline = hook(command, &good, &[]);
        let cases: [(&str, Option<&std::path::Path>, &[&str]); 6] = [
            ("writable --log", Some(&good), &["--log"]),
            ("writable --log-everything", Some(&good), &["--log-everything"]),
            ("read-only home", Some(&readonly), &["--log-everything"]),
            ("missing home", Some(&missing), &["--log-everything"]),
            ("log path is a directory", Some(&blocked), &["--log-everything"]),
            ("HOME unset", None, &["--log-everything"]),
        ];
        for (label, home, flags) in cases {
            let got = hook_env(command, home, flags);
            assert_eq!(
                got.stdout, baseline.stdout,
                "{label}: stdout differs from the no-logging baseline for {command:?}"
            );
            assert_eq!(
                got.exit, baseline.exit,
                "{label}: exit code differs from the no-logging baseline for {command:?}"
            );
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = std::fs::metadata(&readonly).expect("stat").permissions();
        p.set_mode(0o755);
        let _ = std::fs::set_permissions(&readonly, p);
    }
}

/// Off means off: no file, no directory, nothing to find later.
#[test]
fn without_a_flag_nothing_is_written() {
    let home = tmp_home("off");
    for command in all_classes() {
        hook(command, &home, &[]);
    }
    assert!(
        !home.join(".local/state/safe-chains").exists(),
        "logging was off, but a state directory was created"
    );
}

/// The two modes differ in exactly ONE way, and both halves need pinning because each is the
/// other's failure case: logging approvals under `--log` is a volume and privacy surprise, and
/// dropping them under `--log-everything` silently defeats the flag.
#[test]
fn the_modes_differ_only_on_approvals() {
    let quiet = tmp_home("mode-quiet");
    let loud = tmp_home("mode-loud");
    for command in all_classes() {
        hook(command, &quiet, &["--log"]);
        hook(command, &loud, &["--log-everything"]);
    }

    let quiet_cmds: Vec<String> = log_lines(&quiet)
        .iter()
        .map(|e| e["command"].as_str().unwrap_or_default().to_string())
        .collect();
    let loud_cmds: Vec<String> = log_lines(&loud)
        .iter()
        .map(|e| e["command"].as_str().unwrap_or_default().to_string())
        .collect();

    assert!(!quiet_cmds.contains(&ALLOWED.to_string()), "--log recorded an approval");
    assert_eq!(quiet_cmds.len(), 3, "--log should hold the three non-approvals: {quiet_cmds:?}");
    assert_eq!(loud_cmds.len(), 4, "--log-everything should hold all four: {loud_cmds:?}");

    // ...and the non-approval entries are otherwise identical between the modes.
    for cmd in [UNKNOWN, RECOGNIZED_DENIED] {
        let pick = |set: &[serde_json::Value]| {
            let mut e = set
                .iter()
                .find(|e| e["command"] == cmd)
                .unwrap_or_else(|| panic!("{cmd} missing"))
                .clone();
            // The id and timestamp are per-run by construction.
            e["id"] = serde_json::Value::Null;
            e["at"] = serde_json::Value::Null;
            e
        };
        assert_eq!(pick(&log_lines(&quiet)), pick(&log_lines(&loud)), "{cmd} differs by mode");
    }
}

/// Each outcome class lands with the triage that decides what to DO about it: a registry gap is an
/// issue, a classification is a judgement call, a parse failure is a parser signal.
#[test]
fn every_outcome_class_is_recorded_with_its_triage() {
    let home = tmp_home("classes");
    for command in all_classes() {
        hook(command, &home, &["--log-everything"]);
    }
    let by_cmd = |c: &str| {
        log_lines(&home)
            .into_iter()
            .find(|e| e["command"] == c)
            .unwrap_or_else(|| panic!("no entry for {c}"))
    };

    let allowed = by_cmd(ALLOWED);
    assert_eq!(allowed["outcome"], "allowed");
    assert_eq!(allowed["triage"], "allowed");
    assert_eq!(
        allowed["segments"][0]["facets"],
        serde_json::Value::Null,
        "an approved segment owes no reason"
    );

    let unknown = by_cmd(UNKNOWN);
    assert_eq!(unknown["outcome"], "denied");
    assert_eq!(unknown["triage"], "unknown-command");
    assert_eq!(unknown["unknown_commands"], serde_json::json!(["definitelynotarealtool"]));

    let known = by_cmd(RECOGNIZED_DENIED);
    assert_eq!(known["outcome"], "denied");
    assert_eq!(known["triage"], "recognized-but-denied");
    assert!(
        known["segments"][0]["facets"]["refused_by"]["clause"]
            .as_str()
            .is_some_and(|c| c.contains("locus")),
        "expected the refusing axis, got {}", known["segments"][0]
    );

    let bad = by_cmd(UNPARSEABLE);
    assert_eq!(bad["outcome"], "unparseable");
    assert_eq!(bad["triage"], "unparseable");
}

/// Context the harness supplied is carried through, because a denial without the directory it ran
/// in is not reproducible — the exact gap that made the existing hand-kept backlog hard to triage.
#[test]
fn harness_context_reaches_the_entry() {
    let home = tmp_home("context");
    hook(UNKNOWN, &home, &["--log"]);
    let e = &log_lines(&home)[0];
    assert_eq!(e["cwd"], "/work");
    assert_eq!(e["harness"], "claude");
    assert_eq!(e["level"], "developer", "the level must read as --explain names it");
    assert_eq!(e["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(e["schema"], 1);
}

/// `--log` on the CLI must actually log. It was accepted and silently did nothing, which is the
/// worst behaviour a flag can have: `safe-chains --log "cmd"` printed the right verdict and wrote no
/// entry, so the obvious way to check that logging worked quietly proved it did not.
#[test]
fn the_cli_path_logs_too() {
    let home = tmp_home("cli");
    for (command, flags) in
        [(UNKNOWN, vec!["--log"]), (ALLOWED, vec!["--log"]), (ALLOWED, vec!["--log-everything"])]
    {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_safe-chains"));
        for f in &flags {
            cmd.arg(f);
        }
        cmd.arg("--cwd").arg("/work").arg(command).env("HOME", &home);
        cmd.stdout(Stdio::null()).stderr(Stdio::null()).status().expect("run the CLI");
    }
    let entries = log_lines(&home);
    let commands: Vec<&str> = entries.iter().filter_map(|e| e["command"].as_str()).collect();
    // The denial under --log and the approval under --log-everything; the approval under --log is
    // correctly absent, so the mode split holds on this path too.
    assert_eq!(commands, vec![UNKNOWN, ALLOWED], "CLI entries: {commands:?}");
    assert_eq!(entries[0]["harness"], "cli", "CLI traffic must be distinguishable from a hook");
    assert_eq!(entries[0]["cwd"], "/work");
}

/// A blank command is ALLOWED but not grantable — safe-chains says nothing and the harness decides.
/// That is an abstain, not an approval, and `may_grant` exists precisely because approving nothing
/// is not approving. Recording it as `allowed` also left `Outcome::Abstained` with no producer at
/// all, so the field table advertised a value the code could never emit.
#[test]
fn a_blank_command_records_an_abstain() {
    let home = tmp_home("abstain");
    hook("", &home, &["--log"]);
    let entries = log_lines(&home);
    assert_eq!(entries.len(), 1, "a blank command should be recorded under --log");
    assert_eq!(entries[0]["outcome"], "abstained");
}

/// The file holds commands verbatim, credentials included, so it is owner-only from creation — a
/// later chmod would leave a window where it was not.
#[cfg(unix)]
#[test]
fn the_log_is_created_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let home = tmp_home("mode-bits");
    hook(UNKNOWN, &home, &["--log"]);
    let meta = std::fs::metadata(home.join(".local/state/safe-chains/log.jsonl")).expect("stat");
    assert_eq!(meta.permissions().mode() & 0o777, 0o600);
}

/// Concurrent hooks are the normal case — parallel tool calls, several sessions — and a torn line
/// breaks every `jq` consumer for the whole file, not just its own entry.
#[test]
fn concurrent_writers_produce_whole_lines() {
    const WRITERS: usize = 12;
    let home = tmp_home("concurrent");
    let handles: Vec<_> = (0..WRITERS)
        .map(|i| {
            let home = home.clone();
            std::thread::spawn(move || {
                hook(&format!("definitelynotarealtool run{i}"), &home, &["--log"]);
            })
        })
        .collect();
    for h in handles {
        h.join().expect("writer thread");
    }
    // `log_lines` parses every line and panics on a torn one, so reaching the count is the check.
    let lines = log_lines(&home);
    assert_eq!(lines.len(), WRITERS, "expected one whole line per writer");
    let mut seen: Vec<&str> = lines.iter().filter_map(|e| e["command"].as_str()).collect();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), WRITERS, "entries were lost or duplicated under concurrency");
}
