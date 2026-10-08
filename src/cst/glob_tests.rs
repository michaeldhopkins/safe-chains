//! A glob is read against the working directory when it is known: its matches are classified as
//! the words the command receives, unless one could be taken for a flag.

use crate::pathctx::PathCtx;
use proptest::prelude::*;

fn folder(names: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for name in names {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(&path, "").expect("write");
    }
    dir
}

fn allowed_in(dir: &std::path::Path, cmd: &str) -> bool {
    let at = dir.canonicalize().expect("canonical").to_string_lossy().into_owned();
    crate::command_verdict_in(cmd, PathCtx { cwd: Some(at.clone()), root: Some(at), session_id: None }).is_allowed()
}

#[test]
fn matches_that_cannot_be_flags_are_classified_as_paths() {
    let dir = folder(&["a.txt", "b.txt", "src/x.rs", "sub/c.txt", "sub/-rf"]);
    for cmd in
        ["ls *", "cat *.txt", "wc -l */*.rs", "find . *", "for f in *.txt; do cat \"$f\"; done", "cat *.nothing", "cd sub && cat *.txt"]
    {
        assert!(allowed_in(dir.path(), cmd), "`{cmd}` refused in a folder with no flag-like name");
    }
}

#[test]
fn a_match_that_could_be_a_flag_keeps_the_glob_unknown() {
    let cases: &[(&str, &[&str])] = &[
        ("-delete", &["find / *", "ls *", "ls ?delete", "find / [-]delete"]),
        ("--files0-from=x.txt", &["ls *", "cat *.txt"]),
        ("a\nb.txt", &["ls *", "cat *.txt"]),
        ("-rf/x", &["wc -l */x", "ls */*"]),
    ];
    for (hostile, cmds) in cases {
        let dir = folder(&["a.txt", hostile]);
        for cmd in *cmds {
            assert!(!allowed_in(dir.path(), cmd), "`{cmd}` approved beside {hostile:?}");
        }
    }
}

#[test]
fn an_unknown_folder_keeps_the_glob_unknown() {
    let dir = folder(&["a.txt"]);
    assert!(!allowed_in(dir.path(), "cd \"$X\" && ls *"));
    assert!(!allowed_in(dir.path(), "cd missing; find / *"), "a failed cd leaves the shell where it was");
    assert!(!crate::command_verdict_in("ls *", PathCtx::default()).is_allowed());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// `ls *` is approved exactly when no name in the folder could be taken for a flag.
    #[test]
    fn a_glob_is_approved_only_when_no_match_can_lead_with_a_dash(names in prop::collection::btree_set("-?[a-z]{1,6}", 1..6)) {
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let dir = folder(&names);
        let flag_like = names.iter().any(|n| n.starts_with('-'));
        prop_assert_eq!(allowed_in(dir.path(), "ls *"), !flag_like, "{:?}", names);
        prop_assert_eq!(allowed_in(dir.path(), "find / *"), !flag_like, "{:?}", names);
    }
}
