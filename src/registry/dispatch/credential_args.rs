//! The credential-name check of a `first_arg` glob, out of `registry/dispatch.rs`.

use crate::parse::Token;

/// Whether any argument from the glob's first one on names secret material.
///
/// Every positional is asked, and each comma-separated item in it: kubectl reads several kinds in
/// one call (`get pods,secrets`, `get pod/a secret/b`), so a credential kind in second place reads
/// as much as one in first. The value of a listed valued flag is not an argument (`-n secrets` is a
/// namespace). Matched CASE-INSENSITIVELY: kubectl resource kinds are case-insensitive (`get
/// Secret`/`SECRET` reads secrets), so a case-variant must not slip past the deny into the `*`
/// allow. The allow-glob stays case-sensitive — a case-insensitive ALLOW would be fail-open.
///
/// An item holding `*`, `?` or `[` is read as the pattern the shell expands against the directory,
/// and counts when any name it could match is a credential name.
pub(super) fn names_a_credential(arguments: &[Token], credential: &[String], valued: &[&[String]]) -> bool {
    if credential.is_empty() {
        return false;
    }
    let is_valued = |word: &str| valued.iter().any(|list| list.iter().any(|f| f == word));
    let matches = |item: &str| credential.iter().any(|p| item_names(item, p));
    let mut words = arguments.iter().map(Token::as_str);
    while let Some(word) = words.next() {
        if word.starts_with('-') && word != "-" {
            if is_valued(word) {
                words.next();
            }
            continue;
        }
        if word.split(',').any(&matches) {
            return true;
        }
    }
    false
}

/// Whether `item` names `credential`, an exact name or a prefix ending in `*`.
fn item_names(item: &str, credential: &str) -> bool {
    let (name, open) = credential.strip_suffix('*').map_or((credential, false), |prefix| (prefix, true));
    if item.contains(['*', '?', '[']) {
        return pattern_could_spell(item, name, open);
    }
    if open {
        item.len() >= name.len() && item.as_bytes()[..name.len()].eq_ignore_ascii_case(name.as_bytes())
    } else {
        item.eq_ignore_ascii_case(name)
    }
}

enum Piece {
    Any,
    One,
    Class(Vec<char>, bool),
    Literal(char),
}

fn pieces(pattern: &str) -> Vec<Piece> {
    let chars: Vec<char> = pattern.to_ascii_lowercase().chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' => out.push(Piece::Any),
            '?' => out.push(Piece::One),
            '[' => {
                let negated = matches!(chars.get(i + 1), Some('!' | '^'));
                let body = i + 1 + usize::from(negated);
                if let Some(close) = chars.iter().skip(body + 1).position(|c| *c == ']').map(|p| p + body + 1) {
                    out.push(Piece::Class(chars[body..close].to_vec(), negated));
                    i = close + 1;
                    continue;
                }
                out.push(Piece::Literal('['));
            }
            c => out.push(Piece::Literal(c)),
        }
        i += 1;
    }
    out
}

/// Whether the shell pattern could match `name` exactly or, when `open`, some name beginning with
/// it. Ranges in a bracket class are read as matching any character, which can only widen.
fn pattern_could_spell(pattern: &str, name: &str, open: bool) -> bool {
    let pieces = pieces(pattern);
    let fits = |piece: &Piece, c: char| match piece {
        Piece::Any | Piece::One => true,
        Piece::Literal(l) => *l == c,
        Piece::Class(set, negated) => set.contains(&'-') || set.contains(&c) != *negated,
    };
    let close = |states: &mut Vec<bool>| {
        for at in 0..pieces.len() {
            if states[at] && matches!(pieces[at], Piece::Any) {
                states[at + 1] = true;
            }
        }
    };
    let mut states = vec![false; pieces.len() + 1];
    states[0] = true;
    close(&mut states);
    for c in name.to_ascii_lowercase().chars() {
        let mut next = vec![false; pieces.len() + 1];
        for (at, piece) in pieces.iter().enumerate() {
            if states[at] && fits(piece, c) {
                next[if matches!(piece, Piece::Any) { at } else { at + 1 }] = true;
            }
        }
        close(&mut next);
        states = next;
    }
    if open { states.iter().any(|s| *s) } else { states[pieces.len()] }
}
#[cfg(test)]
mod tests {
    use super::names_a_credential;
    use crate::parse::Token;
    use proptest::prelude::*;

    /// A `]` straight after `[` or `[!` is a member of the class, not its end.
    #[test]
    fn a_class_opening_with_a_closing_bracket_still_spells_the_name() {
        let credential = vec!["secret".to_string()];
        for pattern in ["[]s]ecret", "[!]x]ecret", "[]]ecret", "[!a]ecret", "[s]ecret", "[xs]ecret"] {
            let tokens = [Token::from_test(pattern)];
            let spells = pattern != "[]]ecret";
            assert_eq!(names_a_credential(&tokens, &credential, &[]), spells, "{pattern}");
        }
    }

    proptest! {
        /// A credential name with any of its characters replaced by `?` or a one-letter class, or
        /// any run replaced by `*`, is still read as that name; a pattern whose literal letters
        /// cannot begin it is not.
        #[test]
        fn a_pattern_that_could_spell_a_credential_name_is_one(
            which in 0usize..3,
            marks in prop::collection::vec(0u8..4, 13),
            cut in 0usize..13,
            other in "[b-q][a-z]{0,6}[*?]?",
        ) {
            let credential = vec!["secret".to_string(), "secrets/*".to_string(), "aws_session_token".to_string()];
            let name = ["secret", "secrets/db", "aws_session_token"][which];
            let mut pattern = String::new();
            for (at, c) in name.chars().enumerate() {
                match marks.get(at).copied().unwrap_or(0) {
                    1 => pattern.push('?'),
                    2 => pattern.push_str(&format!("[{c}]")),
                    3 if at >= cut => { pattern.push('*'); break; }
                    _ => pattern.push(c),
                }
            }
            let tokens = [Token::from_test(&pattern)];
            prop_assert!(names_a_credential(&tokens, &credential, &[]), "{}", pattern);
            let tokens = [Token::from_test(&other)];
            prop_assert!(!names_a_credential(&tokens, &credential, &[]), "{}", other);
        }

        /// A credential name is found wherever it stands among the arguments, alone or as one
        /// item of a comma list, in any case; the same words without it, or with it only as a
        /// listed flag's value, are not a credential read.
        #[test]
        fn a_credential_name_is_found_in_any_position(
            others in prop::collection::vec("[a-r][a-z]{1,8}", 1..5),
            at in 0usize..5,
            comma in any::<bool>(),
            upper in any::<bool>(),
        ) {
            let credential = vec!["secret".to_string(), "secrets/*".to_string()];
            let valued = vec!["-n".to_string()];
            let name = if upper { "SECRET" } else { "secret" };
            let at = at.min(others.len());
            let mut items = others.clone();
            items.insert(at, name.to_string());
            let words: Vec<String> = if comma { vec![items.join(",")] } else { items };
            let tokens: Vec<Token> = words.iter().map(|w| Token::from_test(w)).collect();
            prop_assert!(names_a_credential(&tokens, &credential, &[&valued]), "{:?}", words);
            let plain: Vec<Token> = others.iter().map(|w| Token::from_test(w)).collect();
            prop_assert!(!names_a_credential(&plain, &credential, &[&valued]), "{:?}", others);
            let mut as_value: Vec<Token> = plain.clone();
            as_value.push(Token::from_test("-n"));
            as_value.push(Token::from_test(name));
            prop_assert!(!names_a_credential(&as_value, &credential, &[&valued]), "{:?} -n {}", others, name);
        }
    }
}
