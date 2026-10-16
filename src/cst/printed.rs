//! What `echo` and `printf` hand a downstream `xargs` or `while read`.
//!
//! Both print text written on the command line, so the item a consumer receives is known exactly
//! when it is one plain word, and that word is what the consumer is classified with. A stand-in
//! would not do: the item can land where the consumer reads a program (`xargs awk`, `xargs jq -n`,
//! `xargs sed`), and a placeholder there is a harmless word while the real item is the code.
//!
//! Anything else is not known exactly: several words become one line or several items depending
//! on xargs' flags, quotes and backslashes are rewritten by xargs' own parsing, `printf`'s `%`
//! directives and escapes rewrite the text, and `echo -e` interprets escapes. Those worst-case to
//! the unknown item, as output nobody can name.

use super::check::UNKNOWN_ITEM;

/// The item `echo` or `printf` (named by `name`) writes for `args`.
pub(super) fn printed_item(name: &str, args: &[&str]) -> String {
    let words = match name {
        "echo" => &args[args.iter().take_while(|a| is_echo_switch(a)).count()..],
        _ => args,
    };
    match words {
        [] => "sc_item".to_string(),
        [word] if is_plain(word, name == "printf") => (*word).to_string(),
        _ => UNKNOWN_ITEM.to_string(),
    }
}

/// `-n`, `-e`, `-E` and clusters of them, which bash's `echo` reads as switches only before the
/// first word. Any other word starting with `-` is printed.
fn is_echo_switch(arg: &str) -> bool {
    arg.strip_prefix('-')
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| matches!(c, 'n' | 'e' | 'E')))
}

/// Text that prints as itself and that xargs passes on as one item: no blank, quote or backslash
/// (xargs' own parsing), no `%` for printf (a directive), and not empty.
fn is_plain(word: &str, printf: bool) -> bool {
    !word.is_empty()
        && !word.chars().any(|c| c.is_whitespace() || matches!(c, '\'' | '"' | '\\') || (printf && c == '%'))
        && !(printf && word.starts_with('-'))
}

#[cfg(test)]
#[path = "printed_tests.rs"]
mod tests;
