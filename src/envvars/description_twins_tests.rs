//! Every environment variable a command's description names has a rule in `envvars.toml`, or is
//! owed in `tests/fixtures/description_env_vars_owed.txt`, which may only shrink.
//!
//! A description that says a flag reads a config or key file usually names the variable that does
//! the same (`--config FILE` or `FOO_CONFIG`). Gating the flag and leaving the variable unlisted
//! refuses one spelling and approves the other.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const OWED: &str = "tests/fixtures/description_env_vars_owed.txt";
const CREATE: &str = "CREATE_DESCRIPTION_ENV_VARS_OWED";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn toml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).expect("commands dir").flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            toml_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "toml") {
            out.push(path);
        }
    }
}

fn descriptions(value: &toml::Value, out: &mut Vec<String>) {
    match value {
        toml::Value::Table(table) => {
            for (key, v) in table {
                match v {
                    toml::Value::String(s) if key == "description" => out.push(s.clone()),
                    _ => descriptions(v, out),
                }
            }
        }
        toml::Value::Array(items) => items.iter().for_each(|v| descriptions(v, out)),
        _ => {}
    }
}

/// Names written as an environment variable: upper case, digits and underscores, holding an
/// underscore and not ending in one, or any upper-case name written after `$`.
pub(super) fn variable_names(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let word_start = i == 0 || !(chars[i - 1].is_ascii_alphanumeric() || chars[i - 1] == '_');
        if !(word_start && chars[i].is_ascii_uppercase()) {
            i += 1;
            continue;
        }
        let end = (i..chars.len())
            .find(|&j| !(chars[j].is_ascii_uppercase() || chars[j].is_ascii_digit() || chars[j] == '_'))
            .unwrap_or(chars.len());
        let word: String = chars[i..end].iter().collect();
        let after_word = chars.get(end).is_none_or(|c| !c.is_ascii_alphanumeric());
        let dollar = i > 0 && chars[i - 1] == '$';
        let shaped = word.contains('_') && !word.ends_with('_') && !word.starts_with('_');
        if after_word && word.len() > 2 && (shaped || dollar) {
            names.insert(word);
        }
        i = end.max(i + 1);
    }
    names
}

fn unlisted() -> (BTreeSet<String>, usize) {
    let mut files = Vec::new();
    toml_files(&root().join("commands"), &mut files);
    assert!(files.len() > 100, "found only {} command files", files.len());
    let mut out = BTreeSet::new();
    let mut listed = 0;
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read toml");
        let value: toml::Value = toml::from_str(&text).expect("parse toml");
        let mut found = Vec::new();
        descriptions(&value, &mut found);
        let command = file.strip_prefix(root().join("commands")).expect("under commands").display().to_string();
        for name in found.iter().flat_map(|d| variable_names(d)) {
            if super::rule_of(&name).is_none() {
                out.insert(format!("{command}\t{name}"));
            } else {
                listed += 1;
            }
        }
    }
    (out, listed)
}

#[test]
fn every_variable_a_description_names_is_listed_or_owed() {
    let (found, listed) = unlisted();
    assert!(listed > 15, "only {listed} named variables have a rule; the scan has gone blind");
    let path = root().join(OWED);
    if std::env::var_os(CREATE).is_some() && !path.exists() {
        std::fs::write(&path, found.iter().cloned().collect::<Vec<_>>().join("\n") + "\n").expect("write owed");
    }
    let owed: BTreeSet<String> = std::fs::read_to_string(&path)
        .expect("owed list")
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    let new: Vec<&String> = found.difference(&owed).collect();
    let done: Vec<&String> = owed.difference(&found).collect();
    assert!(
        new.is_empty(),
        "these descriptions name a variable envvars.toml has no rule for; research it and add a rule \
         (or say in the description why it changes nothing):\n{}",
        new.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
    );
    assert!(
        done.is_empty(),
        "these are listed now; remove them from {OWED}:\n{}",
        done.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn variable_names_reads_the_shapes_a_description_uses() {
    let names = variable_names("reads $HOME and GIT_CONFIG_GLOBAL, not O_RDONLY_ or JSON or _X; see `PIP_CONFIG_FILE`.");
    let got: Vec<&str> = names.iter().map(String::as_str).collect();
    assert_eq!(got, ["GIT_CONFIG_GLOBAL", "HOME", "PIP_CONFIG_FILE"]);
    assert!(variable_names("lowercase_name and Mixed_Case").is_empty());
}
