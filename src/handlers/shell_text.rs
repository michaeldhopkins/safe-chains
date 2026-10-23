//! Words a program pastes into a shell command line it evaluates.
//!
//! macOS `/usr/bin/man` is a shell script: it evaluates `"$cattool \"$manpage\" | $pipeline"`, so a
//! page path, a `-M` folder or a `MANPATH` element holding `$(…)`, a backtick or a double quote runs
//! a command, and it evaluates the output of `locale`, which prints `LANG` as given. Such a word is
//! refused wherever it sits on man's line, and the variables carry the `shell-text` shape.

use crate::parse::Token;

pub(crate) const SHELL_TEXT: &[char] = &['`', '$', '"', '\\', ';', '|', '&', '(', ')', '<', '>', '\n', '\r'];

pub(crate) fn holds_shell_text(word: &str) -> bool {
    word.contains(SHELL_TEXT)
}

pub(crate) fn man_evaluates_a_word(tokens: &[Token]) -> bool {
    tokens.iter().skip(1).any(|token| holds_shell_text(token.as_str()))
}

#[cfg(test)]
#[path = "shell_text_tests.rs"]
mod tests;
