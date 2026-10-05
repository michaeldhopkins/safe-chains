//! The instruction-file budget.
//!
//! Every `AGENTS.md` and `CLAUDE.md` is injected whole into every agent session, so a project's
//! instruction file stays under 24 KB, and the detail lives in `docs/agents/<topic>.md` files under
//! 8 KB that are read when their moment comes. AGENTS.md names every one of them: a topic file
//! nobody names is never read, and a name with no file behind it fails silently. safe-chains'
//! AGENTS.md had reached 27 KB when this went in (2026-10-04).

use std::path::{Path, PathBuf};

const INSTRUCTIONS_LIMIT: usize = 24 * 1024;
const TOPIC_LIMIT: usize = 8 * 1024;
const TOPICS: &str = "docs/agents";

/// What is wrong with the instruction files, given each one's path and size, the topic files'
/// paths and sizes, and the text of the root AGENTS.md.
fn problems(instructions: &[(String, usize)], topics: &[(String, usize)], index: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (path, size) in instructions {
        if *size > INSTRUCTIONS_LIMIT {
            found.push(format!("{path} is {size} bytes, over {INSTRUCTIONS_LIMIT}: move detail into {TOPICS}/"));
        }
    }
    for (path, size) in topics {
        if *size > TOPIC_LIMIT {
            found.push(format!("{path} is {size} bytes, over {TOPIC_LIMIT}: split it"));
        }
        if !index.contains(&format!("`{path}`")) {
            found.push(format!("{path} is not named in AGENTS.md, so nothing will read it"));
        }
    }
    let prefix = format!("`{TOPICS}/");
    for named in index.split(&prefix).skip(1) {
        let Some(end) = named.find('`') else { continue };
        if !named[..end].ends_with(".md") {
            continue;
        }
        let path = format!("{TOPICS}/{}", &named[..end]);
        if !topics.iter().any(|(p, _)| *p == path) {
            found.push(format!("AGENTS.md names {path}, which does not exist"));
        }
    }
    found
}

fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, usize)>, keep: &dyn Fn(&Path) -> bool) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !matches!(name.as_ref(), "target" | ".jj" | ".git" | "corpus" | "artifacts" | "book") {
                walk(&path, root, out, keep);
            }
        } else if keep(&path) {
            let size = std::fs::metadata(&path).map(|m| m.len() as usize).unwrap_or(0);
            let relative = path.strip_prefix(root).unwrap_or(&path);
            out.push((relative.to_string_lossy().into_owned(), size));
        }
    }
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn instruction_files_stay_within_budget_and_every_topic_is_named() {
    let root = root();
    let mut instructions = Vec::new();
    walk(&root, &root, &mut instructions, &|p| matches!(p.file_name().and_then(|n| n.to_str()), Some("AGENTS.md" | "CLAUDE.md")));
    let mut topics = Vec::new();
    walk(&root.join(TOPICS), &root, &mut topics, &|p| p.extension().is_some_and(|e| e == "md"));
    assert!(instructions.iter().any(|(p, _)| p == "AGENTS.md"), "found no root AGENTS.md; the walk is looking in the wrong place");
    assert!(!topics.is_empty(), "found no {TOPICS} files");
    let index = std::fs::read_to_string(root.join("AGENTS.md")).expect("read AGENTS.md");
    let found = problems(&instructions, &topics, &index);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn an_instruction_file_over_the_limit_is_refused() {
    let found = problems(&[("AGENTS.md".into(), INSTRUCTIONS_LIMIT + 1)], &[], "");
    assert_eq!(found.len(), 1);
    assert!(found[0].contains("over 24576"));
    assert!(problems(&[("AGENTS.md".into(), INSTRUCTIONS_LIMIT)], &[], "").is_empty());
}

#[test]
fn a_topic_over_the_limit_or_unnamed_is_refused() {
    let path = "docs/agents/fuzzing.md".to_string();
    let named = "see `docs/agents/fuzzing.md`";
    assert!(problems(&[], &[(path.clone(), TOPIC_LIMIT)], named).is_empty());
    let big = problems(&[], &[(path.clone(), TOPIC_LIMIT + 1)], named);
    assert!(big.iter().any(|p| p.contains("split it")), "{big:?}");
    let unnamed = problems(&[], &[(path, 10)], "fuzzing.md is mentioned bare");
    assert!(unnamed.iter().any(|p| p.contains("not named")), "{unnamed:?}");
}

#[test]
fn a_name_with_no_file_behind_it_is_refused() {
    let found = problems(&[], &[], "read `docs/agents/gone.md` first");
    assert_eq!(found, vec!["AGENTS.md names docs/agents/gone.md, which does not exist"]);
}
