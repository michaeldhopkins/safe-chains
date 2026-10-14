use std::collections::HashMap;

use super::Role;
use crate::parse::Token;

/// If `tokens[i]` is one of `spec`'s mapped flags in any form — `-o V`, `--output=V`, glued
/// `-oV`, or clustered `-qO/etc/x` — return its (role, value, tokens-consumed).
pub(super) fn match_flag<'a>(flags: &HashMap<String, Role>, tokens: &'a [Token], i: usize) -> Option<(Role, &'a str, usize)> {
    let t = tokens[i].as_str();
    for (flag, &role) in flags {
        if t == flag {
            return Some((role, tokens.get(i + 1).map_or("", Token::as_str), 2));
        }
        // A glued `flag=value`. Handles BOTH `--flag=v` (GNU) and single-dash-long `-flag=v`
        // (the Go-flag convention — terraform's `-out=…`/`-state-out=…`, which otherwise sailed
        // past this gate). The `=` must follow the EXACT flag name, so a short flag like `-o`
        // can't spuriously match `-output=…` — only its own `-o=…`.
        if let Some(v) = t.strip_prefix(flag.as_str()).and_then(|r| r.strip_prefix('=')) {
            return Some((role, v, 1));
        }
    }
    // A short flag glued to its value, possibly behind boolean flags in a cluster (`-o/etc/x`,
    // `-qO/etc/x`). Take the LEFTMOST mapped short-flag letter — a boolean prefix can't hide the
    // write. Its value is the rest of the token, or the NEXT token when the letter is last
    // (`-qO /etc/x`); `-qO-` reads `-` (stdout).
    let cluster = t.strip_prefix('-').filter(|c| !c.starts_with('-') && !c.is_empty())?;
    flags
        .iter()
        .filter(|(flag, _)| flag.len() == 2 && flag.starts_with('-'))
        .filter_map(|(flag, &role)| cluster.find(&flag[1..]).map(|p| (p, role)))
        .min_by_key(|&(p, _)| p)
        .map(|(p, role)| match &cluster[p + 1..] {
            "" => (role, tokens.get(i + 1).map_or("", Token::as_str), 2),
            glued => (role, glued, 1),
        })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use proptest::prelude::*;

    use super::match_flag;
    use crate::parse::Token;
    use crate::pathgate::Role;

    fn toks(words: &[&str]) -> Vec<Token> {
        words.iter().map(|w| Token::from_raw((*w).to_string())).collect()
    }

    proptest! {
        #[test]
        fn every_spelling_of_a_mapped_flag_yields_the_same_value(
            letter in "[a-z]",
            booleans in "[A-Z]{0,3}",
            value in "[a-z0-9./~][a-z0-9./~-]{0,8}",
        ) {
            let short = format!("-{letter}");
            let flags = HashMap::from([(short.clone(), Role::Write)]);
            let separate = toks(&["cmd", &short, &value]);
            let equals = toks(&["cmd", &format!("{short}={value}")]);
            let glued = toks(&["cmd", &format!("{short}{value}")]);
            let clustered = toks(&["cmd", &format!("-{booleans}{letter}"), &value]);
            prop_assert_eq!(match_flag(&flags, &separate, 1), Some((Role::Write, value.as_str(), 2)));
            prop_assert_eq!(match_flag(&flags, &equals, 1), Some((Role::Write, value.as_str(), 1)));
            prop_assert_eq!(match_flag(&flags, &clustered, 1), Some((Role::Write, value.as_str(), 2)));
            prop_assert_eq!(match_flag(&flags, &glued, 1).map(|(r, _, _)| r), Some(Role::Write));
        }
    }
}
