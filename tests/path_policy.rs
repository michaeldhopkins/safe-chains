//! The path policy, enforced from the corpus rather than from memory.
//!
//! `tests/fixtures/path_policy_corpus.tsv` names the guard in this file, and until now that guard
//! did not exist — the corpus was a document, so the surface it describes could drift away from it
//! silently, which is the exact failure it was written to prevent.
//!
//! Driven through the real binary, not `is_safe_command`, because the policy depends on the cwd
//! and the workspace root: a sibling read, a `..` that has somewhere to resolve to, and the
//! scratch directory are all questions about where the agent is standing, and an in-process call
//! answers them from the developer's own cwd.

use std::process::Command;

const CORPUS: &str = include_str!("fixtures/path_policy_corpus.tsv");

struct Row<'a> {
    want_allow: bool,
    category: &'a str,
    command: &'a str,
}

fn rows() -> Vec<Row<'static>> {
    CORPUS
        .lines()
        .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .map(|line| {
            let mut f = line.splitn(3, '\t');
            let (want, category, command) = (
                f.next().expect("verdict column"),
                f.next().expect("category column"),
                f.next().expect("command column"),
            );
            assert!(
                matches!(want, "allow" | "deny"),
                "corpus verdict must be allow/deny, got {want:?} in: {line}"
            );
            Row { want_allow: want == "allow", category, command }
        })
        .collect()
}

/// The workspace the corpus is written against. A real directory (this checkout) so that the
/// sibling and parent cases have something to resolve against.
fn workspace() -> String {
    env!("CARGO_MANIFEST_DIR").to_string()
}

fn allows(command: &str) -> bool {
    let w = workspace();
    Command::new(env!("CARGO_BIN_EXE_safe-chains"))
        .args(["--cwd", &w, "--root", &w, command])
        .output()
        .expect("run safe-chains")
        .status
        .success()
}

#[test]
fn the_path_policy_corpus_holds() {
    let rows = rows();
    assert!(rows.len() >= 70, "corpus shrank to {} rows — cases were deleted, not fixed", rows.len());

    let mut wrong = Vec::new();
    for row in &rows {
        let got = allows(row.command);
        if got != row.want_allow {
            let (want, got) = if row.want_allow { ("allow", "deny") } else { ("deny", "allow") };
            wrong.push(format!("  [{}] want {want}, got {got}: {}", row.category, row.command));
        }
    }
    assert!(
        wrong.is_empty(),
        "the path policy no longer matches its corpus ({} of {} rows):\n{}\n\n\
         If the change was intended, edit the corpus in the same commit and say why there — a \
         verdict that moves without the corpus moving is the drift this guard exists to catch.",
        wrong.len(),
        rows.len(),
        wrong.join("\n")
    );
}

/// Each FACE pins both answers, so neither a refuse-everything nor an allow-everything
/// classifier can pass.
///
/// Not per-category: this corpus is deliberately organised so the category names the verdict
/// class (`read-home` is the allows, `write-home` the denies), and demanding a mix inside each
/// would be demanding a different corpus. The property that actually guards it is per-face —
/// reads must show both answers somewhere, and so must writes. Reads are the direction that just
/// moved, and "everything reads now" is precisely the failure this has to be able to see.
#[test]
fn each_face_of_the_corpus_pins_both_answers() {
    let rows = rows();
    for (face, prefix) in [("read", "read-"), ("write", "write-")] {
        let mine: Vec<bool> =
            rows.iter().filter(|r| r.category.starts_with(prefix)).map(|r| r.want_allow).collect();
        assert!(mine.len() >= 10, "{face}: only {} rows — too thin to discriminate", mine.len());
        assert!(mine.iter().any(|a| *a), "{face}: corpus expects no allows, so refusing everything would pass");
        assert!(mine.iter().any(|a| !*a), "{face}: corpus expects no denies, so allowing everything would pass");
    }
}
