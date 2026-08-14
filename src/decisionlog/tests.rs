use super::*;

fn ctx<'a>(command: &'a str) -> Context<'a> {
    Context {
        command,
        cwd: Some("/w"),
        root: Some("/w"),
        session_id: Some("sess-1"),
        harness: "claude",
        level: "developer",
    }
}

fn entry(outcome: Outcome, command: &str) -> Value {
    let explanation = crate::cst::explain(command);
    build_entry(outcome, &ctx(command), Some(&explanation))
}

/// The mode gate is the whole of `--log`'s advantage: it must be decided before anything is built.
#[test]
fn a_mode_keeps_exactly_the_outcomes_it_advertises() {
    use Outcome::*;
    for o in [Allowed, Denied, Abstained, Unparseable] {
        assert!(!Mode::Off.keeps(o), "Off kept {o}");
        assert!(Mode::Everything.keeps(o), "Everything dropped {o}");
    }
    assert!(!Mode::NonApprovals.keeps(Allowed), "--log kept an approval");
    for o in [Denied, Abstained, Unparseable] {
        assert!(Mode::NonApprovals.keeps(o), "--log dropped {o}");
    }
}

/// `--log-everything` wins when both are passed: it is the strictly wider request, so honouring it
/// cannot lose an entry the user asked for.
#[test]
fn the_wider_flag_wins_when_both_are_given() {
    assert_eq!(Mode::from_flags(false, false), Mode::Off);
    assert_eq!(Mode::from_flags(true, false), Mode::NonApprovals);
    assert_eq!(Mode::from_flags(false, true), Mode::Everything);
    assert_eq!(Mode::from_flags(true, true), Mode::Everything);
}

/// Every entry carries the full documented field set. A missing field is invisible to `jq` — it
/// reads as `null` — so absence has to fail here rather than at a reader months later.
#[test]
fn an_entry_carries_every_documented_field() {
    const FIELDS: &[&str] = &[
        "schema", "id", "at", "version", "harness", "outcome", "level", "command", "cwd", "root",
        "session_id", "triage", "unknown_commands", "segments", "stateful",
    ];
    for outcome in [Outcome::Allowed, Outcome::Denied, Outcome::Unparseable] {
        let e = entry(outcome, "ls -la");
        let obj = e.as_object().expect("entry is an object");
        for f in FIELDS {
            assert!(obj.contains_key(*f), "{outcome}: missing `{f}`");
        }
        assert_eq!(obj.len(), FIELDS.len(), "{outcome}: unexpected extra fields");
    }
}

/// An approval owes no triage and no facets — there is no refusal to explain and no registry gap.
/// This is what keeps `--log-everything` affordable, so it is pinned rather than assumed.
#[test]
fn an_approval_records_no_triage_and_no_facets() {
    let e = entry(Outcome::Allowed, "ls -la");
    assert_eq!(e["triage"], "allowed");
    assert_eq!(e["unknown_commands"], json!([]));
    assert_eq!(e["segments"][0]["facets"], Value::Null, "an approved segment owes no reason");
}

/// The highest-value field in the file: an unknown command is a registry gap worth an issue, a
/// recognized one is a classification decision. Getting these backwards mis-files every report.
#[test]
fn triage_splits_a_registry_gap_from_a_classification() {
    let gap = entry(Outcome::Denied, "definitelynotarealtool --frobnicate");
    assert_eq!(gap["triage"], "unknown-command");
    assert_eq!(gap["unknown_commands"], json!(["definitelynotarealtool"]));

    let decision = entry(Outcome::Denied, "aws dynamodb put-item --table-name t --item {}");
    assert_eq!(decision["triage"], "recognized-but-denied");
    assert_eq!(decision["unknown_commands"], json!([]));
}

/// A refusal the engine resolved carries the axis it was refused on — the answer to "is this
/// classification right", which is the question the log exists to make answerable later.
#[test]
fn a_resolved_refusal_records_the_clause_that_refused_it() {
    let e = entry(Outcome::Denied, "aws dynamodb put-item --table-name t --item {}");
    let refused = &e["segments"][0]["facets"]["refused_by"];
    assert!(refused.is_object(), "expected a refusing clause, got {refused}");
    assert_eq!(refused["level"], "developer");
    assert!(
        refused["clause"].as_str().is_some_and(|c| c.contains("locus.remote")),
        "clause did not name the axis: {refused}"
    );
    assert!(
        e["segments"][0]["facets"]["capabilities"].as_array().is_some_and(|c| !c.is_empty())
    );
}

/// A chain gets one segment per top-level element, with the culprit named. Reproducing "which part
/// of this failed" by hand was the tedious half of the triage this feature replaces.
#[test]
fn a_chain_records_each_segment_and_its_verdict() {
    let e = entry(Outcome::Denied, "ls && definitelynotarealtool x");
    let segs = e["segments"].as_array().expect("segments array");
    assert_eq!(segs.len(), 2);
    assert_eq!(segs[0]["text"], "ls");
    assert_eq!(segs[0]["verdict"], "allowed");
    assert_eq!(segs[1]["verdict"], "denied");
    assert_eq!(segs[0]["facets"], Value::Null, "an allowed segment owes no reason");
    // An UNKNOWN command has no facets either, and that is not a gap: no resolver claimed it, so
    // there is no profile to report. `triage`/`unknown_commands` carry the reason for this class.
    assert_eq!(segs[1]["facets"], Value::Null, "an unknown command has no profile to report");
}

/// The case a whole-command `facets` field could never serve. The engine resolves one command at a
/// time, so on a CHAIN the old top-level field was always null — exactly when "which segment, and
/// why" is the question being asked. The reason now rides on the segment that earned it.
#[test]
fn a_denied_segment_in_a_chain_carries_its_own_reason() {
    let e = entry(Outcome::Denied, "ls && aws dynamodb put-item --table-name t --item {}");
    let segs = e["segments"].as_array().expect("segments array");
    assert_eq!(segs[0]["verdict"], "allowed");
    assert_eq!(segs[0]["facets"], Value::Null);
    assert_eq!(segs[1]["verdict"], "denied");
    assert_eq!(
        segs[1]["facets"]["refused_by"]["clause"], "locus.remote = fixed (allowed: none..=none)",
        "the denied segment must name the axis that refused IT, not the whole chain"
    );
}

/// The id's hash half groups repeats of one command without any dedup machinery, so it must depend
/// on the command and nothing else.
#[test]
fn the_id_hash_is_stable_per_command() {
    let hash = |cmd: &str| {
        let id = entry(Outcome::Denied, cmd)["id"].as_str().unwrap().to_string();
        id.split_once('-').unwrap().1.to_string()
    };
    assert_eq!(hash("ls -la"), hash("ls -la"));
    assert_ne!(hash("ls -la"), hash("ls -lah"));
}

/// Every entry is one line of valid JSON. A torn or multi-line entry breaks every `jq` consumer for
/// the whole file, so the serialized form is checked, not just the value.
#[test]
fn an_entry_serializes_to_exactly_one_line() {
    for cmd in ["ls", "ls && rm -rf /", "echo \"unterminated", "curl -X POST https://x.test"] {
        let line = serde_json::to_string(&entry(Outcome::Denied, cmd)).expect("serializes");
        assert!(!line.contains('\n'), "embedded newline for {cmd:?}");
        serde_json::from_str::<Value>(&line).expect("round-trips");
    }
}

/// A command with no explanation (the caller had not computed one) still produces a well-formed
/// entry — the optional argument must not become an unwrap.
#[test]
fn an_entry_without_an_explanation_is_still_well_formed() {
    let e = build_entry(Outcome::Allowed, &ctx("ls"), None);
    assert_eq!(e["segments"], json!([]));
    assert_eq!(e["stateful"], json!(false));
    serde_json::to_string(&e).expect("serializes");
}

/// Rotation is the only path here that DESTROYS data, and it shipped untested because the real cap
/// is 16 MB. With the cap and the generation count as parameters it is cheap to pin the whole
/// cycle: nothing moves below the cap, the generations shift down in order, and only the oldest is
/// ever lost.
#[test]
fn rotation_shifts_generations_and_drops_only_the_oldest() {
    const KEEP: usize = 3;
    let dir = std::env::temp_dir().join(format!("sc-rotate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("log.jsonl");
    let nth = |n: usize| dir.join(format!("log.jsonl.{n}"));
    let read = |p: &std::path::Path| std::fs::read_to_string(p).ok();

    // Under the cap: untouched.
    std::fs::write(&path, "gen1\n").expect("write");
    rotate_at(&path, 1024, KEEP);
    assert!(path.exists() && !nth(1).exists(), "rotated below the cap");

    // Each rotation pushes the current file to `.1` and everything else one place older.
    // (`round`, not `gen` — the latter is a reserved keyword in edition 2024.)
    for round in 1..=KEEP {
        std::fs::write(&path, format!("gen{round}\n")).expect("write");
        rotate_at(&path, 1, KEEP);
        assert!(!path.exists(), "the current file should have been moved aside");
        for age in 1..=round {
            let expected = format!("gen{}\n", round + 1 - age);
            assert_eq!(read(&nth(age)), Some(expected), "generation {age} after {round} rotations");
        }
    }

    // One more: the oldest is dropped, the rest still shift, and nothing accumulates past the count.
    std::fs::write(&path, "gen4\n").expect("write");
    rotate_at(&path, 1, KEEP);
    assert_eq!(read(&nth(1)), Some("gen4\n".to_string()));
    assert_eq!(read(&nth(KEEP)), Some("gen2\n".to_string()), "gen1 should have aged out");
    assert!(!nth(KEEP + 1).exists(), "kept more generations than asked for");

    // A missing file is not an error — the first write of a fresh install goes through this.
    let _ = std::fs::remove_file(&path);
    rotate_at(&path, 1, KEEP);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rfc3339_matches_known_instants() {
    assert_eq!(rfc3339_utc(0), "1970-01-01T00:00:00.000Z");
    assert_eq!(rfc3339_utc(1_000), "1970-01-01T00:00:01.000Z");
    // The instant used throughout the design doc's samples. Cross-checked against
    // `TZ=UTC date -r 1786664461`, because the first constant written into those samples was
    // invented and disagreed with its own `at` string by two days — an `id` whose timestamp half
    // contradicts `at` is exactly the kind of thing a reader would copy and trust.
    assert_eq!(rfc3339_utc(1_786_664_461_233), "2026-08-13T23:41:01.233Z");
    // Leap day, and the century rule that a naive /4 gets wrong.
    assert_eq!(rfc3339_utc(1_709_164_800_000), "2024-02-29T00:00:00.000Z");
    assert_eq!(rfc3339_utc(951_782_400_000), "2000-02-29T00:00:00.000Z");
}

/// Round-trip the day arithmetic across a long span, which catches an off-by-one that a handful of
/// spot dates would not: every day must map to a date that maps back to the same day.
#[test]
fn civil_from_days_round_trips_over_a_century() {
    fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
        let y = if m <= 2 { y - 1 } else { y };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = (y - era * 400) as u64;
        let mp = if m > 2 { m - 3 } else { m + 9 } as u64;
        let doy = (153 * mp + 2) / 5 + u64::from(d) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe as i64 - 719_468
    }
    for z in -25_000..25_000i64 {
        let (y, m, d) = civil_from_days(z);
        assert_eq!(days_from_civil(y, m, d), z, "round-trip failed at day {z}");
        assert!((1..=12).contains(&m) && (1..=31).contains(&d), "bad date at day {z}");
    }
}
