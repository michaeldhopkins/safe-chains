use proptest::prelude::*;

use crate::pathctx::{PathCtx, enter};

const ROOT: &str = "/work/u/proj";
const KEPT: &[&str] = &["'$PATH'", "\\$PATH", "$'$PATH'", "'${PATH}'", "\"\\$PATH\"", "$'\\x24PATH'", "'$'PATH"];
const STANDARD: &[&str] = &["/usr/bin", "/bin", "$PATH", "${PATH}", "\"$PATH\""];

fn allowed(cmd: &str) -> bool {
    let _g = enter(PathCtx { cwd: Some(ROOT.into()), root: Some(ROOT.into()), session_id: None });
    crate::command_verdict(cmd).is_allowed()
}

fn spellings(value: &str) -> Vec<String> {
    vec![
        format!("PATH={value} ls"),
        format!("PATH+=:{value} ls"),
        format!("env PATH={value} ls"),
        format!("env -i PATH={value} ls"),
        format!("export PATH={value}; ls"),
        format!("declare -x PATH={value}; ls"),
        format!("typeset PATH={value}; ls"),
        format!("PATH={value}; ls"),
        format!("path+={value}; ls"),
    ]
}

proptest! {
    /// A search path whose `$PATH` the shell keeps as text names a folder below the working
    /// directory, so it is refused wherever it sits among standard folders and however the
    /// assignment is spelled; the same path with every `$PATH` expanded keeps `ls` approved.
    #[test]
    fn a_dollar_the_shell_keeps_in_a_search_path_is_refused(
        before in prop::collection::vec(prop::sample::select(STANDARD), 0..3),
        kept in prop::sample::select(KEPT),
        after in prop::collection::vec(prop::sample::select(STANDARD), 0..3),
    ) {
        let mut elements = before.clone();
        elements.push(kept);
        elements.extend(after.iter().copied());
        for cmd in spellings(&elements.join(":")) {
            prop_assert!(!allowed(&cmd), "should be refused: {}", cmd);
        }
        let expanded: Vec<&str> = before.iter().chain(&after).copied().chain(["$PATH"]).collect();
        let plain = format!("PATH={} ls", expanded.join(":"));
        prop_assert!(allowed(&plain), "should be allowed: {}", plain);
    }
}

#[test]
fn a_kept_dollar_outside_a_search_path_is_left_to_the_other_rules() {
    for cmd in ["echo '$PATH'", "grep -n '$PATH' ./notes.md", "FOO='$PATH' ls", "export FOO='$PATH'"] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
}

#[test]
fn a_kept_dollar_given_to_a_setter_by_name_is_refused() {
    for cmd in ["tmux setenv PATH '$PATH'", "export PATH='$PATH'", "export 'PATH=$PATH'"] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
}
