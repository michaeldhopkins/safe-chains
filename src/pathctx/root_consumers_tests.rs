//! The project root is a [`Root`](super::Root) so that an unknown one cannot be read as "no
//! boundary". The compiler makes each consumer match on it; this test makes adding one a decision
//! someone has to write down, because a new reader of the root is a new place that must fail closed.

use std::path::{Path, PathBuf};

/// Every non-test source file that reads the root, with how many times, and what it does when the
/// root is unknown.
const CONSUMERS: &[(&str, usize, &str)] = &[
    (
        "src/engine/resolve/frozen_walk.rs",
        6,
        "a glob or unplaced operand is cut short; links are not trusted; nothing is placed in the project",
    ),
    ("src/engine/resolve/regions.rs", 1, "not an adjacent peer, so no grant"),
    ("src/envvars/config_dir.rs", 1, "no extra spelling of the project's parent"),
    ("src/explain_fix/gate.rs", 1, "a link met in the folder counts as leading out"),
    ("src/handlers/ruby/gemfile.rs", 1, "a bundle command is refused"),
    ("src/handlers/vcs/git.rs", 1, "a directory counts as outside the project unless granted for writing"),
    ("src/pathctx.rs", 1, "nothing lies in the root"),
    ("src/pathctx/folder.rs", 3, "no workspace, so folder code is refused"),
    ("src/pathctx/physical.rs", 1, "a glob match is not judged together"),
    ("src/pathgate/folder_code.rs", 2, "the folder is foreign"),
    ("src/pathgate/handlers/jj.rs", 1, "the working directory counts as moved from the root"),
];

const READS: &[&str] = &["pathctx::root()", "super::root()", "pathctx::root)", " root()"];

fn sources(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|e| e == "rs")
            && !path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with("tests.rs") || n == "harness.rs")
        {
            found.push(path);
        }
    }
}

fn reads_of_the_root(text: &str) -> usize {
    let production = text;
    production
        .lines()
        .filter(|line| !line.trim_start().starts_with("//") && !line.contains("fn root("))
        .map(|line| READS.iter().map(|read| line.matches(read).count()).sum::<usize>())
        .sum()
}

#[test]
fn every_reader_of_the_project_root_is_listed_and_handles_an_unknown_one() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources(&base.join("src"), &mut files);
    let mut found: Vec<(String, usize)> = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read source");
        let reads = reads_of_the_root(&text);
        if reads > 0 {
            let name = file.strip_prefix(base).expect("under the crate").to_string_lossy().into_owned();
            assert!(text.contains("Root::Unknown") || text.contains("Root::Known"), "{name} reads the project root without matching on it");
            found.push((name, reads));
        }
    }
    found.sort();
    let listed: Vec<(String, usize)> = CONSUMERS.iter().map(|(name, reads, _)| ((*name).to_string(), *reads)).collect();
    assert_eq!(found, listed, "a reader of the project root was added or removed: list it with what it does when the root is unknown");
}

#[test]
fn nothing_reads_an_unknown_root_as_an_absent_one() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources(&base.join("src"), &mut files);
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read source");
        assert!(!text.contains("root_is_unknown"), "{}", file.display());
    }
}
