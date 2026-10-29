//! Whether the line being judged may set `PAGER`, the program some wrappers hand their flags to.

use crate::cst::{Cmd, Script, SimpleCmd, Word, WordPart};

thread_local! {
    static PAGER_SET: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

const SETTERS: &[&str] =
    &["export", "declare", "typeset", "local", "readonly", "env", "read", "printf", "set", "mapfile", "readarray", "getopts", "eval"];

const PREFIXES: &[&str] = &["builtin", "command", "time", "-p", "--", "exec", "noglob", "nocorrect", "-"];

/// While held, the line counts as setting `PAGER`. Its text naming `PAGER` anywhere counts, and so
/// does a word that is not plain text given to a command that sets variables, since quotes and
/// escapes spell a name the text does not show (`declare -x P"AGER"=more`). Entering never clears
/// the mark an outer guard set.
pub(crate) struct PagerSetGuard(bool);

impl Drop for PagerSetGuard {
    fn drop(&mut self) {
        PAGER_SET.with(|cell| cell.set(self.0));
    }
}

pub(crate) fn enter_pager_set(script: &Script) -> PagerSetGuard {
    let on = script.to_string().contains("PAGER") || script_sets(script);
    PAGER_SET.with(|cell| PagerSetGuard(cell.replace(cell.get() || on)))
}

pub(crate) fn pager_may_be_set() -> bool {
    PAGER_SET.with(std::cell::Cell::get)
}

fn script_sets(script: &Script) -> bool {
    script.0.iter().any(|stmt| stmt.pipeline.commands.iter().any(cmd_sets))
}

fn cmd_sets(cmd: &Cmd) -> bool {
    match cmd {
        Cmd::Simple(s) => simple_sets(s),
        Cmd::Subshell { body, .. } | Cmd::BraceGroup { body, .. } | Cmd::FunctionDef { body, .. } | Cmd::For { body, .. } => {
            script_sets(body)
        }
        Cmd::While { cond, body, .. } | Cmd::Until { cond, body, .. } => script_sets(cond) || script_sets(body),
        Cmd::If { branches, else_body, .. } => {
            branches.iter().any(|b| script_sets(&b.cond) || script_sets(&b.body)) || else_body.as_ref().is_some_and(script_sets)
        }
        Cmd::Case { arms, .. } => arms.iter().any(|arm| script_sets(&arm.body)),
        Cmd::DoubleBracket { .. } => false,
    }
}

fn simple_sets(s: &SimpleCmd) -> bool {
    let expanded: Vec<String> = s.words.iter().flat_map(Word::expand).collect();
    let Some(at) = expanded.iter().position(|w| !PREFIXES.contains(&w.as_str())) else {
        return false;
    };
    SETTERS.contains(&expanded[at].as_str()) && s.words.iter().any(|w| !is_plain(w))
}

fn is_plain(word: &Word) -> bool {
    match word.0.as_slice() {
        [WordPart::Lit(text)] => text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+' | '=' | '.' | '/' | ':')),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn sets(line: &str) -> bool {
        let script = crate::cst::parse(line).expect("parses");
        let _guard = enter_pager_set(&script);
        pager_may_be_set()
    }

    #[test]
    fn a_line_that_never_names_the_pager_leaves_it() {
        for line in ["zless -p TODO a.gz", "export LESS=-R; zless a.gz", "declare -x X=1", "ls | head"] {
            assert!(!sets(line), "{line}");
        }
    }

    #[test]
    fn a_setter_in_either_half_of_a_loop_or_branch_marks_the_line() {
        let set = "declare -x P\"AGER\"=more";
        for form in [
            format!("while {set}; do :; done"),
            format!("while true; do {set}; done"),
            format!("until {set}; do :; done"),
            format!("until true; do {set}; done"),
            format!("if {set}; then :; fi"),
            format!("if true; then {set}; fi"),
            format!("if false; then :; elif {set}; then :; fi"),
            format!("if false; then :; elif true; then {set}; fi"),
            format!("if false; then :; else {set}; fi"),
        ] {
            assert!(sets(&form), "{form}");
        }
    }

    proptest! {
        /// Every way of putting `PAGER` into the line marks it, wherever it sits.
        #[test]
        fn naming_the_pager_marks_the_line(
            form in prop::sample::select(vec![
                "PAGER={v} zless a.gz", "export PAGER={v}; zless a.gz", "PAGER={v}; zless a.gz", "env PAGER={v} zless a.gz",
                "declare -x PAGER={v}", "read PAGER <<< {v}", "(export PAGER={v}); zless a.gz", "if true; then export PAGER={v}; fi",
                "declare -x P\"AGER\"={v}", "export P\\AGER={v}", "printf -v 'PAGER' {v}",
            ]),
            value in "[a-z/]{1,8}",
        ) {
            let line = form.replace("{v}", &value);
            prop_assert!(sets(&line), "{}", line);
        }
    }
}
