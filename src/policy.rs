use crate::parse::{Token, WordSet};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FlagStyle {
    Strict,
    Positional,
}

pub struct FlagPolicy {
    pub standalone: WordSet,
    pub standalone_short: &'static [u8],
    pub valued: WordSet,
    pub valued_short: &'static [u8],
    pub bare: bool,
    pub max_positional: Option<usize>,
    pub flag_style: FlagStyle,
}

impl FlagPolicy {
    pub fn describe(&self) -> String {
        use crate::docs::{wordset_items, DocBuilder};
        let mut builder = DocBuilder::new();
        let standalone = wordset_items(&self.standalone);
        if !standalone.is_empty() {
            builder = builder.section(format!("Allowed standalone flags: {standalone}."));
        }
        let valued = wordset_items(&self.valued);
        if !valued.is_empty() {
            builder = builder.section(format!("Allowed valued flags: {valued}."));
        }
        if self.bare {
            builder = builder.section("Bare invocation allowed.".to_string());
        }
        if self.flag_style == FlagStyle::Positional {
            builder = builder.section("Hyphen-prefixed positional arguments accepted.".to_string());
        }
        let result = builder.build();
        if result.is_empty() && !self.bare {
            return "Positional arguments only.".to_string();
        }
        result
    }

    pub fn flag_summary(&self) -> String {
        use crate::docs::wordset_items;
        let mut parts = Vec::new();
        let standalone = wordset_items(&self.standalone);
        if !standalone.is_empty() {
            parts.push(format!("Flags: {standalone}"));
        }
        let valued = wordset_items(&self.valued);
        if !valued.is_empty() {
            parts.push(format!("Valued: {valued}"));
        }
        if self.flag_style == FlagStyle::Positional {
            parts.push("Positional args accepted".to_string());
        }
        parts.join(". ")
    }
}

pub fn check(tokens: &[Token], policy: &FlagPolicy) -> bool {
    if tokens.len() == 1 {
        return policy.bare;
    }

    let mut i = 1;
    let mut positionals: usize = 0;
    while i < tokens.len() {
        let t = &tokens[i];

        if *t == "--" {
            positionals += tokens.len() - i - 1;
            break;
        }

        if !t.starts_with('-') {
            positionals += 1;
            i += 1;
            continue;
        }

        if policy.standalone.contains(t) {
            i += 1;
            continue;
        }

        if policy.valued.contains(t) {
            i += 2;
            continue;
        }

        if let Some(flag) = t.as_str().split_once('=').map(|(f, _)| f) {
            if policy.valued.contains(flag) {
                i += 1;
                continue;
            }
            if policy.flag_style == FlagStyle::Positional {
                positionals += 1;
                i += 1;
                continue;
            }
            return false;
        }

        if t.starts_with("--") {
            if policy.flag_style == FlagStyle::Positional {
                positionals += 1;
                i += 1;
                continue;
            }
            return false;
        }

        let bytes = t.as_bytes();
        let mut j = 1;
        while j < bytes.len() {
            let b = bytes[j];
            let is_last = j == bytes.len() - 1;
            if policy.standalone_short.contains(&b) {
                j += 1;
                continue;
            }
            if policy.valued_short.contains(&b) {
                if is_last {
                    i += 1;
                }
                break;
            }
            if policy.flag_style == FlagStyle::Positional {
                positionals += 1;
                break;
            }
            return false;
        }
        i += 1;
    }
    policy.max_positional.is_none_or(|max| positionals <= max)
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_POLICY: FlagPolicy = FlagPolicy {
        standalone: WordSet::new(&[
            "--color", "--count", "--help", "--recursive", "--version",
            "-c", "-r",
        ]),
        standalone_short: b"cHilnorsvw",
        valued: WordSet::new(&[
            "--after-context", "--before-context", "--max-count",
            "-A", "-B", "-m",
        ]),
        valued_short: b"ABm",
        bare: false,
        max_positional: None,
        flag_style: FlagStyle::Strict,
    };

    fn toks(words: &[&str]) -> Vec<Token> {
        words.iter().map(|s| Token::from_test(s)).collect()
    }

    #[test]
    fn bare_denied_when_bare_false() {
        assert!(!check(&toks(&["grep"]), &TEST_POLICY));
    }

    #[test]
    fn bare_allowed_when_bare_true() {
        let policy = FlagPolicy {
            standalone: WordSet::new(&[]),
            standalone_short: b"",
            valued: WordSet::new(&[]),
            valued_short: b"",
            bare: true,
            max_positional: None,
            flag_style: FlagStyle::Strict,
        };
        assert!(check(&toks(&["uname"]), &policy));
    }

    #[test]
    fn standalone_long_flag() {
        assert!(check(&toks(&["grep", "--recursive", "pattern", "."]), &TEST_POLICY));
    }

    #[test]
    fn standalone_short_flag() {
        assert!(check(&toks(&["grep", "-r", "pattern", "."]), &TEST_POLICY));
    }

    #[test]
    fn valued_long_flag_space() {
        assert!(check(&toks(&["grep", "--max-count", "5", "pattern"]), &TEST_POLICY));
    }

    #[test]
    fn valued_long_flag_eq() {
        assert!(check(&toks(&["grep", "--max-count=5", "pattern"]), &TEST_POLICY));
    }

    #[test]
    fn valued_short_flag_space() {
        assert!(check(&toks(&["grep", "-m", "5", "pattern"]), &TEST_POLICY));
    }

    #[test]
    fn combined_standalone_short() {
        assert!(check(&toks(&["grep", "-rn", "pattern", "."]), &TEST_POLICY));
    }

    #[test]
    fn combined_short_with_valued_last() {
        assert!(check(&toks(&["grep", "-rnm", "5", "pattern"]), &TEST_POLICY));
    }

    #[test]
    fn combined_short_valued_mid_consumes_rest() {
        assert!(check(&toks(&["grep", "-rmn", "pattern"]), &TEST_POLICY));
    }

    #[test]
    fn unknown_long_flag_denied() {
        assert!(!check(&toks(&["grep", "--exec", "cmd"]), &TEST_POLICY));
    }

    #[test]
    fn unknown_short_flag_denied() {
        assert!(!check(&toks(&["grep", "-z", "pattern"]), &TEST_POLICY));
    }

    #[test]
    fn unknown_combined_short_denied() {
        assert!(!check(&toks(&["grep", "-rz", "pattern"]), &TEST_POLICY));
    }

    #[test]
    fn unknown_long_eq_denied() {
        assert!(!check(&toks(&["grep", "--output=file.txt", "pattern"]), &TEST_POLICY));
    }

    #[test]
    fn double_dash_stops_checking() {
        assert!(check(&toks(&["grep", "--", "--not-a-flag", "file"]), &TEST_POLICY));
    }

    #[test]
    fn positional_args_allowed() {
        assert!(check(&toks(&["grep", "pattern", "file.txt", "other.txt"]), &TEST_POLICY));
    }

    #[test]
    fn mixed_flags_and_positional() {
        assert!(check(
            &toks(&["grep", "-rn", "--color", "--max-count", "10", "pattern", "."]),
            &TEST_POLICY,
        ));
    }

    #[test]
    fn valued_short_in_explicit_form() {
        assert!(check(&toks(&["grep", "-A", "3", "-B", "3", "pattern"]), &TEST_POLICY));
    }

    #[test]
    fn bare_dash_allowed_as_stdin() {
        assert!(check(&toks(&["grep", "pattern", "-"]), &TEST_POLICY));
    }

    #[test]
    fn valued_flag_at_end_without_value() {
        assert!(check(&toks(&["grep", "--max-count"]), &TEST_POLICY));
    }

    #[test]
    fn single_short_in_wordset_and_byte_array() {
        assert!(check(&toks(&["grep", "-c", "pattern"]), &TEST_POLICY));
    }

    static LIMITED_POLICY: FlagPolicy = FlagPolicy {
        standalone: WordSet::new(&["--count", "-c", "-d", "-i", "-u"]),
        standalone_short: b"cdiu",
        valued: WordSet::new(&["--skip-fields", "-f", "-s"]),
        valued_short: b"fs",
        bare: true,
        max_positional: Some(1),
        flag_style: FlagStyle::Strict,
    };

    #[test]
    fn max_positional_within_limit() {
        assert!(check(&toks(&["uniq", "input.txt"]), &LIMITED_POLICY));
    }

    #[test]
    fn max_positional_exceeded() {
        assert!(!check(&toks(&["uniq", "input.txt", "output.txt"]), &LIMITED_POLICY));
    }

    #[test]
    fn max_positional_with_flags_within_limit() {
        assert!(check(&toks(&["uniq", "-c", "-f", "3", "input.txt"]), &LIMITED_POLICY));
    }

    #[test]
    fn max_positional_with_flags_exceeded() {
        assert!(!check(&toks(&["uniq", "-c", "input.txt", "output.txt"]), &LIMITED_POLICY));
    }

    #[test]
    fn max_positional_after_double_dash() {
        assert!(!check(&toks(&["uniq", "--", "input.txt", "output.txt"]), &LIMITED_POLICY));
    }

    #[test]
    fn max_positional_bare_allowed() {
        assert!(check(&toks(&["uniq"]), &LIMITED_POLICY));
    }

    static POSITIONAL_POLICY: FlagPolicy = FlagPolicy {
        standalone: WordSet::new(&["-E", "-e", "-n"]),
        standalone_short: b"Een",
        valued: WordSet::new(&[]),
        valued_short: b"",
        bare: true,
        max_positional: None,
        flag_style: FlagStyle::Positional,
    };

    #[test]
    fn positional_style_unknown_long() {
        assert!(check(&toks(&["echo", "--unknown", "hello"]), &POSITIONAL_POLICY));
    }

    #[test]
    fn positional_style_unknown_short() {
        assert!(check(&toks(&["echo", "-x", "hello"]), &POSITIONAL_POLICY));
    }

    #[test]
    fn positional_style_dashes() {
        assert!(check(&toks(&["echo", "---"]), &POSITIONAL_POLICY));
    }

    #[test]
    fn positional_style_known_flags_still_work() {
        assert!(check(&toks(&["echo", "-n", "hello"]), &POSITIONAL_POLICY));
    }

    #[test]
    fn positional_style_combo_known() {
        assert!(check(&toks(&["echo", "-ne", "hello"]), &POSITIONAL_POLICY));
    }

    #[test]
    fn positional_style_combo_unknown_byte() {
        assert!(check(&toks(&["echo", "-nx", "hello"]), &POSITIONAL_POLICY));
    }

    #[test]
    fn positional_style_unknown_eq() {
        assert!(check(&toks(&["echo", "--foo=bar"]), &POSITIONAL_POLICY));
    }

    #[test]
    fn positional_style_with_max_positional() {
        let policy = FlagPolicy {
            standalone: WordSet::new(&["-n"]),
            standalone_short: b"n",
            valued: WordSet::new(&[]),
            valued_short: b"",
            bare: true,
            max_positional: Some(2),
            flag_style: FlagStyle::Positional,
        };
        assert!(check(&toks(&["echo", "--unknown", "hello"]), &policy));
        assert!(!check(&toks(&["echo", "--a", "--b", "--c"]), &policy));
    }
}
