//! The `standalone` + `valued` overlap guard: a flag in both lists of one scope is a build error
//! unless it is declared `optional_valued`, or is one of the overlaps pinned in
//! `tests/fixtures/standalone_valued_overlap.tsv` before the guard existed.
//!
//! A raw overlap behaves as an optional-value flag (`check_flags` tries `standalone` first, so the
//! bare spelling takes no value; its `=` branch tries `valued`, so the glued spelling does). The
//! same pair of memberships therefore means either "optional value" or "somebody copied the valued
//! list into standalone", and nothing could tell them apart. `optional_valued` says which; the
//! guard makes every new scope say it.

use super::build::build_policy;
use super::types::{TomlCommand, TomlFile, TomlSub};
use crate::parse::Token;
use crate::policy::{FlagTolerance, UnknownTolerance, count_positionals};
use proptest::prelude::*;
use std::collections::BTreeSet;

struct Scope<'a> {
    label: String,
    standalone: &'a [String],
    valued: &'a [String],
}

fn toml_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).expect("read commands dir") {
        let p = e.expect("dir entry").path();
        if p.is_dir() {
            toml_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "toml") {
            out.push(p);
        }
    }
}

fn sub_scopes<'a>(prefix: &str, list: &'a [TomlSub], out: &mut Vec<Scope<'a>>) {
    for sub in list {
        let label = format!("{prefix} {}", sub.name);
        out.push(Scope { label: label.clone(), standalone: &sub.standalone, valued: &sub.valued });
        sub_scopes(&label, &sub.sub, out);
    }
}

fn scopes(cmd: &TomlCommand) -> Vec<Scope<'_>> {
    let mut out = vec![Scope { label: cmd.name.clone(), standalone: &cmd.standalone, valued: &cmd.valued }];
    if let Some(fb) = &cmd.fallback {
        out.push(Scope { label: format!("{} [fallback]", cmd.name), standalone: &fb.standalone, valued: &fb.valued });
    }
    for (key, hp) in &cmd.handler_policy {
        out.push(Scope { label: format!("{} [policy {key}]", cmd.name), standalone: &hp.standalone, valued: &hp.valued });
    }
    if let Some(b) = &cmd.behavior {
        out.push(Scope { label: format!("{} [behavior]", cmd.name), standalone: &b.standalone, valued: &b.valued });
    }
    if let Some(w) = &cmd.wrapper {
        out.push(Scope { label: format!("{} [wrapper]", cmd.name), standalone: &w.standalone, valued: &w.valued });
    }
    sub_scopes(&cmd.name, &cmd.sub, &mut out);
    out
}

/// The flags one scope declares in both `standalone` and `valued`.
fn raw_overlap(standalone: &[String], valued: &[String]) -> BTreeSet<String> {
    standalone.iter().filter(|f| valued.contains(f)).cloned().collect()
}

fn pinned() -> BTreeSet<(String, String)> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/standalone_valued_overlap.tsv");
    let raw = std::fs::read_to_string(&path).expect("read standalone_valued_overlap.tsv");
    raw.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let (scope, flag) = l.split_once('\t').unwrap_or_else(|| panic!("row {l:?} needs <scope>\\t<flag>"));
            (scope.trim().to_string(), flag.trim().to_string())
        })
        .collect()
}

/// Every flag declared in both `standalone` and `valued` of one scope is on the pinned list, and
/// every pinned row is still such a flag.
///
/// The scopes are the command, every nested sub, the fallback grammar, each `handler_policy`
/// block, the `behavior` grammar and the `wrapper`. A flag that genuinely takes an optional value
/// goes in `optional_valued`; one that always takes a value goes in `valued` alone; a boolean goes
/// in `standalone` alone. Nothing is ever added to the pinned list.
#[test]
fn a_flag_is_never_both_standalone_and_valued_in_one_scope() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("commands");
    let mut files = Vec::new();
    toml_files(&root, &mut files);
    let mut walked = 0usize;
    let mut found: BTreeSet<(String, String)> = BTreeSet::new();
    for f in &files {
        let parsed: TomlFile = toml::from_str(&std::fs::read_to_string(f).expect("read toml")).expect("parse toml");
        for cmd in &parsed.command {
            for s in scopes(cmd) {
                walked += 1;
                found.extend(raw_overlap(s.standalone, s.valued).into_iter().map(|flag| (s.label.clone(), flag)));
            }
        }
    }
    assert!(walked > 1000, "only {walked} scopes walked — the registry walk is broken");

    let listed = pinned();
    let new: Vec<String> = found.difference(&listed).map(|(s, f)| format!("  {s}\t{f}")).collect();
    let stale: Vec<String> = listed.difference(&found).map(|(s, f)| format!("  {s}\t{f}")).collect();
    assert!(
        new.is_empty(),
        "these scopes declare a flag in BOTH `standalone` and `valued` ({}). Say which it is: \
         `optional_valued` if the tool takes `--flag` and `--flag=VALUE`, `valued` alone if it always \
         takes a value, `standalone` alone if it never does. Do not add rows to \
         standalone_valued_overlap.tsv:\n{}",
        new.len(),
        new.join("\n"),
    );
    assert!(
        stale.is_empty(),
        "these standalone_valued_overlap.tsv rows are no longer overlaps ({}) — delete them in the \
         same change, so the list only shrinks:\n{}",
        stale.len(),
        stale.join("\n"),
    );
}

fn long_flag() -> impl Strategy<Value = String> {
    "--[a-e]{1,2}"
}

fn tokens(words: &[&str]) -> Vec<Token> {
    words.iter().map(|w| Token::from_test(w)).collect()
}

const STRICT: FlagTolerance = FlagTolerance { unknown: UnknownTolerance::Strict, numeric_dash: false, single_dash_words: false };

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// The only flags a built policy lists as both standalone and valued are the declared
    /// `optional_valued` ones and the raw overlaps of the two lists. So a registry with no raw
    /// overlap (the guard above, once the pins are gone) has a dual-listed flag only where an
    /// author said `optional_valued`.
    #[test]
    fn a_built_policy_lists_a_flag_twice_only_when_declared_optional_or_overlapping(
        standalone in proptest::collection::vec(long_flag(), 0..6),
        valued in proptest::collection::vec(long_flag(), 0..6),
        optional in proptest::collection::vec(long_flag(), 0..6),
    ) {
        let p = build_policy(standalone.clone(), valued.clone(), optional.clone(), None, None, None, None, None);
        let both: BTreeSet<String> = p.standalone.iter().filter(|f| p.valued.contains(f)).cloned().collect();
        let mut expected = raw_overlap(&standalone, &valued);
        expected.extend(optional.iter().cloned());
        prop_assert_eq!(both, expected);
    }

    /// Once the lists are disjoint, each flag has exactly the arity it was declared with: a
    /// `valued` flag takes the next word as its value, a `standalone` flag never does and refuses
    /// a glued `=` value, and an `optional_valued` flag takes a glued `=` value but never the next
    /// word.
    #[test]
    fn a_disjoint_declaration_gives_each_flag_one_arity(
        flags in proptest::collection::btree_set(long_flag(), 1..9),
        split in proptest::collection::vec(0u8..3, 9),
    ) {
        let (mut standalone, mut valued, mut optional) = (Vec::new(), Vec::new(), Vec::new());
        for (flag, kind) in flags.iter().zip(&split) {
            match kind {
                0 => standalone.push(flag.clone()),
                1 => valued.push(flag.clone()),
                _ => optional.push(flag.clone()),
            }
        }
        let p = build_policy(standalone.clone(), valued.clone(), optional.clone(), None, None, None, None, None);
        let (s, v) = (p.standalone.as_slice(), p.valued.as_slice());
        for f in &standalone {
            prop_assert_eq!(count_positionals(&tokens(&["cmd", f, "w"]), s, v, STRICT), Some(1));
            prop_assert_eq!(count_positionals(&tokens(&["cmd", &format!("{f}=w")]), s, v, STRICT), None);
        }
        for f in &valued {
            prop_assert_eq!(count_positionals(&tokens(&["cmd", f, "w"]), s, v, STRICT), Some(0));
            prop_assert_eq!(count_positionals(&tokens(&["cmd", &format!("{f}=w")]), s, v, STRICT), Some(0));
        }
        for f in &optional {
            prop_assert_eq!(count_positionals(&tokens(&["cmd", f, "w"]), s, v, STRICT), Some(1));
            prop_assert_eq!(count_positionals(&tokens(&["cmd", &format!("{f}=w")]), s, v, STRICT), Some(0));
        }
    }
}
