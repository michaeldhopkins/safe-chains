use std::collections::HashMap;

use super::Role;
use crate::parse::Token;
use crate::policy::consumes_next_value;

/// If `tokens[i]` is one of `spec`'s mapped flags in any form — `-o V`, `--output=V`, glued
/// `-oV`, or clustered `-qO/etc/x` — return its (role, value, tokens-consumed). The value is
/// `None` when the flag stands alone: a flag the command also lists as standalone takes a value
/// only glued or after `=`, and the next word is never a value when it spells a flag, as the
/// policy walker reads it.
pub(super) fn match_flag<'a>(
    flags: &HashMap<String, Role>,
    tokens: &'a [Token],
    i: usize,
    standalone: &dyn Fn(&str) -> bool,
) -> Option<(Role, Option<&'a str>, usize)> {
    let t = tokens[i].as_str();
    for (flag, &role) in flags {
        if t == flag {
            return Some(separate_value(role, flag, tokens, i, standalone));
        }
        // A glued `flag=value`. Handles BOTH `--flag=v` (GNU) and single-dash-long `-flag=v`
        // (the Go-flag convention — terraform's `-out=…`/`-state-out=…`, which otherwise sailed
        // past this gate). The `=` must follow the EXACT flag name, so a short flag like `-o`
        // can't spuriously match `-output=…` — only its own `-o=…`.
        if let Some(v) = t.strip_prefix(flag.as_str()).and_then(|r| r.strip_prefix('=')) {
            return Some((role, Some(v), 1));
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
        .filter_map(|(flag, &role)| cluster.find(&flag[1..]).map(|p| (p, flag, role)))
        .min_by_key(|&(p, _, _)| p)
        .map(|(p, flag, role)| match &cluster[p + 1..] {
            "" => separate_value(role, flag, tokens, i, standalone),
            glued => (role, Some(glued), 1),
        })
}

fn separate_value<'a>(
    role: Role,
    flag: &str,
    tokens: &'a [Token],
    i: usize,
    standalone: &dyn Fn(&str) -> bool,
) -> (Role, Option<&'a str>, usize) {
    let Some(next) = tokens.get(i + 1) else {
        return (role, Some(""), 2);
    };
    if (role == Role::Ignore && standalone(flag)) || !consumes_next_value(Some(next)) {
        return (role, None, 1);
    }
    (role, Some(next.as_str()), 2)
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
            let valued = |_: &str| false;
            prop_assert_eq!(match_flag(&flags, &separate, 1, &valued), Some((Role::Write, Some(value.as_str()), 2)));
            prop_assert_eq!(match_flag(&flags, &equals, 1, &valued), Some((Role::Write, Some(value.as_str()), 1)));
            prop_assert_eq!(match_flag(&flags, &clustered, 1, &valued), Some((Role::Write, Some(value.as_str()), 2)));
            prop_assert_eq!(match_flag(&flags, &glued, 1, &valued).map(|(r, _, _)| r), Some(Role::Write));
        }

        #[test]
        fn a_flag_that_may_stand_alone_and_gates_nothing_takes_only_a_glued_value(
            letter in "[a-z]",
            booleans in "[A-Z]{0,3}",
            value in "[a-z0-9./~][a-z0-9./~-]{0,8}",
        ) {
            let short = format!("-{letter}");
            let long = format!("--{letter}{letter}");
            let flags = HashMap::from([(short.clone(), Role::Ignore), (long.clone(), Role::Ignore)]);
            let standalone = |f: &str| f == short || f == long;
            for flag in [&short, &long] {
                let separate = toks(&["cmd", flag, &value]);
                let equals = toks(&["cmd", &format!("{flag}={value}")]);
                prop_assert_eq!(match_flag(&flags, &separate, 1, &standalone), Some((Role::Ignore, None, 1)));
                prop_assert_eq!(match_flag(&flags, &equals, 1, &standalone), Some((Role::Ignore, Some(value.as_str()), 1)));
            }
            let clustered = toks(&["cmd", &format!("-{booleans}{letter}"), &value]);
            prop_assert_eq!(match_flag(&flags, &clustered, 1, &standalone), Some((Role::Ignore, None, 1)));
            let glued = toks(&["cmd", &format!("{short}{value}")]);
            prop_assert_eq!(match_flag(&flags, &glued, 1, &standalone).map(|(r, _, _)| r), Some(Role::Ignore));
            let judged = HashMap::from([(short.clone(), Role::Write)]);
            let separate = toks(&["cmd", &short, &value]);
            prop_assert_eq!(match_flag(&judged, &separate, 1, &standalone), Some((Role::Write, Some(value.as_str()), 2)));
        }

        #[test]
        fn a_word_spelling_a_flag_is_never_the_value_of_the_flag_before_it(
            letter in "[a-z]",
            booleans in "[A-Z]{0,3}",
            next in "--?[a-zA-Z][a-zA-Z0-9-]{0,6}",
        ) {
            let short = format!("-{letter}");
            let flags = HashMap::from([(short.clone(), Role::Write)]);
            let valued = |_: &str| false;
            let separate = toks(&["cmd", &short, &next]);
            let clustered = toks(&["cmd", &format!("-{booleans}{letter}"), &next]);
            prop_assert_eq!(match_flag(&flags, &separate, 1, &valued), Some((Role::Write, None, 1)));
            prop_assert_eq!(match_flag(&flags, &clustered, 1, &valued), Some((Role::Write, None, 1)));
        }
    }
}
