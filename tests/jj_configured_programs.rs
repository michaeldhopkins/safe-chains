//! jj forms that run a program named in configuration are never approved.
//!
//! An editor, a diff editor, a merge tool or a fix tool is whatever `ui.editor`,
//! `ui.diff-editor`, `merge-tools.*` or `fix.tools` says, so approving the form that opens one
//! approves an arbitrary program. `jj describe` with no message, `jj split` with no filesets and
//! `jj resolve` without `--list` all did.
//!
//! The rows pin both answers for each such sub. The enumeration walks every sub in
//! `commands/vcs/jj.toml` and fails on one this file has not classified, so a sub added later is
//! looked at here before it ships.

use std::collections::BTreeSet;

const JJ_TOML: &str = include_str!("../commands/vcs/jj.toml");

const LAUNCHES: &[&str] = &[
    "jj describe", "jj describe -r @-", "jj describe -rm", "jj describe -r m", "jj describe -- -m", "jj describe --editor -m x",
    "jj --no-pager describe", "jj commit", "jj commit -- -m", "jj commit src/main.rs", "jj commit -i -m x", "jj split", "jj split -m x",
    "jj split src/main.rs", "jj split -r @- src/main.rs", "jj split -i -m x src/main.rs", "jj split -m x -r src/main.rs", "jj squash",
    "jj squash --into @-", "jj squash -r @", "jj squash -i -m x", "jj squash -k", "jj resolve", "jj resolve src/main.rs",
    "jj resolve -r @-", "jj resolve --tool meld", "jj fix", "jj fix -s @", "jj unsquash", "jj diffedit", "jj diffedit -r @",
];

const QUIET: &[&str] = &[
    "jj describe -m x", "jj describe -mx", "jj describe --message x", "jj describe --message=x", "jj describe -r @- -m x",
    "jj describe --stdin", "jj commit -m x", "jj commit --message=x src/main.rs", "jj split -m x src/main.rs",
    "jj split -m x -- src/main.rs", "jj split -r @- -m x src/main.rs", "jj squash -m x", "jj squash -u",
    "jj squash --use-destination-message --into @-", "jj squash -k -m x", "jj resolve --list", "jj resolve -l", "jj resolve -l -r @-",
    "jj new", "jj new -m x", "jj abandon", "jj absorb", "jj rebase -d main", "jj restore src/main.rs",
];

const NEVER_OPENS_A_PROGRAM: &[&str] = &[
    "abandon", "absorb", "backout", "bookmark", "bookmark create", "bookmark delete", "bookmark forget", "bookmark list", "bookmark move",
    "bookmark rename", "bookmark set", "bookmark track", "bookmark untrack", "cat", "config", "config get", "config list", "config path",
    "diff", "duplicate", "edit", "file", "file list", "file show", "git", "git fetch", "git import", "git init", "git remote",
    "git remote list", "help", "log", "new", "op", "op log", "parallelize", "rebase", "restore", "root", "show", "simplify-parents", "st",
    "status", "tag", "tag list", "undo", "version", "workspace", "workspace add", "workspace forget", "workspace list", "workspace rename",
    "workspace root", "workspace update-stale",
];

const OPENS_ONE_UNLESS_GATED: &[&str] = &["commit", "describe", "resolve", "split", "squash"];

fn subs(table: &toml::Value, prefix: &str, out: &mut BTreeSet<String>, candidates: &mut BTreeSet<String>) {
    let Some(list) = table.get("sub").and_then(toml::Value::as_array) else { return };
    for sub in list {
        let name = sub.get("name").and_then(toml::Value::as_str).expect("every sub has a name");
        let full = if prefix.is_empty() { name.to_string() } else { format!("{prefix} {name}") };
        if sub.get("candidate").and_then(toml::Value::as_bool) == Some(true) {
            candidates.insert(full);
            continue;
        }
        subs(sub, &full, out, candidates);
        out.insert(full);
    }
}

fn jj_subs() -> (BTreeSet<String>, BTreeSet<String>) {
    let doc: toml::Value = toml::from_str(JJ_TOML).expect("jj.toml parses");
    let command = doc
        .get("command")
        .and_then(toml::Value::as_array)
        .and_then(|c| c.iter().find(|c| c.get("name").and_then(toml::Value::as_str) == Some("jj")))
        .expect("jj.toml defines jj");
    let (mut out, mut candidates) = (BTreeSet::new(), BTreeSet::new());
    subs(command, "", &mut out, &mut candidates);
    (out, candidates)
}

#[test]
fn no_jj_form_that_opens_a_configured_program_is_approved() {
    let wrongly_allowed: Vec<&str> = LAUNCHES.iter().copied().filter(|c| safe_chains::is_safe_command(c)).collect();
    assert!(
        wrongly_allowed.is_empty(),
        "these open an editor, merge tool or fix tool and were approved:\n  {}",
        wrongly_allowed.join("\n  ")
    );
}

#[test]
fn the_message_and_list_forms_stay_approved() {
    let wrongly_denied: Vec<&str> = QUIET.iter().copied().filter(|c| !safe_chains::is_safe_command(c)).collect();
    assert!(wrongly_denied.is_empty(), "these open nothing and were refused:\n  {}", wrongly_denied.join("\n  "));
}

#[test]
fn every_jj_sub_is_classified_for_programs_it_opens() {
    let known: BTreeSet<String> = NEVER_OPENS_A_PROGRAM.iter().chain(OPENS_ONE_UNLESS_GATED).map(|s| (*s).to_string()).collect();
    let (present, candidates) = jj_subs();
    assert!(present.len() >= 40, "found only {} jj subs; the walk is broken", present.len());
    for candidate in &candidates {
        let bare = format!("jj {candidate}");
        assert!(!safe_chains::is_safe_command(&bare), "{bare} is a candidate and must not be approved");
    }
    let unclassified: Vec<&String> = present.difference(&known).collect();
    assert!(
        unclassified.is_empty(),
        "classify these jj subs here: does any allowed form open ui.editor, a diff editor, a merge tool or fix.tools?\n  {unclassified:?}"
    );
    let stale: Vec<&String> = known.difference(&present).collect();
    assert!(stale.is_empty(), "classified but no longer in jj.toml: {stale:?}");
}

#[test]
fn every_gated_sub_is_refused_bare_and_has_a_row_each_way() {
    for sub in OPENS_ONE_UNLESS_GATED {
        let bare = format!("jj {sub}");
        assert!(!safe_chains::is_safe_command(&bare), "{bare} opens a configured program");
        assert!(LAUNCHES.contains(&bare.as_str()), "{bare} needs a LAUNCHES row");
        assert!(QUIET.iter().any(|q| q.starts_with(&format!("{bare} "))), "{sub} needs a QUIET row showing the form that is approved");
    }
}

const PROGRAM_FLAGS: &[&str] = &["--config", "--config-file", "--config-toml", "--edit", "--editor", "--interactive", "--tool", "-i"];

const NAMED_SO_BUT_OPENS_NOTHING: &[&str] = &["jj new: --edit"];

fn listed_flags(table: &toml::Value, path: &str, out: &mut Vec<String>) {
    for key in ["standalone", "valued", "bare_flags", "require_any"] {
        for flag in table.get(key).and_then(toml::Value::as_array).into_iter().flatten().filter_map(toml::Value::as_str) {
            out.push(format!("{path}: {flag}"));
        }
    }
    for sub in table.get("sub").and_then(toml::Value::as_array).into_iter().flatten() {
        let name = sub.get("name").and_then(toml::Value::as_str).unwrap_or("?");
        listed_flags(sub, &format!("{path} {name}"), out);
    }
}

#[test]
fn no_jj_sub_lists_a_flag_that_opens_a_configured_program() {
    let doc: toml::Value = toml::from_str(JJ_TOML).expect("jj.toml parses");
    let mut all = Vec::new();
    for command in doc.get("command").and_then(toml::Value::as_array).into_iter().flatten() {
        listed_flags(command, "jj", &mut all);
    }
    assert!(all.len() > 200, "found only {} listed flags; the walk is broken", all.len());
    let opening: Vec<&String> = all
        .iter()
        .filter(|f| !NAMED_SO_BUT_OPENS_NOTHING.contains(&f.as_str()))
        .filter(|f| PROGRAM_FLAGS.iter().any(|p| f.ends_with(&format!(": {p}")) || f.contains(&format!(": {p}="))))
        .collect();
    assert!(opening.is_empty(), "these flags run an editor, a diff or merge tool, or load config naming one:\n  {opening:?}");
}
