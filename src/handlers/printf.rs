use crate::parse::Token;
use crate::registry;
use crate::verdict::Verdict;

const SPEC_BYTES: &[u8] = b"-+ #0'I.123456789*$hlLqjzt";

pub fn check_printf(tokens: &[Token]) -> Verdict {
    if format_of(tokens).is_some_and(assigns_a_variable) {
        return Verdict::Denied;
    }
    registry::try_fallback_grammar("printf", tokens).unwrap_or(Verdict::Denied)
}

fn format_of(tokens: &[Token]) -> Option<&str> {
    let mut rest = tokens.iter().skip(1).map(Token::as_str);
    while let Some(word) = rest.next() {
        if word == "--" {
            return rest.next();
        }
        if word == "-" || !word.starts_with('-') {
            return Some(word);
        }
    }
    None
}

fn assigns_a_variable(format: &str) -> bool {
    let bytes = format.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] != b'%' {
            at += 1;
            continue;
        }
        at += 1;
        while at < bytes.len() && SPEC_BYTES.contains(&bytes[at]) {
            at += 1;
        }
        match bytes.get(at) {
            Some(b'n') => return true,
            Some(b'(') => at = past_a_time_conversion(bytes, at),
            Some(_) => at += 1,
            None => {}
        }
    }
    false
}

fn past_a_time_conversion(bytes: &[u8], open: usize) -> usize {
    let close = bytes[open..].iter().position(|&b| b == b')').map(|offset| open + offset);
    match close {
        Some(close) if bytes.get(close + 1) == Some(&b'T') => close + 2,
        _ => open + 1,
    }
}

#[cfg(test)]
mod tests {
    use super::assigns_a_variable;
    use crate::is_safe_command;
    use proptest::prelude::*;

    #[test]
    fn a_conversion_that_stores_a_count_is_found() {
        for format in ["%n", "abc%n", "%5n", "%-5n", "%ln", "%hhn", "%1$n", "%*n", "%.3n", "x%sy%nz", "%%%n"] {
            assert!(assigns_a_variable(format), "{format}");
        }
    }

    #[test]
    fn text_and_other_conversions_are_not() {
        for format in ["", "%", "%%n", "n", "%s\\n", "%d%%", "50%", "%(%n)T", "%b", "%q", "\\x25n"] {
            assert!(!assigns_a_variable(format), "{format}");
        }
    }

    #[test]
    fn a_parenthesis_that_does_not_close_a_time_conversion_hides_nothing() {
        for format in ["%(abc%n", "%(%n", "%(%n)", "%(%n)y", "%5(%n)x", "%(x)T%(%n)"] {
            assert!(assigns_a_variable(format), "{format}");
        }
    }

    #[test]
    fn a_percent_word_that_reads_as_a_conversion_is_refused() {
        assert!(assigns_a_variable("50%normal"));
    }

    #[test]
    fn the_format_is_the_first_word_or_the_one_after_a_double_dash() {
        for (command, safe) in [
            ("printf hi", true),
            ("printf '%s\\n' hi", true),
            ("printf -- '%s' hi", true),
            ("printf '%s' '%n'", true),
            ("printf -- '%s' '%n'", true),
            ("printf '%n' x", false),
            ("printf -- '%n' x", false),
            ("printf -h '%n' x", false),
            ("printf 'x%5n' count", false),
            ("printf '%n' 'a[$(touch x)]'", false),
            ("gprintf '%n' x", false),
        ] {
            assert_eq!(is_safe_command(command), safe, "{command}");
        }
    }

    proptest! {
        #[test]
        fn any_spelling_of_the_store_conversion_is_refused(
            before in "[a-z0-9 ]{0,8}",
            spec in "[-+ #0'I.1-9*hlLqjzt]{0,6}",
            position in prop::option::of("[1-9]\\$"),
            after in "[a-z0-9 ]{0,8}",
        ) {
            let format = format!("{before}%{}{spec}n{after}", position.unwrap_or_default());
            prop_assert!(assigns_a_variable(&format), "{format}");
        }

        #[test]
        fn a_format_without_a_percent_is_never_refused(format in "[^%]{0,24}") {
            prop_assert!(!assigns_a_variable(&format));
        }

        #[test]
        fn an_escaped_percent_never_starts_a_conversion(before in "[a-z ]{0,8}", after in "[a-z ]{0,8}") {
            let format = format!("{before}%%n{after}");
            prop_assert!(!assigns_a_variable(&format), "{format}");
        }

        #[test]
        fn a_store_inside_parentheses_is_refused_unless_they_close_a_time_conversion(
            before in "[a-z0-9 ]{0,6}",
            after in "[a-z0-9 ]{0,6}",
            closing in prop::option::of("\\)[^T]?"),
        ) {
            let format = format!("%({before}%n{after}{}", closing.unwrap_or_default());
            prop_assert!(assigns_a_variable(&format), "{format}");
        }

        #[test]
        fn no_format_makes_the_scan_panic(format in "\\PC{0,40}") {
            let _ = assigns_a_variable(&format);
        }
    }
}
