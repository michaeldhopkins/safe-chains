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

/// A loop variable bound to a glob's matches is not a number, and splitting it in a printf argument
/// after a format with no numeric slot is harmless; reading it must not record it as a number.
#[test]
fn a_loop_over_matches_may_print_its_variable_unquoted() {
    let dir = folder(&["a.txt", "b.txt"]);
    for cmd in ["for f in *; do printf '%s\\n' $f; done", "for f in *.txt; do printf '%s\\n' $f; done"] {
        assert!(allowed_in(dir.path(), cmd), "`{cmd}` refused");
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

/// A line that turns on dot names gives `*` the dot names too: `rm *` in a folder holding `.envrc`
/// deletes it after `shopt -s dotglob` or a `GLOBIGNORE`, and not otherwise.
#[test]
fn a_line_that_turns_on_dot_names_matches_them() {
    let dir = folder(&["a.txt", ".envrc"]);
    assert!(allowed_in(dir.path(), "rm *"), "`rm *` refused without dot names");
    assert!(allowed_in(dir.path(), "shopt -s dotglob; rm a.txt"), "the setting alone was refused");
    for cmd in ["shopt -s dotglob; rm *", "GLOBIGNORE=x; rm *"] {
        assert!(!allowed_in(dir.path(), cmd), "`{cmd}` approved deleting .envrc");
    }
}

/// The shell removes quotes and escapes and expands braces before `shopt` sees its option, so no
/// spelling of a dot-names option turns it on unnoticed.
#[test]
fn a_dot_names_option_is_seen_however_it_is_spelt() {
    let dir = folder(&["a.txt", ".envrc"]);
    for setting in [
        "shopt -s dot''glob", "shopt -s dot\\glob", "shopt -s \"dot\"glob", "shopt -s {dot,x}glob", "shopt -s dot{glob,}",
        "shopt -s $'\\x64otglob'", "sh''opt -s dot\"\"glob", "builtin shopt -s dot''glob", "command shopt -s dot''glob",
        "setopt glob\\_dots", "unsetopt no''globdots", "export GLOB''IGNORE=x", "export GLOB{IGNORE,}=x", "{ shopt -s dot''glob; }",
        "time shopt -s {dot,x}glob", "time -p shopt -s {dot,x}glob", "{ time shopt -s {dot,x}glob; }", "{shopt,-s} {dot,x}glob",
        "{time,shopt} -s {dot,x}glob", "time setopt glob\\_dots", "exec shopt -s dot''glob", "noglob setopt glob''dots",
    ] {
        let cmd = format!("{setting}; rm *");
        assert!(!allowed_in(dir.path(), &cmd), "`{cmd}` approved deleting .envrc");
    }
    for cmd in ["shopt -s nullglob; rm *", "shopt -s extglob; rm *", "shopt -s globstar; rm *", "export FOO=bar; rm *"] {
        assert!(allowed_in(dir.path(), cmd), "`{cmd}` refused though it leaves dot names off");
    }
}

fn spellings_of(name: &'static str) -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(vec!["", "''", "\"\"", "\\"]), name.len()).prop_map(move |cuts| {
        let mut out = String::new();
        for (c, cut) in name.chars().zip(cuts) {
            if c.is_ascii_alphabetic() {
                out.push_str(cut);
            }
            out.push(c);
        }
        out
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    fn no_quoting_of_dotglob_hides_it(
        setting in prop_oneof![
            spellings_of("dotglob").prop_map(|o| format!("shopt -s {o}")),
            spellings_of("GLOB_DOTS").prop_map(|o| format!("setopt {o}")),
            spellings_of("GLOBIGNORE").prop_map(|o| format!("{o}=x")),
        ],
        prefix in prop::sample::select(vec!["", "time ", "time -p ", "builtin ", "command ", "{ time "]),
    ) {
        let dir = folder(&["a.txt", ".envrc"]);
        let close = if prefix.starts_with('{') { "; }" } else { "" };
        let cmd = format!("{prefix}{setting}{close}; rm *");
        prop_assert!(!allowed_in(dir.path(), &cmd), "`{}` approved deleting .envrc", cmd);
    }
}
