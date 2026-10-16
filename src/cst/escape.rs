//! Backslash escapes, and the line continuation bash removes before it splits words: a
//! backslash-newline joins the two halves of a word (`imp\<newline>ort` is `import`), and between
//! words it is whitespace. Inside single quotes and quoted heredocs it stays as written.
//!
//! bash removes a continuation before it recognises anything else, so one inside `$\<nl>(cmd)`,
//! `$X\<nl>Y`, `&\<nl>&` or `<\<nl><` changes what the line means. The parser models only the
//! joins of plain text and the ones beside whitespace; each continuation it removes is judged
//! where it stands in the input, and any other refuses the whole parse.
use super::WordPart;
use std::cell::{Cell, RefCell};
use winnow::ModalResult;
use winnow::combinator::{alt, preceded};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::{any, take_while};

thread_local! {
    static INPUT: RefCell<(usize, String)> = const { RefCell::new((0, String::new())) };
    static REFUSED: Cell<bool> = const { Cell::new(false) };
}

/// Start judging the continuations of a parse of `input`, which every slice the parser sees
/// points into.
pub(super) fn begin(input: &str) {
    INPUT.with(|i| *i.borrow_mut() = (input.as_ptr() as usize, input.to_string()));
    REFUSED.with(|r| r.set(false));
}

/// Whether every continuation removed since [`begin`] was a plain one, and nothing else refused.
pub(super) fn all_plain() -> bool {
    !REFUSED.with(Cell::get)
}

/// Refuse the parse under way: it read something it cannot model.
pub(super) fn refuse() {
    REFUSED.with(|r| r.set(true));
}

/// The command a backquoted substitution runs. bash first removes `\<newline>` and turns a
/// backslash followed by `\`, `$` or a backquote into that character, so a nested, escaped
/// backquote pair runs its command; every other backslash stays.
pub(crate) fn backtick_command(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        match chars.peek() {
            Some(&next @ ('\\' | '$' | '`')) if c == '\\' => {
                out.push(next);
                chars.next();
            }
            Some('\n') if c == '\\' => {
                chars.next();
            }
            _ => out.push(c),
        }
    }
    out
}

pub(super) fn backtick(raw: String) -> WordPart {
    if raw.contains("\\\"") {
        refuse();
    }
    WordPart::Backtick(raw)
}

/// Judge the continuation `at` starts with, by its place in the input being parsed. A slice from
/// anywhere else has no known context and refuses.
fn judge(at: &str) {
    let plain = INPUT.with(|i| {
        let (base, text) = &*i.borrow();
        (at.as_ptr() as usize)
            .checked_sub(*base)
            .filter(|&o| o < text.len())
            .is_some_and(|o| plain_at(text.as_bytes(), o))
    });
    if !plain {
        refuse();
    }
}

/// Whether the continuation at `at` is a plain join of two non-blank word characters (`l\<nl>s`),
/// which belongs to a word and never to inter-token whitespace.
fn joins_word_here(at: &str) -> bool {
    INPUT.with(|i| {
        let (base, text) = &*i.borrow();
        (at.as_ptr() as usize)
            .checked_sub(*base)
            .filter(|&o| o < text.len())
            .is_some_and(|o| plain_at(text.as_bytes(), o) && !touches_blank(text.as_bytes(), o))
    })
}

/// Whether removing the continuation at `b[at..at + 2]` can only join literal text or touch
/// whitespace. Neighbouring continuations are looked through.
pub(super) fn plain_at(b: &[u8], at: usize) -> bool {
    let (start, end, before, after) = span_around(b, at);
    let blank = |c: Option<u8>| c.is_none_or(|c| matches!(c, b' ' | b'\t' | b'\n'));
    let plain = |c: Option<u8>| c.is_some_and(|c| c.is_ascii_alphanumeric() || b"_./-,:".contains(&c));
    !splits_syntax_token(b, start, end) && (blank(before) || blank(after) || (plain(before) && plain(after)))
}

fn touches_blank(b: &[u8], at: usize) -> bool {
    let (_, _, before, after) = span_around(b, at);
    let blank = |c: Option<u8>| c.is_none_or(|c| matches!(c, b' ' | b'\t' | b'\n'));
    blank(before) || blank(after)
}

fn span_around(b: &[u8], at: usize) -> (usize, usize, Option<u8>, Option<u8>) {
    let mut start = at;
    while start >= 2 && b[start - 2] == b'\\' && b[start - 1] == b'\n' {
        start -= 2;
    }
    let mut end = at + 2;
    while b.get(end) == Some(&b'\\') && b.get(end + 1) == Some(&b'\n') {
        end += 2;
    }
    (start, end, start.checked_sub(1).map(|j| b[j]), b.get(end).copied())
}

/// Whether the plain run touching `b[start..end]` is a parameter name, a shell keyword, an
/// assignment name, a function header or a redirection descriptor, which the parser reads raw.
fn splits_syntax_token(b: &[u8], start: usize, end: usize) -> bool {
    let is_plain = |c: u8| c.is_ascii_alphanumeric() || b"_./-,:".contains(&c);
    let mut left = start;
    while left > 0 {
        if is_plain(b[left - 1]) {
            left -= 1;
        } else if left >= 2 && b[left - 2] == b'\\' && b[left - 1] == b'\n' {
            left -= 2;
        } else {
            break;
        }
    }
    let mut right = end;
    while right < b.len() {
        if is_plain(b[right]) {
            right += 1;
        } else if b.get(right) == Some(&b'\\') && b.get(right + 1) == Some(&b'\n') {
            right += 2;
        } else {
            break;
        }
    }
    if left == start && right == end {
        return false;
    }
    if left > 0 && matches!(b[left - 1], b'$' | b'{' | b'>' | b'<' | b'&') {
        return true;
    }
    if left < start && b.get(right).is_some_and(|c| matches!(c, b'=' | b'(' | b'>' | b'<')) {
        return true;
    }
    let mut word = Vec::with_capacity(right - left);
    let mut i = left;
    while i < right {
        if b[i] == b'\\' && b.get(i + 1) == Some(&b'\n') {
            i += 2;
        } else {
            word.push(b[i]);
            i += 1;
        }
    }
    matches!(
        word.as_slice(),
        b"if"
            | b"then"
            | b"elif"
            | b"else"
            | b"fi"
            | b"for"
            | b"in"
            | b"do"
            | b"done"
            | b"while"
            | b"until"
            | b"case"
            | b"esac"
            | b"function"
            | b"select"
            | b"time"
            | b"coproc"
    )
}

/// The heredoc body line starting at `start`: where it ends, and its text. In a body that expands,
/// a line ending in a continuation runs on into the next, and bash compares the joined line with
/// the delimiter, so `EO\<nl>F` ends a `<<EOF` body.
pub(super) fn heredoc_line(s: &str, start: usize, joins: bool) -> (usize, String) {
    let mut text = String::new();
    let mut at = start;
    loop {
        let end = s[at..].find('\n').map_or(s.len(), |rel| at + rel);
        let line = &s[at..end];
        let slashes = line.len() - line.trim_end_matches('\\').len();
        if !(joins && slashes % 2 == 1 && end < s.len()) {
            text.push_str(line);
            return (end, text);
        }
        text.push_str(&line[..line.len() - 1]);
        at = end + 1;
    }
}

/// Consumes one line continuation in whitespace, if the input starts with one and it does not
/// join two halves of a word.
pub(super) fn skip_continuation(input: &mut &str) -> bool {
    if !input.starts_with("\\\n") || joins_word_here(input) {
        return false;
    }
    judge(input);
    *input = &input[2..];
    true
}

/// Consumes one line continuation inside a heredoc delimiter, if the input starts with one.
pub(super) fn join_continuation(input: &mut &str) -> bool {
    let found = judged(input);
    if found {
        *input = &input[2..];
    }
    found
}

/// A descriptor target for `>&`: `-` or digits, ending at a shell token boundary so a file path
/// starting with digits (`>&1/../x`) falls through to the file-redirect arm.
pub(super) fn fd_target(input: &mut &str) -> ModalResult<String> {
    let dst: &str = alt(("-", take_while(1.., |c: char| c.is_ascii_digit()))).parse_next(input)?;
    if !input.is_empty() && !matches!(input.as_bytes()[0], b' ' | b'\t' | b'\n' | b';' | b'&' | b'|' | b'(' | b')' | b'<' | b'>') {
        return Err(ErrMode::Backtrack(ContextError::new()));
    }
    Ok(dst.to_string())
}

/// Whether `input` starts with a continuation, judging it if so.
fn judged(input: &str) -> bool {
    let found = input.starts_with("\\\n");
    if found {
        judge(input);
    }
    found
}

/// A word made of nothing but line continuations is no word at all to bash.
pub(super) fn only_continuations(parts: &[WordPart]) -> bool {
    parts.iter().all(|p| matches!(p, WordPart::Lit(s) if s.is_empty()))
}

pub(super) fn escaped(input: &mut &str) -> ModalResult<WordPart> {
    judged(input);
    preceded('\\', any)
        .map(|c: char| if c == '\n' { WordPart::Lit(String::new()) } else { WordPart::Escape(c) })
        .parse_next(input)
}

pub(super) fn dq_escape(input: &mut &str) -> ModalResult<WordPart> {
    judged(input);
    preceded('\\', any)
        .map(|c: char| match c {
            '"' | '\\' | '$' | '`' => WordPart::Escape(c),
            '\n' => WordPart::Lit(String::new()),
            _ => WordPart::Lit(format!("\\{c}")),
        })
        .parse_next(input)
}

#[cfg(test)]
#[path = "escape_tests.rs"]
mod tests;
