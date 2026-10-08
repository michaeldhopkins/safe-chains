//! Bash's two `$`-quotes: ANSI-C `$'…'` and locale `$"…"`.
//!
//! Both must be read as bash reads them. Left to the generic `$` rule, `$'-delete'` became the
//! literal `$` followed by a single-quoted `-delete`: a word starting with `$`, which no flag
//! allowlist ever matches, while bash hands the command `-delete`.

use super::budget;
use super::{Word, WordPart};
use winnow::ModalResult;
use winnow::error::{ContextError, ErrMode};

fn backtrack<T>() -> ModalResult<T> {
    Err(ErrMode::Backtrack(ContextError::new()))
}

/// `$'…'`, kept RAW (the text between the quotes) so `--explain` echoes what was typed; the value
/// is [`decode`]d on evaluation. A backslash escapes the next byte for scanning, so `$'a\'b'` is
/// one quote holding `a\'b`. An unclosed quote consumes nothing.
pub(super) fn ansi_c_quoted(input: &mut &str) -> ModalResult<WordPart> {
    let Some(body) = input.strip_prefix("$'") else {
        return backtrack();
    };
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i] != b'\'' {
        i += if bytes[i] == b'\\' { 2 } else { 1 };
    }
    if !budget::charge(0, i.min(bytes.len())) || i >= bytes.len() {
        return backtrack();
    }
    let raw = body[..i].to_string();
    *input = &body[i + 1..];
    Ok(WordPart::AnsiC(raw))
}

/// `$"…"`: bash translates the string through the locale's message catalog, and with none
/// installed (the case everywhere outside a localized script) the result is the plain
/// double-quoted string, expansions included. So it parses as exactly that.
pub(super) fn locale_quoted(input: &mut &str, double_quoted: fn(&mut &str) -> ModalResult<WordPart>) -> ModalResult<WordPart> {
    if !input.starts_with("$\"") {
        return backtrack();
    }
    let mut rest = &input[1..];
    let part = double_quoted(&mut rest)?;
    *input = rest;
    Ok(part)
}

/// The value bash gives the body of `$'…'`, byte for byte (checked against bash 5.3):
///
/// - `\a \b \e \E \f \n \r \t \v \\ \' \" \?` are the usual characters;
/// - `\NNN` is one to three octal digits, taken modulo 256;
/// - `\xHH` is one or two hex digits, `\uHHHH` one to four, `\UHHHHHHHH` one to eight; with no
///   digit the escape stays literal (`\x`);
/// - `\cX` is the control character `X & 0x1f` (`\c?` is DEL, and `\c\\` reads one backslash);
/// - any other `\X` stays as typed, backslash included;
/// - a NUL from any escape ends the string there.
///
/// Bytes that are not valid UTF-8 become U+FFFD. Bash keeps the raw byte, but no such byte can
/// spell anything the classifier matches, so the substitution changes no verdict.
pub(crate) fn decode(raw: &str) -> String {
    let b = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'\\' || i + 1 >= b.len() {
            out.push(b[i]);
            i += 1;
            continue;
        }
        let c = b[i + 1];
        i += 2;
        match c {
            b'a' => out.push(7),
            b'b' => out.push(8),
            b'e' | b'E' => out.push(27),
            b'f' => out.push(12),
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            b'v' => out.push(11),
            b'\\' | b'\'' | b'"' | b'?' => out.push(c),
            b'0'..=b'7' => {
                let (v, used) = digits(&b[i..], 8, 2);
                out.push(((u32::from(c - b'0') << (3 * used)) + v) as u8);
                i += used;
            }
            b'x' | b'u' | b'U' => {
                let max = match c {
                    b'x' => 2,
                    b'u' => 4,
                    _ => 8,
                };
                let (v, used) = digits(&b[i..], 16, max);
                i += used;
                if used == 0 {
                    out.extend_from_slice(&[b'\\', c]);
                } else if c == b'x' {
                    out.push(v as u8);
                } else {
                    let ch = char::from_u32(v).unwrap_or(char::REPLACEMENT_CHARACTER);
                    out.extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes());
                }
            }
            b'c' if i < b.len() => {
                let x = b[i];
                i += 1;
                if x == b'\\' && b.get(i) == Some(&b'\\') {
                    i += 1;
                }
                out.push(if x == b'?' { 0x7f } else { x.to_ascii_uppercase() & 0x1f });
            }
            _ => out.extend_from_slice(&[b'\\', c]),
        }
    }
    if let Some(nul) = out.iter().position(|&x| x == 0) {
        out.truncate(nul);
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Up to `max` leading digits of `radix`, and how many were read.
fn digits(b: &[u8], radix: u32, max: usize) -> (u32, usize) {
    let mut v = 0u32;
    let mut used = 0;
    while used < max {
        let Some(d) = b.get(used).and_then(|&x| char::from(x).to_digit(radix)) else {
            break;
        };
        v = v * radix + d;
        used += 1;
    }
    (v, used)
}

impl Word {
    /// Whether any part of the word is an ANSI-C quote, whose decoded value can differ from its
    /// spelling in every way that matters (`$'\055delete'` is `-delete`).
    pub fn has_ansi_c(&self) -> bool {
        self.0.iter().any(|p| matches!(p, WordPart::AnsiC(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn decodes_as_bash_does() {
        let cases: &[(&str, &str)] = &[
            ("-delete", "-delete"),
            ("\\x2d\\x2Ddelete", "--delete"),
            ("\\55x", "-x"),
            ("\\0555", "-5"),
            ("\\u2d", "-"),
            ("\\u", "\\u"),
            ("\\xZ", "\\xZ"),
            ("\\x", "\\x"),
            ("\\x41g", "Ag"),
            ("\\101\\1012", "AA2"),
            ("\\z", "\\z"),
            ("\\q\\?\\\"", "\\q?\""),
            ("a\\'b", "a'b"),
            ("\\cA", "\u{1}"),
            ("\\c?", "\u{7f}"),
            ("\\c\\\\x", "\u{1c}x"),
            ("\\c\\x", "\u{1c}x"),
            ("\\c-", "\r"),
            ("\\c", "\\c"),
            ("\\e\\E", "\u{1b}\u{1b}"),
            ("\\777", "\u{fffd}"),
            ("\\u00410", "A0"),
            ("\\U0001F600", "\u{1F600}"),
            ("\\9", "\\9"),
            ("\\t\\n", "\t\n"),
        ];
        for (raw, want) in cases {
            assert_eq!(decode(raw), *want, "$'{raw}'");
        }
    }

    #[test]
    fn the_quote_renders_as_typed() {
        for input in ["echo $'a\\'b'", "find / $'\\x2ddelete' x"] {
            let script = crate::cst::parse(input).expect("parses");
            assert_eq!(script.to_string().trim_end_matches(';'), input);
            assert_eq!(crate::cst::parse(&script.to_string()), Some(script));
        }
    }

    #[test]
    fn a_nul_ends_the_string() {
        for raw in ["a\\0b", "a\\x00b", "a\\u0000b", "a\\c@b", "a\\400b", "a\\x00"] {
            assert_eq!(decode(raw), "a", "$'{raw}'");
        }
    }
}
