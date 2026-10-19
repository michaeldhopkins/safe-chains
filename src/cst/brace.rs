//! Brace expansion of one literal, the way this machine's bash does it.
//!
//! `/bin/bash` here is 3.2. A comma group (`a{1,2}b`) and a sequence (`{1..5}`, `{c..a}`, `{t..t}`)
//! both expand. A leading zero is part of the integer (`{01..03}` prints `1 2 3`), and a third
//! field (`{1..5..2}`) is not a sequence. Mixed types and multi-character letters stay written as
//! they are. A sequence longer than the cap is [`UNRESOLVED_BRACE`] after its prefix, not a
//! partial list and not a made-up relative name.

use super::eval::{BRACE_EXPANSION_CAP, UNPINNABLE, UNRESOLVED_BRACE};

/// Brace-expand a literal. Nested groups recurse. A group this bash leaves written stays as it
/// is. Too many brace groups fails closed to the unpinnable sentinel.
pub(super) fn expand(s: &str) -> Vec<String> {
    if s.matches('{').count() > 8 {
        return vec![UNPINNABLE.to_string()];
    }
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if let Some(group) = closed_group(&chars, i) {
            let content = group_text(&chars, group.open + 1, group.close);
            let expanded = match group.kind {
                GroupKind::Comma => Some(expand_alts(&chars, &group, &split_top_commas(&content), true)),
                GroupKind::Sequence => sequence_terms(&content).map(|terms| match terms {
                    Sequence::Terms(terms) => expand_alts(&chars, &group, &terms, false),
                    Sequence::Unresolved => unresolved_span(&chars, &group),
                }),
                GroupKind::Literal => None,
            };
            if let Some(words) = expanded {
                return words;
            }
        }
        i += 1;
    }
    vec![s.to_string()]
}

struct Group {
    open: usize,
    close: usize,
    kind: GroupKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GroupKind {
    Comma,
    Sequence,
    Literal,
}

/// The first closed `{...}` at `open`, classified by what this bash expands.
fn closed_group(chars: &[char], open: usize) -> Option<Group> {
    if chars.get(open) != Some(&'{') {
        return None;
    }
    let mut depth = 1;
    let mut j = open + 1;
    let mut comma = false;
    let mut dots = false;
    while j < chars.len() {
        match chars[j] {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            ',' if depth == 1 => comma = true,
            '.' if depth == 1 && chars.get(j + 1) == Some(&'.') => dots = true,
            _ => {}
        }
        j += 1;
    }
    if j >= chars.len() || depth != 0 {
        return None;
    }
    let kind = if comma {
        GroupKind::Comma
    } else if dots {
        GroupKind::Sequence
    } else {
        GroupKind::Literal
    };
    Some(Group { open, close: j, kind })
}

fn group_text(chars: &[char], start: usize, end: usize) -> String {
    chars[start..end].iter().collect()
}

enum Sequence {
    Terms(Vec<String>),
    Unresolved,
}

/// Terms of a `{x..y}` sequence, or `None` when this bash leaves the braces written.
fn sequence_terms(content: &str) -> Option<Sequence> {
    let (lhs, rhs) = content.split_once("..")?;
    if rhs.contains("..") || lhs.is_empty() || rhs.is_empty() {
        return None;
    }
    if let (Some(start), Some(end)) = (parse_int(lhs), parse_int(rhs)) {
        return Some(int_terms(start, end));
    }
    if let (Some(start), Some(end)) = (parse_letter(lhs), parse_letter(rhs)) {
        return Some(Sequence::Terms(letter_terms(start, end)));
    }
    None
}

fn parse_int(text: &str) -> Option<i64> {
    let (sign, digits) = match text.as_bytes().first()? {
        b'+' => (1i64, &text[1..]),
        b'-' => (-1, &text[1..]),
        _ => (1, text),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    sign.checked_mul(digits.parse().ok()?)
}

fn parse_letter(text: &str) -> Option<u8> {
    let bytes = text.as_bytes();
    (bytes.len() == 1 && bytes[0].is_ascii_alphabetic()).then_some(bytes[0])
}

fn int_terms(start: i64, end: i64) -> Sequence {
    let count = end.abs_diff(start).saturating_add(1);
    if count > BRACE_EXPANSION_CAP as u64 {
        return Sequence::Unresolved;
    }
    let step: i64 = if start <= end { 1 } else { -1 };
    let mut out = Vec::with_capacity(count as usize);
    let mut n = start;
    loop {
        out.push(n.to_string());
        if n == end {
            break;
        }
        n += step;
    }
    Sequence::Terms(out)
}

fn letter_terms(start: u8, end: u8) -> Vec<String> {
    let step: i16 = if start <= end { 1 } else { -1 };
    let mut n = i16::from(start);
    let end = i16::from(end);
    let mut out = Vec::new();
    loop {
        out.push((n as u8 as char).to_string());
        if n == end {
            break;
        }
        n += step;
    }
    out
}

/// Prefix and suffix around `group`, with each alternative expanded. A comma fan-out that passes
/// the cap fails closed to the unpinnable sentinel (the historical comma ceiling). A sequence
/// fan-out that passes it is unresolved: the marker stays after the prefix, so `cat ~/.ssh/{1..300}`
/// is still a read under `~/.ssh`.
fn expand_alts(chars: &[char], group: &Group, alts: &[String], comma: bool) -> Vec<String> {
    let prefix = group_text(chars, 0, group.open);
    let suffix_alts = expand(&group_text(chars, group.close + 1, chars.len()));
    if suffix_alts.iter().any(|alt| alt == UNPINNABLE) {
        return vec![UNPINNABLE.to_string()];
    }
    if suffix_alts.iter().any(|alt| alt.contains(UNRESOLVED_BRACE)) {
        // The suffix did not expand, so each alternative we can still name has to keep its own
        // directory. Dropping them for a bare marker would let `cat {~/.ssh/id_rsa,}{1..300}`
        // through as a read of a relative name.
        let mut kept = Vec::new();
        for alt in alts {
            let pieces = expand(alt);
            if pieces.iter().any(|piece| piece == UNPINNABLE) {
                return vec![UNPINNABLE.to_string()];
            }
            for piece in pieces {
                if kept.len() >= BRACE_EXPANSION_CAP {
                    return vec![UNPINNABLE.to_string()];
                }
                if piece.contains(UNRESOLVED_BRACE) {
                    kept.push(format!("{prefix}{piece}"));
                } else {
                    kept.push(format!("{prefix}{piece}{UNRESOLVED_BRACE}"));
                }
            }
        }
        if kept.is_empty() {
            return vec![format!("{prefix}{UNRESOLVED_BRACE}")];
        }
        return kept;
    }
    let mut out = Vec::new();
    for alt in alts {
        let pieces = expand(alt);
        if pieces.iter().any(|piece| piece == UNPINNABLE) {
            return vec![UNPINNABLE.to_string()];
        }
        for alt_x in &pieces {
            for suf_x in &suffix_alts {
                if out.len() >= BRACE_EXPANSION_CAP {
                    if comma {
                        return vec![UNPINNABLE.to_string()];
                    }
                    return vec![format!("{prefix}{UNRESOLVED_BRACE}")];
                }
                out.push(format!("{prefix}{alt_x}{suf_x}"));
            }
        }
    }
    out
}

/// A sequence longer than the cap. The marker replaces the sequence only, so the directory in
/// front of it still classifies (`/etc/{1..300}` stays under `/etc`).
fn unresolved_span(chars: &[char], group: &Group) -> Vec<String> {
    let prefix = group_text(chars, 0, group.open);
    let suffix_alts = expand(&group_text(chars, group.close + 1, chars.len()));
    if suffix_alts.iter().any(|alt| alt == UNPINNABLE) || suffix_alts.len() > BRACE_EXPANSION_CAP {
        return vec![UNPINNABLE.to_string()];
    }
    if suffix_alts.iter().any(|alt| alt.contains(UNRESOLVED_BRACE)) {
        return vec![format!("{prefix}{UNRESOLVED_BRACE}")];
    }
    suffix_alts.into_iter().map(|suffix| format!("{prefix}{UNRESOLVED_BRACE}{suffix}")).collect()
}

/// Split on commas at brace-nesting depth 0, so `a,{b,c},d` → [`a`, `{b,c}`, `d`].
fn split_top_commas(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '{' => {
                depth += 1;
                cur.push(c);
            }
            '}' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => parts.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    parts.push(cur);
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// What bash prints for one unquoted word. Brace expansion runs because the word is not quoted.
    fn bash_sequence_words(word: &str) -> Vec<String> {
        let script = format!("printf \"%s\\n\" {word}");
        let out = std::process::Command::new("bash")
            .args(["--noprofile", "--norc", "-c", &script])
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("LC_ALL", "C")
            .output()
            .unwrap_or_else(|err| panic!("bash: {err}"));
        assert!(out.status.success(), "bash rejected {word}: {}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8(out.stdout).expect("utf-8").split_terminator('\n').map(str::to_string).collect()
    }

    #[test]
    fn sequence_spellings_match_bash() {
        let mut mismatches = Vec::new();
        for word in [
            ".gi{t..t}", ".{g..i}it", ".g{i..i}t", ".GI{T..T}", "log.{1..3}", "sub/.gi{t..t}", "{1..5}", "{5..1}", "{01..05}", "{1..05}",
            "{1..10..2}", "{10..1..2}", "{1..10..-2}", "{a..c}", "{c..a}", "{a..e..2}", "{t..t}", "{g..i}", "{1..a}", "{a..1}", "{ab..cd}",
            "{a..bb}", "{1..2..x}", "x{1..2}y{3..4}", "{1..3,5}",
        ] {
            let bash = bash_sequence_words(word);
            let ours = expand(word);
            if ours != bash {
                mismatches.push(format!("{word}: ours {ours:?} bash {bash:?}"));
            }
        }
        assert!(mismatches.is_empty(), "sequence spellings differ from bash:\n{}", mismatches.join("\n"));
    }

    fn pad_num(n: i32, width: usize) -> String {
        let digits = n.unsigned_abs().to_string();
        let body = if width > digits.len() { format!("{digits:0>width$}") } else { digits };
        if n < 0 { format!("-{body}") } else { body }
    }

    fn small_span(start: i32, end: i32, step: i32) -> bool {
        let step = u64::from(step.unsigned_abs()).max(1);
        u64::from(start.abs_diff(end)) / step < 8
    }

    fn arb_numeric() -> impl Strategy<Value = String> {
        (1usize..=4, -12i32..=12, -12i32..=12, 0i32..=4, any::<bool>()).prop_filter_map(
            "span stays inside the expansion cap",
            |(width, start, end, step, reversed)| {
                let (start, end) = if reversed { (end, start) } else { (start, end) };
                small_span(start, end, step).then_some(())?;
                let left = pad_num(start, width);
                let right = pad_num(end, width);
                Some(if step == 1 { format!("{{{left}..{right}}}") } else { format!("{{{left}..{right}..{step}}}") })
            },
        )
    }

    fn arb_letter() -> impl Strategy<Value = String> {
        (
            prop::char::range('a', 'z'),
            prop::char::range('a', 'z'),
            prop::char::range('A', 'Z'),
            prop::char::range('A', 'Z'),
            0i32..=3,
            any::<bool>(),
        )
            .prop_filter_map("letter span stays short", |(a, b, c, d, step, upper)| {
                let (start, end) = if upper { (c, d) } else { (a, b) };
                ((start as u8).abs_diff(end as u8) <= 12).then_some(())?;
                Some(if step <= 1 { format!("{{{start}..{end}}}") } else { format!("{{{start}..{end}..{step}}}") })
            })
    }

    fn arb_invalid() -> impl Strategy<Value = String> {
        prop_oneof![
            (any::<i8>(), prop::char::range('a', 'z')).prop_map(|(n, c)| format!("{{{n}..{c}}}")),
            (prop::char::range('a', 'z'), any::<i8>()).prop_map(|(c, n)| format!("{{{c}..{n}}}")),
            (prop::char::range('a', 'z'), prop::char::range('a', 'z'), prop::char::range('a', 'z'))
                .prop_map(|(a, b, c)| format!("{{{a}{b}..{c}}}")),
            (prop::char::range('a', 'z'), prop::char::range('a', 'z'), prop::char::range('a', 'z'))
                .prop_map(|(a, b, c)| format!("{{{a}..{b}{c}}}")),
            (any::<i8>(), any::<i8>()).prop_map(|(a, b)| format!("{{{a}..{b}..x}}")),
        ]
    }

    fn arb_affix() -> impl Strategy<Value = String> {
        prop::string::string_regex("[A-Za-z0-9._]{0,3}").unwrap()
    }

    fn arb_sequence_word() -> impl Strategy<Value = String> {
        (arb_affix(), prop_oneof![arb_numeric(), arb_letter(), arb_invalid()], arb_affix())
            .prop_map(|(prefix, body, suffix)| format!("{prefix}{body}{suffix}"))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]
        /// A sequence brace expands to the words bash prints, including padding, a step, letters,
        /// reverse order, and the invalid forms bash leaves written as they are.
        #[test]
        fn a_sequence_brace_matches_bash(word in arb_sequence_word()) {
            prop_assert_eq!(expand(&word), bash_sequence_words(&word));
        }
    }
}
