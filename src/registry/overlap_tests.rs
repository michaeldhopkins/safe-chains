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
use super::types::{DispatchKind, SubSpec, TomlCommand, TomlFile, TomlSub};
use crate::parse::Token;
use crate::policy::{FlagTolerance, UnknownTolerance, count_positionals};
use proptest::prelude::*;
use std::collections::BTreeSet;

struct Scope {
    label: String,
    standalone: Vec<String>,
    valued: Vec<String>,
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

fn scope(label: String, standalone: &[String], valued: &[String]) -> Scope {
    Scope { label, standalone: standalone.to_vec(), valued: valued.to_vec() }
}

/// The `first_arg` grammar: `first_arg_loopback_valued` has the same arity as `first_arg_valued`.
fn first_arg_scope(label: &str, standalone: &[String], valued: &[String], loopback: &[String]) -> Scope {
    let valued = valued.iter().chain(loopback).cloned().collect();
    Scope { label: format!("{label} [first_arg]"), standalone: standalone.to_vec(), valued }
}

fn sub_scopes(prefix: &str, list: &[TomlSub], out: &mut Vec<Scope>) {
    for sub in list {
        let label = format!("{prefix} {}", sub.name);
        let valued: Vec<String> = sub.valued.iter().chain(&sub.loopback_valued).cloned().collect();
        out.push(scope(label.clone(), &sub.standalone, &valued));
        out.push(first_arg_scope(&label, &sub.first_arg_standalone, &sub.first_arg_valued, &sub.first_arg_loopback_valued));
        sub_scopes(&label, &sub.sub, out);
    }
}

fn scopes(cmd: &TomlCommand) -> Vec<Scope> {
    let name = &cmd.name;
    let mut out = vec![
        scope(name.clone(), &cmd.standalone, &cmd.valued),
        first_arg_scope(name, &cmd.first_arg_standalone, &cmd.first_arg_valued, &cmd.first_arg_loopback_valued),
    ];
    if let Some(fb) = &cmd.fallback {
        out.push(scope(format!("{name} [fallback]"), &fb.standalone, &fb.valued));
    }
    for (key, hp) in &cmd.handler_policy {
        out.push(scope(format!("{name} [policy {key}]"), &hp.standalone, &hp.valued));
    }
    if let Some(b) = &cmd.behavior {
        out.push(scope(format!("{name} [behavior]"), &b.standalone, &b.valued));
    }
    if let Some(w) = &cmd.wrapper {
        out.push(scope(format!("{name} [wrapper]"), &w.standalone, &w.valued));
    }
    if let Some(vc) = &cmd.verb_chain {
        out.push(scope(format!("{name} [verb_chain]"), &vc.main_standalone, &vc.main_valued));
    }
    sub_scopes(name, &cmd.sub, &mut out);
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
/// The scopes are the grammars that pair a standalone list with a valued one: the command and
/// every nested sub (a sub's `loopback_valued` counts as valued), the `first_arg` grammar of each,
/// the fallback grammar, each `handler_policy` block, the `behavior` grammar, the `wrapper` and
/// the `verb_chain` main flags. A flag that genuinely takes an optional value goes in
/// `optional_valued`; one that always takes a value goes in `valued` alone; a boolean goes in
/// `standalone` alone. Nothing is ever added to the pinned list.
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
                found.extend(raw_overlap(&s.standalone, &s.valued).into_iter().map(|flag| (s.label.clone(), flag)));
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

/// The flag lists a dispatch walks to find where the subcommand, verb or inner command starts:
/// the pre-flags of every `Branching` level, a `Wrapper`'s flags and a verb chain's main flags.
fn boundary_lists<'a>(label: &str, kind: &'a DispatchKind, out: &mut Vec<(String, &'a [String], &'a [String])>) {
    let subs: &[SubSpec] = match kind {
        DispatchKind::Branching { subs, pre_standalone, pre_valued, .. } => {
            out.push((format!("{label} [pre-flags]"), pre_standalone, pre_valued));
            subs
        }
        DispatchKind::Custom { subs, .. } => subs,
        DispatchKind::Wrapper { standalone, valued, .. } => {
            out.push((format!("{label} [wrapper]"), standalone, valued));
            return;
        }
        DispatchKind::VerbChain(vc) => {
            out.push((format!("{label} [verb_chain]"), &vc.main_standalone, &vc.main_valued));
            return;
        }
        _ => return,
    };
    for sub in subs {
        boundary_lists(&format!("{label} {}", sub.name), &sub.kind, out);
    }
}

/// No flag in front of a subcommand, a verb or a wrapped command is both standalone and valued,
/// in any built command, with no pinned exceptions.
///
/// Those walks decide where the next word belongs. A flag in both lists is read as taking a
/// value, so the word after it is skipped and the one after that is dispatched as the
/// subcommand: `borg --bypass-lock delete info ./repo` was judged as `info` while borg ran
/// `delete`. Read the other way it is just as wrong for a flag that does take a value. Walks the
/// BUILT registry, so a list a sub inherits from its wrapper is covered as well as one it declares.
#[test]
fn no_flag_before_a_subcommand_or_wrapped_command_is_both_standalone_and_valued() {
    let mut lists = Vec::new();
    for (name, spec) in super::TOML_REGISTRY.iter().chain(super::CUSTOM_REGISTRY.iter()) {
        boundary_lists(name, &spec.kind, &mut lists);
    }
    assert!(lists.len() > 300, "only {} boundary flag lists walked — the registry walk is broken", lists.len());
    let mut found: Vec<String> = lists
        .iter()
        .flat_map(|(label, standalone, valued)| raw_overlap(standalone, valued).into_iter().map(move |f| format!("  {label}\t{f}")))
        .collect();
    found.sort();
    found.dedup();
    assert!(
        found.is_empty(),
        "these flags sit in front of a subcommand, verb or wrapped command and are declared both \
         standalone and valued ({}). Read the tool's option table and keep the flag in one list:\n{}",
        found.len(),
        found.join("\n"),
    );
}

fn long_flag() -> impl Strategy<Value = String> {
    "--[a-e]{1,2}"
}

fn tokens(words: &[&str]) -> Vec<Token> {
    words.iter().map(|w| Token::from_test(w)).collect()
}

const STRICT: FlagTolerance =
    FlagTolerance { unknown: UnknownTolerance::Strict, numeric_dash: false, single_dash_words: false, ..FlagTolerance::strict() };

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

    /// The short-flag arm: a disjoint `valued` letter takes the next word or the rest of its
    /// cluster as its value (`-o w`, `-ow`, `-so w`), a `standalone` letter never takes a value
    /// and leaves the rest of the cluster to be read as flags, and an `optional_valued` letter,
    /// which can only be spelled with `=` when long, takes no value in either short spelling.
    #[test]
    fn a_disjoint_short_declaration_gives_each_letter_one_arity(
        letters in proptest::collection::btree_set(proptest::char::range('a', 'e'), 1..6),
        split in proptest::collection::vec(0u8..3, 6),
    ) {
        let (mut standalone, mut valued, mut optional) = (Vec::new(), Vec::new(), Vec::new());
        for (letter, kind) in letters.iter().zip(&split) {
            match kind {
                0 => standalone.push(format!("-{letter}")),
                1 => valued.push(format!("-{letter}")),
                _ => optional.push(format!("-{letter}")),
            }
        }
        let p = build_policy(standalone.clone(), valued.clone(), optional.clone(), None, None, None, None, None);
        let (s, v) = (p.standalone.as_slice(), p.valued.as_slice());
        for f in standalone.iter().chain(&optional) {
            prop_assert_eq!(count_positionals(&tokens(&["cmd", f, "w"]), s, v, STRICT), Some(1));
            prop_assert_eq!(count_positionals(&tokens(&["cmd", &format!("{f}w")]), s, v, STRICT), None);
        }
        for f in &valued {
            prop_assert_eq!(count_positionals(&tokens(&["cmd", f, "w"]), s, v, STRICT), Some(0));
            prop_assert_eq!(count_positionals(&tokens(&["cmd", &format!("{f}w")]), s, v, STRICT), Some(0));
            for b in &standalone {
                prop_assert_eq!(count_positionals(&tokens(&["cmd", &format!("{b}{}", &f[1..]), "w"]), s, v, STRICT), Some(0));
                prop_assert_eq!(count_positionals(&tokens(&["cmd", &format!("{f}{}", &b[1..]), "w"]), s, v, STRICT), Some(1));
            }
        }
    }
}

/// A flag a `Branching` command lists in front of its subcommands is standalone for the path gate
/// whether or not any subcommand lists it too, and so is a flag only a subcommand lists.
#[test]
fn a_branching_command_declares_its_pre_flags_and_its_subcommands_flags_standalone() {
    let mut checked = 0;
    for spec in super::TOML_REGISTRY.values().chain(super::CUSTOM_REGISTRY.values()) {
        let DispatchKind::Branching { subs, pre_standalone, .. } = &spec.kind else { continue };
        let sub_flags = subs.iter().filter_map(|sub| match &sub.kind {
            DispatchKind::Policy { policy, .. } => Some(policy.standalone.as_slice()),
            _ => None,
        });
        for flag in pre_standalone.iter().chain(sub_flags.flatten()) {
            assert!(spec.kind.declares_standalone(flag), "{}: {flag} is listed standalone but not declared so", spec.name);
            checked += 1;
        }
    }
    assert!(checked > 100, "only {checked} branching flags checked — the registry walk is broken");
}

/// A flag a `Custom` command's handler policy lists standalone is standalone for the path gate
/// whether or not a TOML subcommand lists it too, and so is a flag only a subcommand lists.
#[test]
fn a_custom_command_declares_its_handler_flags_and_its_subcommands_flags_standalone() {
    let mut checked = 0;
    for spec in super::TOML_REGISTRY.values().chain(super::CUSTOM_REGISTRY.values()) {
        let DispatchKind::Custom { subs, handler_policies, .. } = &spec.kind else { continue };
        let sub_flags = subs.iter().filter_map(|sub| match &sub.kind {
            DispatchKind::Policy { policy, .. } => Some(policy.standalone.as_slice()),
            _ => None,
        });
        let handler_flags = handler_policies.values().map(|policy| policy.standalone.as_slice());
        for flag in handler_flags.flatten().chain(sub_flags.flatten()) {
            assert!(spec.kind.declares_standalone(flag), "{}: {flag} is listed standalone but not declared so", spec.name);
            checked += 1;
        }
    }
    assert!(checked > 20, "only {checked} custom flags checked — the registry walk is broken");
}
