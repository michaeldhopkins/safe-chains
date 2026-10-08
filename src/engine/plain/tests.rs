use super::*;
use crate::engine::level::{Clause, Level, OrdBound};

/// The axis names `--explain` prints, read from `Capability::set_facets` itself, so an axis added
/// there without a vocabulary entry fails here.
fn printed_axes() -> Vec<String> {
    let source = include_str!("../facet.rs");
    let body = source.split("pub fn set_facets").nth(1).expect("set_facets exists");
    let body = body.split("\n    }\n").next().expect("set_facets ends");
    let mut names: Vec<String> = Vec::new();
    for marker in ["push!(\"", "out.push((\""] {
        for piece in body.split(marker).skip(1) {
            names.push(piece.split('"').next().expect("a quoted name").to_string());
        }
    }
    names.sort();
    names.dedup();
    names
}

#[test]
fn the_vocabulary_covers_every_axis_explain_prints() {
    let mut ours: Vec<String> = vocabulary().into_iter().map(|(a, _)| a.to_string()).collect();
    ours.sort();
    let printed = printed_axes();
    assert!(printed.len() >= 25, "only {} axes found in set_facets; the parse broke", printed.len());
    assert_eq!(ours, printed);
}

#[test]
fn every_term_of_every_axis_has_a_phrase() {
    let mut missing = Vec::new();
    for (axis, terms) in vocabulary() {
        for t in terms {
            if term(axis, t).is_none() {
                missing.push(format!("{axis} = {t}"));
            }
        }
    }
    assert!(missing.is_empty(), "no plain phrase for:\n{}", missing.join("\n"));
    let known: Vec<(&str, &str)> = vocabulary().into_iter().flat_map(|(a, ts)| ts.into_iter().map(move |t| (a, t))).collect();
    for (axis, t, _) in TERMS {
        assert!(known.contains(&(*axis, *t)), "a phrase for {axis} = {t}, which the code no longer has");
    }
}

#[test]
fn every_operation_in_every_place_has_one_phrase() {
    let operations: Vec<&str> = Operation::all().iter().map(|o| o.as_str()).collect();
    let places: Vec<&str> = LocalLocus::all().iter().map(|l| l.as_str()).collect();
    for o in &operations {
        for p in &places {
            let found = ACTIONS.iter().filter(|(ao, ap, _)| ao == o && ap == p).count();
            assert_eq!(found, 1, "{o} in {p}: {found} phrases");
        }
    }
    assert_eq!(ACTIONS.len(), operations.len() * places.len(), "a phrase for a combination the code no longer has");
}

#[test]
fn the_phrases_are_plain() {
    for phrase in ACTIONS.iter().map(|(_, _, p)| p).chain(TERMS.iter().map(|(_, _, p)| p)) {
        assert!(!phrase.contains(['—', ';', '`']), "{phrase}");
        assert!(phrase.len() <= 90, "{phrase}");
        for word in crate::refusal::AVOID.iter().chain(&["locus", "facet", "worktree", "principal", "ambient", "n/a"]) {
            assert!(!phrase.to_lowercase().contains(word), "`{word}` in {phrase}");
        }
    }
}

/// Writes the dictionary as Markdown to the path in SAFE_CHAINS_WRITE_PLAIN_TABLE, for review.
#[test]
fn the_dictionary_can_be_written_out() {
    let mut out = String::from("## What a command does, by where it does it\n\n| Operation | Place | Plain words |\n| --- | --- | --- |\n");
    for (o, p, phrase) in ACTIONS {
        out.push_str(&format!("| {o} | {p} | {phrase} |\n"));
    }
    out.push_str("\n## Every other detail\n\n| Detail | Value | Plain words |\n| --- | --- | --- |\n");
    for (a, t, phrase) in TERMS {
        out.push_str(&format!("| {a} | {t} | {phrase} |\n"));
    }
    if let Some(path) = std::env::var_os("SAFE_CHAINS_WRITE_PLAIN_TABLE") {
        std::fs::write(path, &out).expect("write the dictionary");
    }
    assert_eq!(out.lines().filter(|l| l.starts_with("| ")).count(), ACTIONS.len() + TERMS.len() + 4);
}

proptest::proptest! {
    /// Whatever capability the engine resolves, every facet `set_facets` prints has a phrase, and
    /// so does what it does where.
    #[test]
    fn every_printed_facet_of_any_capability_has_plain_words(op in proptest::sample::select(Operation::all()), place in proptest::sample::select(LocalLocus::all()), worst in proptest::bool::ANY) {
        let mut cap = if worst { Capability::worst("probe") } else { Capability::default() };
        cap.operation = op;
        cap.locus.local = place;
        for (axis, value) in cap.set_facets() {
            proptest::prop_assert!(term(axis, value).is_some(), "{} = {}", axis, value);
        }
        proptest::prop_assert!(action(cap.operation.as_str(), cap.locus.local.as_str()).is_some());
    }
}

/// Rows awaiting the owner's approval of their wording. They print today's term until a phrase
/// replaces the empty string; a row leaves this list in the change that approves it.
const HELD: &[(&str, &str, &str)] = &[
    ("action", "observe", "machine"),
    ("action", "create", "machine"),
    ("action", "mutate", "machine"),
    ("action", "destroy", "machine"),
    ("action", "execute", "machine"),
    ("action", "communicate", "machine"),
    ("term", "locus.local", "machine"),
    ("term", "locus.remote", "none"),
    ("term", "locus.binding", "n/a"),
    ("term", "locus.provenance", "n/a"),
    ("term", "reversibility", "trivial"),
    ("term", "reversibility", "recoverable"),
    ("term", "reversibility", "effortful"),
];

#[test]
fn only_the_held_rows_are_empty_and_every_held_row_is_empty() {
    let mut empty: Vec<(&str, &str, &str)> = ACTIONS.iter().filter(|(_, _, p)| is_held(p)).map(|(o, l, _)| ("action", *o, *l)).collect();
    empty.extend(TERMS.iter().filter(|(_, _, p)| is_held(p)).map(|(a, t, _)| ("term", *a, *t)));
    let mut held = HELD.to_vec();
    empty.sort();
    held.sort();
    assert_eq!(empty, held);
}

fn caps(op: Operation, place: LocalLocus) -> Capability {
    let mut cap = Capability { operation: op, ..Capability::default() };
    cap.locus.local = place;
    cap
}

#[test]
fn an_approved_place_prints_one_row_of_what_the_command_does() {
    let mut cap = caps(Operation::Destroy, LocalLocus::Worktree);
    cap.scale = Scale::Unbounded;
    cap.reversibility = Reversibility::Irreversible;
    let rows = rows(&cap.set_facets());
    assert_eq!(
        rows,
        vec![
            ("does".to_string(), "deletes files in your project".to_string()),
            ("scale".to_string(), "everything it finds, without limit".to_string()),
            ("reversibility".to_string(), "cannot be undone".to_string()),
        ]
    );
}

#[test]
fn a_command_that_stays_in_its_process_still_says_what_it_does() {
    let rows = rows(&caps(Operation::Observe, LocalLocus::Process).set_facets());
    assert_eq!(rows, vec![("does".to_string(), "looks only at its own input".to_string())]);
}

#[test]
fn a_held_place_prints_the_two_rows_it_always_did() {
    let mut cap = caps(Operation::Destroy, LocalLocus::Machine);
    cap.reversibility = Reversibility::Effortful;
    let rows = rows(&cap.set_facets());
    assert_eq!(
        rows,
        vec![
            ("operation".to_string(), "destroy".to_string()),
            ("locus.local".to_string(), "machine".to_string()),
            ("reversibility".to_string(), "effortful".to_string()),
        ]
    );
}

fn miss(clause: Clause, cap: &Capability) -> String {
    mismatch(&Level::new("probe").allowing(clause).nearest_miss(cap).expect("the clause refuses it"))
}

#[test]
fn a_refusal_says_what_the_command_does_and_what_the_level_allows() {
    let talks = caps(Operation::Communicate, LocalLocus::Process);
    let reads_or_writes = Clause { operation: Some(vec![Operation::Observe, Operation::Create]), ..Default::default() };
    assert_eq!(miss(reads_or_writes, &talks), "talks to another system (allowed: one of: reads or looks / creates)");

    let home = caps(Operation::Observe, LocalLocus::User);
    let project = Clause { local_locus: Some(OrdBound::at_most(LocalLocus::Worktree)), ..Default::default() };
    assert_eq!(miss(project, &home), "your home folder (allowed up to: your project)");

    let floor = Clause { isolation: Some(OrdBound::at_least(Isolation::Namespace)), ..Default::default() };
    assert_eq!(miss(floor, &home), "with no sandbox (allowed at least: in a container)");
}

#[test]
fn a_refusal_keeps_the_term_while_its_wording_is_held() {
    let outside = caps(Operation::Observe, LocalLocus::Machine);
    let project = Clause { local_locus: Some(OrdBound::at_most(LocalLocus::Worktree)), ..Default::default() };
    assert_eq!(miss(project, &outside), "locus.local = machine (allowed: <= worktree)");

    let mut reaches = caps(Operation::Communicate, LocalLocus::Process);
    reaches.locus.remote = RemoteReach::Fixed;
    let nowhere =
        Clause { remote_reach: Some(OrdBound { min: Some(RemoteReach::None), max: Some(RemoteReach::None) }), ..Default::default() };
    assert_eq!(miss(nowhere, &reaches), "locus.remote = fixed (allowed: none..=none)");
}

#[test]
fn a_refusal_that_is_not_about_a_facet_reads_as_before() {
    let m = Level::new("empty").nearest_miss(&Capability::default()).expect("nothing is allowed");
    assert_eq!(mismatch(&m), m.to_string());
}

proptest::proptest! {
    /// Whatever the engine resolves, no row is empty, none reads a dotted axis name where the
    /// operation or place was, and a row appears for every other facet.
    #[test]
    fn rows_are_never_empty_and_account_for_every_facet(op in proptest::sample::select(Operation::all()), place in proptest::sample::select(LocalLocus::all()), worst in proptest::bool::ANY) {
        let mut cap = if worst { Capability::worst("probe") } else { Capability::default() };
        cap.operation = op;
        cap.locus.local = place;
        let facets = cap.set_facets();
        let printed = rows(&facets);
        for (name, words) in &printed {
            proptest::prop_assert!(!name.is_empty() && !words.is_empty());
        }
        let others = facets.iter().filter(|(a, _)| *a != "operation" && *a != "locus.local").count();
        let merged = printed.iter().any(|(n, _)| n == "does");
        proptest::prop_assert_eq!(merged, action(op.as_str(), place.as_str()).is_some_and(|p| !is_held(p)));
        let expected = if merged { others + 1 } else { facets.len() };
        proptest::prop_assert_eq!(printed.len(), expected);
        if merged {
            proptest::prop_assert!(printed.iter().all(|(n, _)| n != "operation" && n != "locus.local"));
        }
    }
}

#[test]
fn a_bound_naming_a_held_term_keeps_todays_line() {
    let mut cap = caps(Operation::Observe, LocalLocus::Process);
    cap.reversibility = Reversibility::Irreversible;
    let cap_at_backup = Clause { reversibility: Some(OrdBound::at_most(Reversibility::Effortful)), ..Default::default() };
    assert_eq!(miss(cap_at_backup, &cap), "reversibility = irreversible (allowed: <= effortful)");
}

#[test]
fn a_two_sided_bound_names_both_ends() {
    let cap = caps(Operation::Observe, LocalLocus::Kernel);
    let band = Clause {
        local_locus: Some(OrdBound { min: Some(LocalLocus::SandboxScope), max: Some(LocalLocus::Worktree) }),
        ..Default::default()
    };
    assert_eq!(miss(band, &cap), "the operating system's core (allowed: this session's scratch folder to your project)");
}

#[test]
fn an_unbounded_clause_never_refuses() {
    let cap = caps(Operation::Observe, LocalLocus::Kernel);
    assert!(Level::new("open").allowing(Clause::default()).nearest_miss(&cap).is_none());
}
