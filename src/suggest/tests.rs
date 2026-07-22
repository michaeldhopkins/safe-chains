use super::*;
use crate::parse::Token;
use crate::registry::{dispatch_spec, load_toml};
use proptest::prelude::*;

// ── Generators ──────────────────────────────────────────────────────────────────────────────────

/// A command name safe-chains does NOT already know — the case `--suggest` generates for.
fn arb_unknown_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{3,11}".prop_filter("must be unknown to safe-chains", |n| !is_known(n))
}

/// An observed flag: short, short-cluster, glued-value, or long.
fn arb_flag() -> impl Strategy<Value = String> {
    prop_oneof![
        "-[a-z]",
        "-[a-z]{2,4}",
        "--[a-z][a-z-]{1,7}",
        "--[a-z]{2,6}=[a-z0-9]{1,4}",
    ]
}

/// An observed positional argument (never starts with `-`).
fn arb_positional() -> impl Strategy<Value = String> {
    "[a-z0-9][a-z0-9._/]{0,7}"
}

/// A flag no `arb_flag` can produce (has a dash *and* a digit), so it is guaranteed "unobserved".
const UNOBSERVED_FLAG: &str = "--never-observed-9z";

fn argv_command(name: &str, flags: &[String], positionals: &[String]) -> (String, Vec<String>) {
    let mut argv = vec![name.to_string()];
    argv.extend(flags.iter().cloned());
    argv.extend(positionals.iter().cloned());
    (argv.join(" "), argv)
}

fn tokens(argv: &[String]) -> Vec<Token> {
    argv.iter().map(|s| Token::from_test(s)).collect()
}

fn only_entry(command: &str) -> GeneratedEntry {
    match analyze(command) {
        Outcome::Generated { mut entries, .. } => {
            assert_eq!(entries.len(), 1, "expected one entry for `{command}`");
            entries.remove(0)
        }
        other => panic!("expected Generated for `{command}`, got {other:?}"),
    }
}

// ── Round-trip: the generated entry admits the observed command ──────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(600))]

    /// THE core guarantee: for an unknown command with any observed flags/positionals, the generated
    /// TOML parses and, loaded, makes that exact invocation classify as Allowed. Loads the spec via
    /// `load_toml` and dispatches with `dispatch_spec` — no global registry, no env, fully in-process.
    #[test]
    fn generated_entry_admits_the_observed_command(
        name in arb_unknown_name(),
        flags in prop::collection::vec(arb_flag(), 0..5),
        positionals in prop::collection::vec(arb_positional(), 0..4),
    ) {
        let (command, argv) = argv_command(&name, &flags, &positionals);
        let outcome = analyze(&command);
        prop_assert!(matches!(outcome, Outcome::Generated { .. }), "expected Generated, got {outcome:?}");
        let Outcome::Generated { entries, .. } = outcome else { unreachable!() };
        prop_assert_eq!(entries.len(), 1);
        prop_assert_eq!(&entries[0].name, &name);

        let toml = render_toml(&entries);
        let specs = load_toml(&toml, "suggest-test");
        prop_assert_eq!(specs.len(), 1, "generated toml did not load to one spec:\n{}", toml);
        prop_assert!(
            dispatch_spec(&tokens(&argv), &specs[0]).is_allowed(),
            "generated entry did not admit `{}`; toml:\n{}", command, toml
        );
    }

    /// SCOPING (flags): the entry must NOT admit a flag that was never observed — no blanket
    /// tolerate-unknown broadening.
    #[test]
    fn generated_entry_rejects_an_unobserved_flag(
        name in arb_unknown_name(),
        flags in prop::collection::vec(arb_flag(), 0..4),
        positionals in prop::collection::vec(arb_positional(), 0..3),
    ) {
        let (command, _) = argv_command(&name, &flags, &positionals);
        let entry = only_entry(&command);
        prop_assert!(!entry.standalone.iter().any(|f| f == UNOBSERVED_FLAG));

        let toml = render_toml(std::slice::from_ref(&entry));
        let specs = load_toml(&toml, "suggest-test");
        let probe = tokens(&[name.clone(), UNOBSERVED_FLAG.to_string()]);
        prop_assert!(
            !dispatch_spec(&probe, &specs[0]).is_allowed(),
            "entry wrongly admitted an unobserved flag; toml:\n{}", toml
        );
    }

    /// SCOPING (positionals): the entry records the observed positional count and denies one more.
    #[test]
    fn generated_entry_bounds_positionals(
        name in arb_unknown_name(),
        positionals in prop::collection::vec(arb_positional(), 0..4),
    ) {
        let (command, _) = argv_command(&name, &[], &positionals);
        let entry = only_entry(&command);
        prop_assert_eq!(entry.max_positional, positionals.len());

        let toml = render_toml(std::slice::from_ref(&entry));
        let specs = load_toml(&toml, "suggest-test");
        let mut argv = vec![name.clone()];
        argv.extend(std::iter::repeat_n("x".to_string(), entry.max_positional + 1));
        prop_assert!(
            !dispatch_spec(&tokens(&argv), &specs[0]).is_allowed(),
            "entry admitted {}+1 positionals; toml:\n{}", entry.max_positional, toml
        );
    }

    /// The emitted TOML is always valid and round-trips its names, for any entry shape.
    #[test]
    fn generated_toml_always_loads(
        name in arb_unknown_name(),
        flags in prop::collection::vec(arb_flag(), 0..6),
        max_positional in 0usize..6,
    ) {
        let entry = GeneratedEntry {
            name: name.clone(),
            standalone: {
                let mut v: Vec<String> = flags.into_iter().collect::<BTreeSet<_>>().into_iter().collect();
                v.sort();
                v
            },
            max_positional,
            level: "SafeWrite".to_string(),
        };
        let toml = render_toml(std::slice::from_ref(&entry));
        let specs = load_toml(&toml, "suggest-test");
        prop_assert_eq!(specs.len(), 1, "toml failed to load:\n{}", toml);
        prop_assert_eq!(&specs[0].name, &name);
    }

    /// `analyze` is a pure, deterministic function of its input.
    #[test]
    fn analyze_is_deterministic(
        name in arb_unknown_name(),
        flags in prop::collection::vec(arb_flag(), 0..4),
    ) {
        let (command, _) = argv_command(&name, &flags, &[]);
        prop_assert_eq!(analyze(&command), analyze(&command));
    }
}

// ── The hash matches the trust mechanism's ────────────────────────────────────────────────────────

#[test]
fn config_hash_matches_trust_sha256() {
    // Identical to `registry::custom::sha256_hex` (verified there against the same NIST vectors), so
    // the printed pin is exactly what `repo_is_trusted` recomputes over the file.
    assert_eq!(
        config_hash(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        config_hash(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

/// The pin hash is taken over the WHOLE merged file, so a round-trip through `merged_content` +
/// `config_hash` equals hashing the file a user would `shasum` — the property the trust check needs.
#[test]
fn pin_hash_is_over_the_merged_file() {
    let entry = GeneratedEntry {
        name: "mytool".into(),
        standalone: vec!["-x".into()],
        max_positional: 1,
        level: "SafeWrite".into(),
    };
    let merged = merged_content("", std::slice::from_ref(&entry));
    assert_eq!(config_hash(merged.as_bytes()), config_hash(render_toml(std::slice::from_ref(&entry)).as_bytes()));
    // Appending to an existing file keeps the existing content verbatim ahead of the new block.
    let with_existing = merged_content("[[command]]\nname = \"other\"\n", std::slice::from_ref(&entry));
    assert!(with_existing.starts_with("[[command]]\nname = \"other\"\n"));
    assert!(with_existing.contains("name = \"mytool\""));
    // The merged file still parses as a whole (existing + appended), to BOTH commands.
    let specs = load_toml(&with_existing, "suggest-test");
    let names: BTreeSet<&str> = specs.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains("other") && names.contains("mytool"), "merged file lost a command: {names:?}");
}

// ── Outcome classification ────────────────────────────────────────────────────────────────────────

#[test]
fn already_allowed_commands_yield_nothing() {
    assert_eq!(analyze("ls -la"), Outcome::AlreadyAllowed);
    assert_eq!(analyze("echo hello"), Outcome::AlreadyAllowed);
}

#[test]
fn recognized_but_denied_is_not_overridden() {
    // git is recognized; an unknown subcommand is a classification decision, not an unknown command.
    match analyze("git frobnicate-xyz-42") {
        Outcome::RecognizedButDenied { names } => assert!(names.contains(&"git".to_string())),
        other => panic!("expected RecognizedButDenied, got {other:?}"),
    }
}

#[test]
fn unknown_command_is_generated() {
    match analyze("zzmadeuptool-42 --wat foo") {
        Outcome::Generated { entries, .. } => {
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].name, "zzmadeuptool-42");
            assert!(entries[0].standalone.contains(&"--wat".to_string()));
            assert_eq!(entries[0].max_positional, 1);
        }
        other => panic!("expected Generated, got {other:?}"),
    }
}

#[test]
fn mixed_chain_generates_unknown_and_flags_recognized() {
    // An unknown tool chained with a recognized-but-denied command: generate only for the unknown,
    // and report the recognized one so the user knows why the whole chain may still be blocked.
    match analyze("zzmixtool-42 -x && rm -rf /") {
        Outcome::Generated { entries, also_recognized } => {
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].name, "zzmixtool-42");
            assert!(also_recognized.contains(&"rm".to_string()), "rm not reported: {also_recognized:?}");
        }
        other => panic!("expected Generated, got {other:?}"),
    }
}

#[test]
fn unparseable_command_reports_unparseable() {
    // An unbalanced quote can't be fixed by adding a command definition.
    assert_eq!(analyze("zztool 'unterminated"), Outcome::Unparseable);
}

#[test]
fn unknown_command_nested_in_substitution_is_found() {
    // The unknown tool hides inside a command substitution; it must still be surfaced.
    match analyze("echo $(zzhiddentool-42 -a)") {
        Outcome::Generated { entries, .. } => {
            assert!(entries.iter().any(|e| e.name == "zzhiddentool-42"));
        }
        other => panic!("expected Generated, got {other:?}"),
    }
}

#[test]
fn path_basename_is_used_for_the_name() {
    // `/opt/bin/zztool-42` classifies under its basename.
    match analyze("/opt/bin/zztool-42 --flag") {
        Outcome::Generated { entries, .. } => {
            assert!(entries.iter().any(|e| e.name == "zztool-42"), "entries: {entries:?}");
        }
        other => panic!("expected Generated, got {other:?}"),
    }
}

/// Close the loop against the trust check: the written file's hash equals the printed pin's hash,
/// and the pin carries the file's canonical parent dir — exactly the two things
/// `registry::custom::repo_is_trusted` compares. So a user who pastes the pin gets a trusted file.
#[test]
fn generated_pin_matches_the_written_file_and_dir() {
    let dir = tempfile::tempdir().unwrap();
    let entry = GeneratedEntry {
        name: "zztool-42".into(),
        standalone: vec!["-x".into()],
        max_positional: 0,
        level: "SafeWrite".into(),
    };
    let content = merged_content("", std::slice::from_ref(&entry));
    let file = dir.path().join(".safe-chains.toml");
    std::fs::write(&file, &content).unwrap();

    let hash = config_hash(content.as_bytes());
    // What repo_is_trusted recomputes: the hash of the file's raw bytes.
    assert_eq!(config_hash(&std::fs::read(&file).unwrap()), hash);
    // What repo_is_trusted compares the pinned path against: the canonicalized parent dir.
    let canonical_dir = std::fs::canonicalize(dir.path()).unwrap();
    let pin = pin_block(&canonical_dir.to_string_lossy(), &hash);
    assert!(pin.contains(&hash), "pin must carry the file hash");
    assert!(pin.contains(&*canonical_dir.to_string_lossy()), "pin must carry the canonical dir");
}

#[test]
fn toml_str_escapes_quotes_and_backslashes() {
    assert_eq!(toml_str("a\"b\\c"), "\"a\\\"b\\\\c\"");
    assert_eq!(toml_str("plain"), "\"plain\"");
}
