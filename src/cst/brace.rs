//! Brace expansion of one literal, limited to what every bash expands alike.
//!
//! A comma group (`a{1,2}b`) and a plain sequence (`{1..5}`, `{c..a}`, `{t..t}`) expand the same in
//! bash 3.2 and 5. A sequence they disagree on is [`UNRESOLVED_BRACE`]: a third field
//! (`{1..5..2}`, left written by 3.2 and stepped by 5), a zero-padded end (`{01..03}`, unpadded by
//! 3.2), letters of both cases (`{a..Z}`, whose punctuation 3.2 quotes away), or a group inside the
//! sequence. Other mixed or multi-character ends stay written as they are. A sequence longer than
//! the cap is unresolved after its prefix, not a partial list and not a made-up relative name.

use super::eval::{BRACE_EXPANSION_CAP, UNPINNABLE, UNRESOLVED_BRACE};

/// The byte offset of the first `{` in `s` that does not open a group expanded within `s`, so
/// text after `s` may still close it. `word_start` says whether `s` begins the word: bash reads a
/// `{}` there as plain text.
pub(super) fn spanning_open(s: &str, word_start: bool) -> Option<usize> {
    let chars: Vec<char> = s.chars().collect();
    let offsets: Vec<usize> = s.char_indices().map(|(offset, _)| offset).collect();
    (leading_pair(s, word_start)..chars.len())
        .filter(|&i| chars[i] == '{')
        .find(|&i| closed_group(&chars, i).is_none_or(|group| matches!(group.kind, GroupKind::Literal | GroupKind::Dotted)))
        .map(|i| offsets[i])
}

fn leading_pair(s: &str, word_start: bool) -> usize {
    if word_start && s.starts_with("{}") { 2 } else { 0 }
}

/// Brace-expand a literal that begins its word. Nested groups recurse. A group this bash leaves
/// written stays as it is. Too many brace groups fails closed to the unpinnable sentinel.
pub(super) fn expand(s: &str) -> Vec<String> {
    expand_from(s, true)
}

/// Brace-expand a literal that follows a quoted or substituted part of its word.
pub(super) fn expand_after(s: &str) -> Vec<String> {
    expand_from(s, false)
}

fn expand_from(s: &str, word_start: bool) -> Vec<String> {
    if s.matches('{').count() > 8 {
        return vec![UNPINNABLE.to_string()];
    }
    let chars: Vec<char> = s.chars().collect();
    let mut i = leading_pair(s, word_start);
    while i < chars.len() {
        if let Some(group) = closed_group(&chars, i) {
            let content = group_text(&chars, group.open + 1, group.close);
            let expanded = match group.kind {
                GroupKind::Comma => Some(expand_alts(&chars, &group, &split_top_commas(&content), true)),
                GroupKind::Sequence => sequence_terms(&content).map(|terms| match terms {
                    Sequence::Terms(terms) => expand_alts(&chars, &group, &terms, false),
                    Sequence::Unresolved => unresolved_span(&chars, &group),
                }),
                GroupKind::Dotted if chars[group.close + 1..].contains(&'{') => {
                    Some(vec![format!("{}{UNRESOLVED_BRACE}", group_text(&chars, 0, group.open))])
                }
                GroupKind::Literal | GroupKind::Dotted => None,
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
    /// A dotted text that is not a sequence (`{..a}`). Bash 3.2 expands nothing after it; bash 5 does.
    Dotted,
}

/// The group that opens at `open`, classified by what bash expands. A `}` that closes text with
/// no comma and no valid sequence does not end the group when a later `}` can: bash keeps looking,
/// so `{ab},x}` is the group `ab},x`. A `{}` pair is plain text.
fn closed_group(chars: &[char], open: usize) -> Option<Group> {
    if chars.get(open) != Some(&'{') {
        return None;
    }
    let mut depth = 1;
    let mut j = open + 1 + usize::from(chars.get(open + 1) == Some(&'}'));
    let mut comma = false;
    let mut dots = false;
    let mut ends_at_close = false;
    let mut first_close = None;
    while j < chars.len() {
        match chars[j] {
            '{' if chars.get(j + 1) == Some(&'}') => j += 1,
            '{' => depth += 1,
            '}' if depth > 1 => depth -= 1,
            '}' => {
                let content = group_text(chars, open + 1, j);
                let kind = if comma {
                    Some(GroupKind::Comma)
                } else {
                    (dots && sequence_terms(&content).is_some()).then_some(GroupKind::Sequence)
                };
                if let Some(kind) = kind {
                    return Some(Group { open, close: j, kind });
                }
                first_close.get_or_insert(j);
                if ends_at_close {
                    return Some(Group { open, close: j, kind: GroupKind::Dotted });
                }
            }
            ',' if depth == 1 => comma = true,
            '.' if depth == 1 && chars.get(j + 1) == Some(&'.') => {
                dots = true;
                ends_at_close |= chars.get(j + 2) != Some(&'}');
            }
            _ => {}
        }
        j += 1;
    }
    first_close.map(|close| Group { open, close, kind: GroupKind::Literal })
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
    if content.contains(['{', '}']) {
        return Some(Sequence::Unresolved);
    }
    let (lhs, rhs) = content.split_once("..")?;
    if rhs.contains("..") {
        return Some(Sequence::Unresolved);
    }
    if lhs.is_empty() || rhs.is_empty() {
        return None;
    }
    if let (Some(start), Some(end)) = (parse_int(lhs), parse_int(rhs)) {
        if zero_padded(lhs) || zero_padded(rhs) {
            return Some(Sequence::Unresolved);
        }
        return Some(int_terms(start, end));
    }
    if let (Some(start), Some(end)) = (parse_letter(lhs), parse_letter(rhs)) {
        if start.is_ascii_uppercase() != end.is_ascii_uppercase() {
            return Some(Sequence::Unresolved);
        }
        return Some(Sequence::Terms(letter_terms(start, end)));
    }
    None
}

fn zero_padded(text: &str) -> bool {
    let digits = text.trim_start_matches(['+', '-']);
    digits.len() > 1 && digits.starts_with('0')
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
/// fan-out that passes it is unresolved: the marker stays after the prefix. A word holding the
/// marker is unpinnable, so every read or write of it is refused.
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

/// A sequence longer than the cap, or one the bash versions expand differently: the marker
/// replaces the sequence, and the word is unpinnable wherever it is used.
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

/// Split on commas at brace-nesting depth 0, so `a,{b,c},d` → [`a`, `{b,c}`, `d`]. A `{}` pair
/// and a `}` with nothing open are plain text, as bash reads them.
fn split_top_commas(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'}') => {
                cur.push(c);
                cur.extend(chars.next());
            }
            '{' => {
                depth += 1;
                cur.push(c);
            }
            '}' => {
                depth = (depth - 1).max(0);
                cur.push(c);
            }
            ',' if depth == 0 => parts.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    parts.push(cur);
    parts
}
