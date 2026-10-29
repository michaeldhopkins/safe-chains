//! Whether `*` matches names that start with a dot, for the line being judged.

use crate::cst::{Cmd, Script, SimpleCmd, Word, WordPart};

thread_local! {
    static DOT_NAMES: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

const OPTIONS: [&str; 3] = ["dotglob", "globdots", "globignore"];

const SETTERS: &[&str] = &["shopt", "setopt", "unsetopt", "set", "export", "declare", "typeset", "local", "readonly"];

/// While held, `*` also matches names that start with a dot. A line that turns on bash's `dotglob`,
/// sets `GLOBIGNORE` (which turns `dotglob` on too) or turns on zsh's `GLOB_DOTS` anywhere is judged
/// so throughout: the words are matched without reading the shell, so `shopt -s dotglob; rm -rf *`
/// reaches `.git`. Words are read after quote removal, and a word given to a command that sets
/// options or variables counts as turning it on unless it is plain text, since braces and `$'…'`
/// escapes spell an option the text does not show. Entering never turns it off for a line an outer
/// guard already marked.
pub(crate) struct DotNamesGuard(bool);

impl Drop for DotNamesGuard {
    fn drop(&mut self) {
        DOT_NAMES.with(|cell| cell.set(self.0));
    }
}

pub(crate) fn enter_dot_names(script: &Script) -> DotNamesGuard {
    let on = names_an_option(&script.to_string()) || script_turns_on(script);
    DOT_NAMES.with(|cell| DotNamesGuard(cell.replace(cell.get() || on)))
}

pub(crate) fn dot_names_matched() -> bool {
    DOT_NAMES.with(std::cell::Cell::get)
}

fn names_an_option(text: &str) -> bool {
    let folded: String = text.chars().filter(|c| *c != '_').flat_map(char::to_lowercase).collect();
    OPTIONS.iter().any(|option| folded.contains(option))
}

fn script_turns_on(script: &Script) -> bool {
    script.0.iter().any(|stmt| stmt.pipeline.commands.iter().any(cmd_turns_on))
}

fn cmd_turns_on(cmd: &Cmd) -> bool {
    match cmd {
        Cmd::Simple(s) => simple_turns_on(s),
        Cmd::Subshell { body, .. } | Cmd::BraceGroup { body, .. } | Cmd::FunctionDef { body, .. } => script_turns_on(body),
        Cmd::For { body, .. } => script_turns_on(body),
        Cmd::While { cond, body, .. } | Cmd::Until { cond, body, .. } => script_turns_on(cond) || script_turns_on(body),
        Cmd::If { branches, else_body, .. } => {
            branches.iter().any(|b| script_turns_on(&b.cond) || script_turns_on(&b.body)) || else_body.as_ref().is_some_and(script_turns_on)
        }
        Cmd::Case { arms, .. } => arms.iter().any(|arm| script_turns_on(&arm.body)),
        Cmd::DoubleBracket { .. } => false,
    }
}

fn simple_turns_on(s: &SimpleCmd) -> bool {
    if s.env.iter().any(|(name, _)| names_an_option(name)) || s.words.iter().any(|w| names_an_option(&w.eval())) {
        return true;
    }
    let mut words = s.words.iter().skip_while(|w| matches!(w.eval().as_str(), "builtin" | "command"));
    let Some(name) = words.next() else {
        return false;
    };
    SETTERS.contains(&name.eval().as_str()) && words.any(|w| !is_plain(w))
}

fn is_plain(word: &Word) -> bool {
    match word.0.as_slice() {
        [WordPart::Lit(text)] => text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+' | '=' | '.' | '/' | ':')),
        _ => false,
    }
}
