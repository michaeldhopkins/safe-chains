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

/// macOS `zless` and `zmore` start `$PAGER`, less or more only when it is unset, with the flags
/// they gathered. Set to another program, those flags are that program's: `PAGER=pax zless -r` has
/// pax extract the archive, and set to more, `-p` is a command more runs at startup (`-p'!sh'`).
/// So on a line that may set `PAGER`, the first argument must be `--` or a plain file name: one
/// that starts with `-`, or that a brace, a pattern or a substitution could still turn into a flag
/// (`{,-r}`, `[-]r`), is refused, and so is no argument at all.
pub(crate) fn hands_flags_to_another_pager(tokens: &[Token]) -> bool {
    crate::cst::pager_may_be_set() && !tokens.get(1).map(Token::as_str).is_some_and(|word| word == "--" || is_plain_file_name(word))
}

fn is_plain_file_name(word: &str) -> bool {
    !word.is_empty() && !word.starts_with('-') && !word.contains(['{', '}', '*', '?', '[', '$', '`']) && !word.contains("__SAFE_CHAINS_")
}

#[cfg(test)]
#[path = "shell_text_tests.rs"]
mod tests;
