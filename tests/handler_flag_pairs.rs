//! A hand-written handler that walks its own flags reads each one with a single arity.
//!
//! A handler holding a standalone list and a valued list for one grammar (`MVN_STANDALONE` and
//! `MVN_VALUED`, `XARGS_FLAGS_NO_ARG` and `XARGS_FLAGS_WITH_ARG`) decides from them where the
//! next word belongs. A flag in both is read by whichever list the walk tries first, so the word
//! after it is judged as a value or as an operand, whatever the tool does with it. The registry's
//! own lists are checked by `registry::overlap_tests`; this scans the handler sources, so a pair
//! added to a new handler is covered the day it lands. A standalone list is compared with every
//! other list its file keeps for the same tool (`MVN_LOG_FILES`, `MVN_DEFINES`), not only the one
//! whose name ends in `_VALUED`.

use std::collections::{BTreeMap, BTreeSet};

const SUFFIXES: [(&str, bool); 4] = [("_STANDALONE", false), ("_NO_ARG", false), ("_VALUED", true), ("_WITH_ARG", true)];

const OPTIONAL_VALUED: &[(&str, &str, &str)] = &[("coreutils/grep.rs", "GREP", "--color"), ("coreutils/grep.rs", "GREP", "--colour")];

fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read handlers dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push(path);
        }
    }
}

fn quoted(text: &str) -> BTreeSet<String> {
    text.split('"').skip(1).step_by(2).map(str::to_string).collect()
}

fn list_name(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let rest = ["pub(super) ", "pub(crate) ", "pub "].iter().find_map(|p| line.strip_prefix(p)).unwrap_or(line);
    let rest = rest.strip_prefix("static ").or_else(|| rest.strip_prefix("const "))?;
    let (name, ty) = rest.split_once(':')?;
    let ty = ty.trim_start();
    (ty.starts_with("WordSet") || ty.starts_with("&[&str]")).then_some(name.trim())
}

fn flag_lists(source: &str) -> Vec<(String, BTreeSet<String>)> {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let Some(name) = list_name(line) else { continue };
        let end = (i..lines.len()).find(|&j| lines[j].trim_end().ends_with(';')).unwrap_or(i);
        out.push((name.to_string(), quoted(&lines[i..=end].join("\n"))));
    }
    out
}

type Pairs = BTreeMap<(String, String), [Option<BTreeSet<String>>; 2]>;

fn standalone_stem(name: &str) -> Option<String> {
    SUFFIXES
        .iter()
        .filter(|(_, valued)| !valued)
        .find_map(|(suffix, _)| name.strip_suffix(suffix))
        .map(|stem| stem.strip_suffix("_FLAGS").unwrap_or(stem).to_string())
}

fn file_pairs(rel: &str, source: &str) -> Pairs {
    let lists = flag_lists(source);
    let mut out = Pairs::new();
    for (name, flags) in &lists {
        let Some(stem) = standalone_stem(name) else { continue };
        let prefix = format!("{stem}_");
        let valued: BTreeSet<String> = lists
            .iter()
            .filter(|(other, _)| other.starts_with(&prefix) && standalone_stem(other).is_none())
            .flat_map(|(_, other_flags)| other_flags.iter().cloned())
            .collect();
        out.insert((rel.to_string(), stem), [Some(flags.clone()), (!valued.is_empty()).then_some(valued)]);
    }
    out
}

fn pairs() -> Pairs {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/handlers");
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    let mut out = Pairs::new();
    for file in &files {
        let rel = file.strip_prefix(&root).expect("under handlers").to_string_lossy().replace('\\', "/");
        out.extend(file_pairs(&rel, &std::fs::read_to_string(file).expect("read handler")));
    }
    out
}

#[test]
fn no_handler_flag_is_both_standalone_and_valued() {
    let both: Vec<_> = pairs().into_iter().filter_map(|(k, [s, v])| Some((k, s?, v?))).collect();
    assert!(both.len() >= 6, "only {} standalone/valued pairs found in src/handlers — the scan is broken", both.len());
    let mut found = BTreeSet::new();
    for ((file, stem), standalone, valued) in &both {
        for flag in standalone.intersection(valued) {
            found.insert((file.clone(), stem.clone(), flag.clone()));
        }
    }
    let allowed: BTreeSet<_> = OPTIONAL_VALUED.iter().map(|(f, s, x)| (f.to_string(), s.to_string(), x.to_string())).collect();
    let new: Vec<_> = found.difference(&allowed).map(|(f, s, x)| format!("  {f} {s}\t{x}")).collect();
    let stale: Vec<_> = allowed.difference(&found).map(|(f, s, x)| format!("  {f} {s}\t{x}")).collect();
    assert!(
        new.is_empty(),
        "these handler flags are listed both standalone and valued ({}). Read the tool's option table \
         and keep each in one list:\n{}",
        new.len(),
        new.join("\n"),
    );
    assert!(stale.is_empty(), "these OPTIONAL_VALUED entries are no longer in both lists; remove them:\n{}", stale.join("\n"));
}

#[test]
fn the_scan_reads_a_multi_line_list_and_pairs_it_by_stem() {
    let source = "static TOOL_STANDALONE: WordSet = WordSet::new(&[\n    \"-a\", \"-b\",\n]);\nstatic TOOL_VALUED: WordSet = WordSet::flags(&[\"-b\"]);\nconst OTHER: &[&str] = &[\"x\"];\n";
    let lists = flag_lists(source);
    let names: Vec<&str> = lists.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["TOOL_STANDALONE", "TOOL_VALUED", "OTHER"]);
    assert_eq!(lists[0].1, BTreeSet::from(["-a".to_string(), "-b".to_string()]));
    assert_eq!(lists[1].1.intersection(&lists[0].1).count(), 1);
}

#[test]
fn a_standalone_list_is_paired_with_every_other_list_its_handler_keeps_for_the_same_tool() {
    let source = "static TOOL_STANDALONE: WordSet = WordSet::new(&[\"-a\", \"-l\"]);\nstatic TOOL_VALUED: WordSet = WordSet::new(&[\"-P\"]);\nstatic TOOL_LOG_FILES: WordSet = WordSet::new(&[\"-l\"]);\nstatic OTHER_VALUED: WordSet = WordSet::new(&[\"-a\"]);\n";
    let paired = file_pairs("tool.rs", source);
    let [standalone, valued] = &paired[&("tool.rs".to_string(), "TOOL".to_string())];
    let both: Vec<&String> = standalone.as_ref().expect("standalone").intersection(valued.as_ref().expect("valued")).collect();
    assert_eq!(both, ["-l"]);
}
