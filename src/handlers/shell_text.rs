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

/// macOS `zless`, `zmore`, `zcmp`, `zdiff`, `bzcmp` and `bzdiff` are shell scripts that gather
/// the flags in front of the first file into one string and start the pager, cmp or diff with it
/// unquoted, so the shell splits it at spaces and expands any pattern in it.
/// `--prompt='x --lesskey-src=./k'` hands less a flag nobody gave it. A flag holding a space or a
/// pattern character is refused. bzdiff keeps reading flags after `--`, so `--` ends nothing here.
pub(crate) fn splits_a_flag(tokens: &[Token]) -> bool {
    const SPLIT: &[char] = &[' ', '\t', '\n', '*', '?', '['];
    tokens
        .iter()
        .skip(1)
        .map(Token::as_str)
        .take_while(|word| word.starts_with('-'))
        .any(|word| word.contains(SPLIT))
}

#[cfg(test)]
#[path = "shell_text_tests.rs"]
mod tests;
