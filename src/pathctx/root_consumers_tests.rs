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
    ("src/pathctx/place/physical.rs", 1, "a glob match is not judged together"),
    ("src/pathgate/folder_code.rs", 2, "the folder is foreign"),
    ("src/pathgate/handlers/jj.rs", 1, "the working directory counts as moved from the root"),
];

/// The files that read the context's raw `root` field instead of [`super::root`], with how many
/// times, and what they do when it holds the unknown folder. Only the path context's own module can
/// reach the ambient context, so only its files are searched.
const FIELD_READERS: &[(&str, usize, &str)] = &[
    ("src/pathctx.rs", 1, "a `cd` keeps the root as it was"),
    ("src/pathctx/home.rs", 1, "a `~/` path never lies inside the unknown folder, so it stays a home path"),
    (
        "src/pathctx/place.rs",
        2,
        "a `~/` path stays absolute; a relative one is placed by the folder level, or followed through a link in the reported folder",
    ),
    ("src/pathctx/root.rs", 2, "the unknown folder is `Root::Unknown`"),
];

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

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn code_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines().filter(|line| !line.trim_start().starts_with("//"))
}

/// Calls of the free function `root()`, however it is reached or wrapped (`(root())`, `!root()`),
/// and the function named without a call (`.then(pathctx::root)`). A method `x.root()` and another
/// function ending in `root` are not it.
fn reads_of_the_root(text: &str) -> usize {
    code_lines(text)
        .filter(|line| !line.contains("fn root("))
        .map(|line| {
            let calls = line
                .match_indices("root()")
                .filter(|(at, _)| line[..*at].chars().next_back().is_none_or(|c| !is_ident(c) && c != '.'))
                .count();
            let named = line
                .match_indices("::root")
                .filter(|(at, read)| line[at + read.len()..].chars().next().is_some_and(|c| matches!(c, ')' | ',' | ';')))
                .count();
            calls + named
        })
        .sum()
}

/// Reads of a context's `root` field: `ctx.root`, `c.borrow().root`, but not `..root` or `x.root_of`.
fn reads_of_the_field(text: &str) -> usize {
    code_lines(text)
        .map(|line| {
            line.match_indices(".root")
                .filter(|(at, read)| {
                    let before = line[..*at].chars().next_back();
                    let after = line[at + read.len()..].chars().next();
                    before.is_some_and(|c| is_ident(c) || c == ')') && after.is_none_or(|c| !is_ident(c) && c != '(')
                })
                .count()
        })
        .sum()
}

fn found(dir: &str, count: fn(&str) -> usize, must_match_on_root: bool) -> Vec<(String, usize)> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources(&base.join(dir), &mut files);
    if dir == "src/pathctx" {
        files.push(base.join("src/pathctx.rs"));
    }
    let mut found = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read source");
        let reads = count(&text);
        if reads > 0 {
            let name = file.strip_prefix(base).expect("under the crate").to_string_lossy().into_owned();
            if must_match_on_root {
                assert!(
                    text.contains("Root::Unknown") || text.contains("Root::Known"),
                    "{name} reads the project root without matching on it"
                );
            }
            found.push((name, reads));
        }
    }
    found.sort();
    found
}

#[test]
fn every_reader_of_the_project_root_is_listed_and_handles_an_unknown_one() {
    let listed: Vec<(String, usize)> = CONSUMERS.iter().map(|(name, reads, _)| ((*name).to_string(), *reads)).collect();
    assert_eq!(
        found("src", reads_of_the_root, true),
        listed,
        "a reader of the project root was added or removed: list it with what it does when the root is unknown"
    );
}

#[test]
fn every_reader_of_the_raw_root_field_is_listed() {
    let listed: Vec<(String, usize)> = FIELD_READERS.iter().map(|(name, reads, _)| ((*name).to_string(), *reads)).collect();
    assert_eq!(
        found("src/pathctx", reads_of_the_field, false),
        listed,
        "the raw root field is read somewhere new: read it through `root()`, or list it with what it does when it holds the unknown folder"
    );
}

#[test]
fn the_counters_see_every_spelling_of_a_read() {
    for read in [
        "let r = root();", "if (root()) {}", "match !root() {}", "x(crate::pathctx::root());", "let r = super::root();",
        ".then(crate::pathctx::root)", "f(pathctx::root, 1)",
    ] {
        assert_eq!(reads_of_the_root(read), 1, "{read}");
    }
    for other in ["path.root()", "physical::root(&r)", "fn root() -> Root {", "// root()", "is_root()", "pathctx::root_of(x)"] {
        assert_eq!(reads_of_the_root(other), 0, "{other}");
    }
    for read in ["ctx.root.as_deref()", "c.borrow().root.clone()", "(b.root)", "let r = b.root;"] {
        assert_eq!(reads_of_the_field(read), 1, "{read}");
    }
    for other in ["&root[..root.len() - 1]", "x.root_dir", "p.root()", "root: b"] {
        assert_eq!(reads_of_the_field(other), 0, "{other}");
    }
}

#[test]
fn nothing_renames_the_root_reader_out_of_sight() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources(&base.join("src"), &mut files);
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read source");
        for line in code_lines(&text) {
            assert!(!(line.contains("root as ") && line.trim_start().starts_with("use ")), "{}: {line}", file.display());
        }
    }
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
