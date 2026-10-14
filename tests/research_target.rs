//! The research reference target (docs/agents/research-target.md): every command safe-chains
//! knows, measured against what the research procedure in AGENTS.md is meant to leave behind.
//!
//! Each command's unmet criteria are pinned in `tests/fixtures/research_target_owed.tsv`, which may
//! only shrink. A command not in the pin must meet every criterion, so a new entry lands at the
//! target; a pinned criterion the command now meets must leave the pin in the same change, so the
//! pin never hides a regression.
//!
//! `UPDATE_RESEARCH_TARGET=1 cargo test --test research_target` rewrites the pin after an
//! improvement, and refuses when anything would be added to it. `RESEARCH_TARGET_REPORT=<path>`
//! also writes every command's state as TSV, for whatever tracks the remaining gap.

#[path = "research_target/criteria.rs"]
mod criteria;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;

const OWED: &str = "tests/fixtures/research_target_owed.tsv";
const UPDATE: &str = "UPDATE_RESEARCH_TARGET";
const REPORT: &str = "RESEARCH_TARGET_REPORT";
const CREATE: &str = "CREATE_RESEARCH_TARGET";

#[derive(Default, Clone)]
struct Command {
    files: BTreeSet<String>,
    aliases: BTreeSet<String>,
    missing: BTreeSet<&'static str>,
    custom: bool,
}

type Owed = BTreeMap<String, BTreeSet<String>>;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn toml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("commands dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            toml_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "toml") && path.file_name().is_some_and(|n| n != "SAMPLE.toml") {
            out.push(path);
        }
    }
}

fn registry() -> BTreeMap<String, Command> {
    let mut files = Vec::new();
    toml_files(&root().join("commands"), &mut files);
    let mut out: BTreeMap<String, Command> = BTreeMap::new();
    for path in files {
        let relative = path.strip_prefix(root()).expect("under root").to_string_lossy().to_string();
        let parsed: Value = toml::from_str(&fs::read_to_string(&path).expect("read toml")).expect("toml parses");
        for entry in parsed.get("command").and_then(Value::as_array).into_iter().flatten() {
            let name = entry.get("name").and_then(Value::as_str).expect("every command has a name").to_string();
            let command = out.entry(name).or_default();
            command.files.insert(relative.clone());
            let aliases = entry.get("aliases").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str);
            command.aliases.extend(aliases.map(str::to_string));
            command.missing.extend(criteria::missing(entry));
            command.custom |= criteria::custom(entry);
        }
    }
    let named: BTreeSet<String> = out.iter().flat_map(|(n, c)| std::iter::once(n.clone()).chain(c.aliases.iter().cloned())).collect();
    // A handler documents several programs under one heading ("awk / gawk / mawk / nawk").
    for doc in safe_chains::docs::all_command_docs() {
        let mut names = doc.name.split(" / ").map(str::trim).filter(|n| !n.is_empty()).map(str::to_string);
        let Some(first) = names.next() else { continue };
        let aliases: BTreeSet<String> = names.chain(doc.aliases).collect();
        if !named.contains(&first) {
            let command = out.entry(first).or_default();
            command.missing.insert(criteria::ENTRY);
            command.custom = true;
            command.aliases.extend(aliases);
        }
    }
    out
}

fn parse_owed(text: &str) -> Owed {
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (name, list) = l.split_once('\t').unwrap_or((l, ""));
            (name.to_string(), list.split(',').filter(|c| !c.is_empty()).map(str::to_string).collect())
        })
        .collect()
}

fn render_owed(owed: &Owed) -> String {
    let mut out = String::from("# command\tunmet criteria (docs/agents/research-target.md); may only shrink\n");
    for (name, list) in owed.iter().filter(|(_, l)| !l.is_empty()) {
        out.push_str(&format!("{name}\t{}\n", list.iter().cloned().collect::<Vec<_>>().join(",")));
    }
    out
}

/// What the registry misses beyond its pin (growth), and what the pin still lists that is now met or
/// gone (stale). Both fail the test; only stale can be fixed by regenerating.
fn ratchet(now: &Owed, pinned: &Owed) -> (Vec<String>, Vec<String>) {
    let empty = BTreeSet::new();
    let mut growth = Vec::new();
    let mut stale = Vec::new();
    for (name, missing) in now {
        let pin = pinned.get(name).unwrap_or(&empty);
        // A command pinned as owing an entry had no entry to judge, so it owed everything: the
        // entry it gains may still miss the other criteria, one step at a time.
        if pin.contains(criteria::ENTRY) && !missing.contains(criteria::ENTRY) {
            continue;
        }
        for c in missing.difference(pin) {
            growth.push(format!("{name}: misses {c}"));
        }
    }
    for (name, pin) in pinned {
        match now.get(name) {
            None => stale.push(format!("{name}: no longer in the registry")),
            Some(missing) => stale.extend(pin.difference(missing).map(|c| format!("{name}: now meets {c}"))),
        }
    }
    (growth, stale)
}

fn as_owed(registry: &BTreeMap<String, Command>) -> Owed {
    registry
        .iter()
        .map(|(n, c)| (n.clone(), c.missing.iter().map(|s| (*s).to_string()).collect()))
        .collect()
}

fn write_report(path: &str, registry: &BTreeMap<String, Command>) {
    let mut out = String::from("name\tat_target\tmissing\tcustom\taliases\tfiles\n");
    for (name, c) in registry {
        let join = |s: &BTreeSet<String>| s.iter().cloned().collect::<Vec<_>>().join(",");
        let missing = c.missing.iter().copied().collect::<Vec<_>>().join(",");
        out.push_str(&format!("{name}\t{}\t{missing}\t{}\t{}\t{}\n", c.missing.is_empty(), c.custom, join(&c.aliases), join(&c.files)));
    }
    fs::write(path, out).expect("write report");
}

fn summary(registry: &BTreeMap<String, Command>) -> String {
    let at = registry.values().filter(|c| c.missing.is_empty()).count();
    let total = registry.len();
    let mut out = format!("research target: {at} of {total} commands at target ({:.1}%)\n", 100.0 * at as f64 / total.max(1) as f64);
    for c in criteria::ALL {
        let owed = registry.values().filter(|x| x.missing.contains(c)).count();
        out.push_str(&format!("  {c:<13} owed by {owed}\n"));
    }
    let custom = registry.values().filter(|c| c.custom).count();
    let custom_at = registry.values().filter(|c| c.custom && c.missing.is_empty()).count();
    out.push_str(&format!("  custom code: {custom_at} of {custom} at target\n"));
    out
}

#[test]
fn every_command_meets_the_research_target_or_is_pinned() {
    let registry = registry();
    assert!(registry.len() > 1000, "found only {} commands; the walk is looking in the wrong place", registry.len());
    assert!(registry.values().any(|c| c.custom), "found no custom-code command; the handler walk is broken");
    if let Ok(path) = std::env::var(REPORT) {
        write_report(&path, &registry);
    }
    println!("{}", summary(&registry));
    let now = as_owed(&registry);
    let Ok(pin_text) = fs::read_to_string(root().join(OWED)) else {
        if std::env::var_os(CREATE).is_some() {
            fs::write(root().join(OWED), render_owed(&now)).expect("write pin");
            return;
        }
        panic!("{OWED} is missing; it is created once, with {CREATE}=1, and only shrinks after that");
    };
    let pinned = parse_owed(&pin_text);
    let (growth, stale) = ratchet(&now, &pinned);
    assert!(growth.is_empty(), "below the research target and not pinned (the pin may only shrink):\n  {}", growth.join("\n  "));
    if std::env::var_os(UPDATE).is_some() {
        fs::write(root().join(OWED), render_owed(&now)).expect("write pin");
        return;
    }
    assert!(
        stale.is_empty(),
        "the pin lists criteria now met; run {UPDATE}=1 cargo test --test research_target:\n  {}",
        stale.join("\n  ")
    );
}

#[cfg(test)]
mod ratchet_tests {
    use super::*;

    fn owed(rows: &[(&str, &[&str])]) -> Owed {
        rows.iter().map(|(n, l)| ((*n).to_string(), l.iter().map(|s| (*s).to_string()).collect())).collect()
    }

    #[test]
    fn an_unpinned_shortfall_is_growth() {
        let (growth, stale) = ratchet(&owed(&[("a", &["version"])]), &owed(&[]));
        assert_eq!(growth, vec!["a: misses version"]);
        assert!(stale.is_empty());
    }

    #[test]
    fn a_met_criterion_must_leave_the_pin() {
        let (growth, stale) = ratchet(&owed(&[("a", &[])]), &owed(&[("a", &["version"])]));
        assert!(growth.is_empty());
        assert_eq!(stale, vec!["a: now meets version"]);
    }

    #[test]
    fn a_removed_command_must_leave_the_pin() {
        let (_, stale) = ratchet(&owed(&[]), &owed(&[("gone", &["entry"])]));
        assert_eq!(stale, vec!["gone: no longer in the registry"]);
    }

    #[test]
    fn the_pin_round_trips_and_drops_complete_commands() {
        let pin = owed(&[("a", &["examples", "version"]), ("b", &[])]);
        let parsed = parse_owed(&render_owed(&pin));
        assert_eq!(parsed, owed(&[("a", &["examples", "version"])]));
    }

    #[test]
    fn a_new_entry_may_owe_the_rest_but_a_lost_one_may_not() {
        let (growth, stale) = ratchet(&owed(&[("a", &["description", "version"])]), &owed(&[("a", &["entry"])]));
        assert!(growth.is_empty());
        assert_eq!(stale, vec!["a: now meets entry"]);
        let (growth, _) = ratchet(&owed(&[("b", &["entry", "version"])]), &owed(&[("b", &["entry"])]));
        assert_eq!(growth, vec!["b: misses version"]);
    }
}
