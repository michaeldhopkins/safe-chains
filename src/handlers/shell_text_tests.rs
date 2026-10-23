use proptest::prelude::*;

use super::{SHELL_TEXT, holds_shell_text};
use crate::pathctx::{PathCtx, enter};

const ROOT: &str = "/work/u/proj";
const PROGRAMS: &[&str] = &["man", "apropos", "whatis", "manpath"];

fn allowed(cmd: &str) -> bool {
    let _g = enter(PathCtx { cwd: Some(ROOT.into()), root: Some(ROOT.into()), session_id: None });
    crate::command_verdict(cmd).is_allowed()
}

fn quoted(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
}

fn arb_shell_word() -> impl Strategy<Value = String> {
    ("[a-z./]{0,4}", prop::sample::select(SHELL_TEXT), "[a-z ./]{0,4}").prop_map(|(before, c, after)| format!("{before}{c}{after}"))
}

fn spellings(program: &str, word: &str) -> Vec<String> {
    let q = quoted(word);
    let mut out = vec![format!("{program} {q}"), format!("{program} ls {q}")];
    if program == "man" {
        out.extend([
            format!("man -M {q} ls"),
            format!("man --manpath={q} ls"),
            format!("man -S {q} ls"),
            format!("man -m {q} ls"),
            format!("man -L {q} ls"),
        ]);
    }
    out
}

proptest! {
    /// Any word holding a character the man script's shell would act on refuses man and the
    /// programs that are the same script, wherever the word sits.
    #[test]
    fn a_word_the_man_script_would_evaluate_is_refused(program in prop::sample::select(PROGRAMS), word in arb_shell_word()) {
        for cmd in spellings(program, &word) {
            prop_assert!(!allowed(&cmd), "should be refused: {}", cmd);
        }
    }

    /// A variable the man script pastes into its command line refuses the same characters, as a
    /// prefix and as a lasting assignment.
    #[test]
    fn a_variable_the_man_script_would_evaluate_is_refused(name in prop::sample::select(&["LANG", "MANPATH", "MANSECT"][..]), word in arb_shell_word()) {
        let q = quoted(&word);
        for cmd in [format!("{name}={q} man ls"), format!("export {name}={q}; man ls"), format!("env {name}={q} man ls")] {
            prop_assert!(!allowed(&cmd), "should be refused: {}", cmd);
        }
    }

    #[test]
    fn a_plain_word_is_not_shell_text(word in "[A-Za-z0-9 ._/:@+,=-]{0,12}") {
        prop_assert!(!holds_shell_text(&word));
    }
}

#[test]
fn the_reviewed_man_spellings() {
    for cmd in [
        "MANPATH='./m`touch x`' man ls", "man './a`touch x`.1'", "man -M './m$(touch x)' ls", "LANG='x$(touch y)' man ls",
        "export LANG='en_US.UTF-8$(id)'; man ls",
    ] {
        assert!(!allowed(cmd), "should be refused: {cmd}");
    }
    for cmd in [
        "man ls", "man 3 printf", "man -k grep", "LANG=en_US.UTF-8 man ls", "MANPATH=/usr/share/man man ls", "man ./docs/a.1",
        "apropos grep",
    ] {
        assert!(allowed(cmd), "should be allowed: {cmd}");
    }
}
