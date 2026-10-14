use crate::parse::Token;

const PATTERN_LONG_FLAGS: &[&str] = &["regexp", "regex", "file", "match"];

pub(crate) fn supplies_pattern(tokens: &[Token]) -> bool {
    tokens.iter().skip(1).any(|t| is_pattern_flag(t.as_str()))
}

fn is_pattern_flag(token: &str) -> bool {
    if let Some(long) = token.strip_prefix("--") {
        let name = long.split('=').next().unwrap_or(long);
        return !name.is_empty() && PATTERN_LONG_FLAGS.iter().any(|flag| flag.starts_with(name));
    }
    token
        .strip_prefix('-')
        .is_some_and(|cluster| cluster.bytes().take_while(u8::is_ascii_alphabetic).any(|b| b == b'e' || b == b'f'))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PatternWord {
    Pattern,
    File,
}

pub(crate) fn takes_next_word(token: &str) -> Option<PatternWord> {
    if let Some(name) = token.strip_prefix("--") {
        if name.is_empty() || name.contains('=') {
            return None;
        }
        if "file".starts_with(name) {
            return Some(PatternWord::File);
        }
        return ["regexp", "regex", "match"].iter().any(|f| f.starts_with(name)).then_some(PatternWord::Pattern);
    }
    let cluster = token.strip_prefix('-')?;
    let at = cluster.bytes().take_while(u8::is_ascii_alphabetic).position(|b| b == b'e' || b == b'f')?;
    if at + 1 != cluster.len() {
        return None;
    }
    Some(if cluster.as_bytes()[at] == b'f' { PatternWord::File } else { PatternWord::Pattern })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PatternStep {
    Next(usize),
    Deny,
}

pub(crate) fn step_over(tokens: &[Token], i: usize, read_denied: impl Fn(&str) -> bool) -> Option<PatternStep> {
    let word = takes_next_word(tokens.get(i)?.as_str())?;
    if word == PatternWord::File && tokens.get(i + 1).is_some_and(|t| read_denied(t.as_str())) {
        return Some(PatternStep::Deny);
    }
    Some(PatternStep::Next(i + 2))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{PatternStep, PatternWord, is_pattern_flag, step_over, supplies_pattern, takes_next_word};
    use crate::parse::Token;

    fn toks(words: &[&str]) -> Vec<Token> {
        words.iter().map(|w| Token::from_raw((*w).to_string())).collect()
    }

    #[test]
    fn the_pattern_flags_are_recognised_in_every_spelling() {
        for flag in ["-e", "-ex", "-ie", "-f", "-fpats", "-if", "--regexp", "--regexp=x", "--reg=x", "--file=p", "--regex=x", "--match"] {
            assert!(is_pattern_flag(flag), "{flag}");
        }
        for flag in ["-i", "-n", "-r", "-A3", "-E", "-F", "--ignore-case", "--", "-", "--color=always", "--context=3"] {
            assert!(!is_pattern_flag(flag), "{flag}");
        }
    }

    #[test]
    fn a_pattern_flag_spelled_alone_takes_the_next_word() {
        for flag in ["-e", "-ie", "-wie", "--regexp", "--regex", "--reg", "--match", "--m"] {
            assert_eq!(takes_next_word(flag), Some(PatternWord::Pattern), "{flag}");
        }
        for flag in ["-f", "-if", "--file", "--fi"] {
            assert_eq!(takes_next_word(flag), Some(PatternWord::File), "{flag}");
        }
        for flag in ["-ex", "-fpats", "-ei", "--regexp=x", "--file=p", "-i", "-A3", "--", "-", "--ignore-case", "x"] {
            assert_eq!(takes_next_word(flag), None, "{flag}");
        }
    }

    #[test]
    fn only_a_pattern_file_is_judged_as_a_read() {
        let secret = |p: &str| p.contains(".ssh");
        let tokens = toks(&["rg", "-e", "~/.ssh/id_rsa", "-f", "~/.ssh/id_rsa", "-f", "pats", "-i", "-e"]);
        assert_eq!(step_over(&tokens, 1, secret), Some(PatternStep::Next(3)));
        assert_eq!(step_over(&tokens, 3, secret), Some(PatternStep::Deny));
        assert_eq!(step_over(&tokens, 5, secret), Some(PatternStep::Next(7)));
        assert_eq!(step_over(&tokens, 7, secret), None);
        assert_eq!(step_over(&tokens, 8, secret), Some(PatternStep::Next(10)));
        assert_eq!(step_over(&tokens, 10, secret), None);
    }

    #[test]
    fn the_command_name_is_not_read_as_a_flag() {
        assert!(!supplies_pattern(&toks(&["-e", "x", "file"])));
        assert!(supplies_pattern(&toks(&["rg", "x", "file", "-e", "y"])));
    }

    proptest! {
        #[test]
        fn a_cluster_carrying_e_or_f_supplies_a_pattern(
            before in "[a-dg-zA-Z]{0,3}",
            letter in prop::sample::select(vec!['e', 'f']),
            glued in "[a-z0-9./~]{0,8}",
        ) {
            let flag = format!("-{before}{letter}{glued}");
            prop_assert!(is_pattern_flag(&flag), "{flag}");
        }

        #[test]
        fn every_prefix_of_a_pattern_long_flag_supplies_a_pattern(
            flag in prop::sample::select(vec!["regexp", "regex", "file", "match"]),
            cut in 1usize..7,
            value in proptest::option::of("[a-z/~.]{0,6}"),
        ) {
            let name = &flag[..cut.min(flag.len())];
            let token = match &value {
                Some(v) => format!("--{name}={v}"),
                None => format!("--{name}"),
            };
            prop_assert!(is_pattern_flag(&token), "{token}");
        }

        #[test]
        fn a_bare_word_never_supplies_a_pattern(words in proptest::collection::vec("[a-z0-9./~][a-z0-9./~-]{0,8}", 0..6)) {
            let mut all = vec!["rg".to_string()];
            all.extend(words);
            let tokens: Vec<Token> = all.into_iter().map(Token::from_raw).collect();
            prop_assert!(!supplies_pattern(&tokens));
        }
    }
}
