use crate::parse::{Token, WordSet};

/// Whether unrecognized flag-shaped tokens are denied or silently accepted
/// as positional arguments. The default (Strict) makes the allowlist
/// authoritative — any unrecognized `-X` or `--foo` is denied.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UnknownTolerance {
    /// Deny every unrecognized flag-shaped token. The safe default.
    #[default]
    Strict,
    /// Accept unknown single-dash tokens (`-X`, `-help`, `-mayDie`) as
    /// positional. Reject unknown double-dash. Use for tools like
    /// `pdftotext` that have single-dash long flags.
    Short,
    /// Accept unknown double-dash tokens (`--foo`, `--foo=value`) as
    /// positional. Reject unknown single-dash. Dangerous: most modern
    /// destructive flags are double-dash, so enabling this can silently
    /// accept mutating options. Reserved for tools with genuinely
    /// unbounded long-flag surfaces (AWS CLI service flags).
    Long,
    /// Accept both single-dash and double-dash unknowns as positional.
    /// Most permissive; combines the cost of `Short` and `Long`.
    Both,
}

impl UnknownTolerance {
    pub const fn allows_short(self) -> bool {
        matches!(self, Self::Short | Self::Both)
    }
    pub const fn allows_long(self) -> bool {
        matches!(self, Self::Long | Self::Both)
    }
}

/// How the dispatcher treats tokens that look like flags but aren't in the
/// allowlist. `unknown` controls flag-shaped unknowns; `numeric_dash` opts
/// into `-NUMBER` shorthand (e.g. `head -20`).
#[derive(Clone, Copy, Debug, Default)]
pub struct FlagTolerance {
    pub unknown: UnknownTolerance,
    pub numeric_dash: bool,
}

impl FlagTolerance {
    /// Strict allowlist: deny every unrecognized flag-shaped token.
    /// `const`-callable for use in static `FlagPolicy` literals.
    pub const fn strict() -> Self {
        Self { unknown: UnknownTolerance::Strict, numeric_dash: false }
    }
}

/// Predicate over the first positional token of a fallback grammar.
/// Lets a TOML-declared fallback say "the first positional must look
/// like a path" without the handler hardcoding the test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PositionalShape {
    /// Looks like a file path: contains `/`, contains `.`, or is `-`
    /// (the conventional stdin marker). Rejects flag-shaped tokens.
    Path,
}

impl PositionalShape {
    pub fn matches(self, token: &str) -> bool {
        match self {
            Self::Path => looks_like_path(token),
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "path" => Some(Self::Path),
            _ => None,
        }
    }
}

/// Heuristic for "this token looks like a file path." Used by the
/// `path` `PositionalShape`. Conservative on purpose — a bare word
/// like `Tiltfile` is a valid filename in cwd but the heuristic
/// rejects it to avoid swallowing flag-less subcommands. Callers
/// that want bare-name acceptance should match a sub block instead.
pub fn looks_like_path(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    if token.starts_with('-') {
        return token == "-";
    }
    token.contains('/') || token.contains('.')
}

pub trait FlagSet {
    fn contains_flag(&self, token: &str) -> bool;
    fn contains_short(&self, byte: u8) -> bool;
}

impl FlagSet for WordSet {
    fn contains_flag(&self, token: &str) -> bool {
        self.contains(token)
    }
    fn contains_short(&self, byte: u8) -> bool {
        self.contains_short(byte)
    }
}

impl FlagSet for [String] {
    fn contains_flag(&self, token: &str) -> bool {
        self.iter().any(|f| f.as_str() == token)
    }
    fn contains_short(&self, byte: u8) -> bool {
        self.iter().any(|f| f.len() == 2 && f.as_bytes()[1] == byte)
    }
}

impl FlagSet for Vec<String> {
    fn contains_flag(&self, token: &str) -> bool {
        self.as_slice().contains_flag(token)
    }
    fn contains_short(&self, byte: u8) -> bool {
        self.as_slice().contains_short(byte)
    }
}

pub struct FlagPolicy {
    pub standalone: WordSet,
    pub valued: WordSet,
    pub bare: bool,
    pub max_positional: Option<usize>,
    pub tolerance: FlagTolerance,
}

impl FlagPolicy {
    pub fn describe(&self) -> String {
        use crate::docs::wordset_items;
        let mut lines = Vec::new();
        let standalone = wordset_items(&self.standalone);
        if !standalone.is_empty() {
            lines.push(format!("- Allowed standalone flags: {standalone}"));
        }
        let valued = wordset_items(&self.valued);
        if !valued.is_empty() {
            lines.push(format!("- Allowed valued flags: {valued}"));
        }
        if self.bare {
            lines.push("- Bare invocation allowed".to_string());
        }
        if self.tolerance.unknown != UnknownTolerance::Strict {
            lines.push("- Hyphen-prefixed positional arguments accepted".to_string());
        }
        if self.tolerance.numeric_dash {
            lines.push("- Numeric shorthand accepted (e.g. -20 for -n 20)".to_string());
        }
        if lines.is_empty() && !self.bare {
            return "- Positional arguments only".to_string();
        }
        lines.join("\n")
    }

}

pub fn check(tokens: &[Token], policy: &FlagPolicy) -> bool {
    check_flags(
        tokens,
        &policy.standalone,
        &policy.valued,
        policy.bare,
        policy.max_positional,
        policy.tolerance,
    )
}

pub fn check_flags<S: FlagSet + ?Sized, V: FlagSet + ?Sized>(
    tokens: &[Token],
    standalone: &S,
    valued: &V,
    bare: bool,
    max_positional: Option<usize>,
    tolerance: FlagTolerance,
) -> bool {
    if tokens.len() == 1 {
        return bare;
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

        if tolerance.numeric_dash && t.len() > 1 && t[1..].bytes().all(|b| b.is_ascii_digit()) {
            i += 1;
            continue;
        }

        if standalone.contains_flag(t) {
            i += 1;
            continue;
        }

        if valued.contains_flag(t) {
            i += 2;
            continue;
        }

        if let Some(flag) = t.as_str().split_once('=').map(|(f, _)| f) {
            if valued.contains_flag(flag) {
                i += 1;
                continue;
            }
            // `--foo=value` forms are governed by the long-flag tolerance.
            if tolerance.unknown.allows_long() {
                positionals += 1;
                i += 1;
                continue;
            }
            return false;
        }

        if t.starts_with("--") {
            if tolerance.unknown.allows_long() {
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
            if standalone.contains_short(b) {
                j += 1;
                continue;
            }
            if valued.contains_short(b) {
                if is_last {
                    i += 1;
                }
                break;
            }
            if tolerance.unknown.allows_short() {
                positionals += 1;
                break;
            }
            return false;
        }
        i += 1;
    }
    max_positional.is_none_or(|max| positionals <= max)
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_POLICY: FlagPolicy = FlagPolicy {
        standalone: WordSet::flags(&[
            "--color", "--count", "--help", "--recursive", "--version",
            "-H", "-c", "-i", "-l", "-n", "-o", "-r", "-s", "-v", "-w",
        ]),
        valued: WordSet::flags(&[
            "--after-context", "--before-context", "--max-count",
            "-A", "-B", "-m",
        ]),
        bare: false,
        max_positional: None,
        tolerance: FlagTolerance::strict(),
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
            standalone: WordSet::flags(&[]),
            valued: WordSet::flags(&[]),
            bare: true,
            max_positional: None,
            tolerance: FlagTolerance::strict(),
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
        standalone: WordSet::flags(&["--count", "-c", "-d", "-i", "-u"]),
        valued: WordSet::flags(&["--skip-fields", "-f", "-s"]),
        bare: true,
        max_positional: Some(1),
        tolerance: FlagTolerance::strict(),
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

    static BOTH_TOLERANCES_POLICY: FlagPolicy = FlagPolicy {
        standalone: WordSet::flags(&["-E", "-e", "-n"]),
        valued: WordSet::flags(&[]),
        bare: true,
        max_positional: None,
        tolerance: FlagTolerance { unknown: UnknownTolerance::Both, numeric_dash: false },
    };

    #[test]
    fn both_tolerances_accept_unknown_long() {
        assert!(check(&toks(&["echo", "--unknown", "hello"]), &BOTH_TOLERANCES_POLICY));
    }

    #[test]
    fn both_tolerances_accept_unknown_short() {
        assert!(check(&toks(&["echo", "-x", "hello"]), &BOTH_TOLERANCES_POLICY));
    }

    #[test]
    fn both_tolerances_accept_triple_dash() {
        assert!(check(&toks(&["echo", "---"]), &BOTH_TOLERANCES_POLICY));
    }

    #[test]
    fn both_tolerances_known_flags_still_work() {
        assert!(check(&toks(&["echo", "-n", "hello"]), &BOTH_TOLERANCES_POLICY));
    }

    #[test]
    fn both_tolerances_combo_known_short() {
        assert!(check(&toks(&["echo", "-ne", "hello"]), &BOTH_TOLERANCES_POLICY));
    }

    #[test]
    fn both_tolerances_combo_unknown_short_byte() {
        assert!(check(&toks(&["echo", "-nx", "hello"]), &BOTH_TOLERANCES_POLICY));
    }

    #[test]
    fn both_tolerances_unknown_eq_form() {
        assert!(check(&toks(&["echo", "--foo=bar"]), &BOTH_TOLERANCES_POLICY));
    }

    // ============ Narrow tolerance: short-only ============
    // tolerate_unknown_short = true accepts unknown single-dash tokens
    // (-X, -mayDie, -help) as positional, while leaving double-dash unknowns
    // strict. This is the safer setting because most modern destructive
    // flags are double-dash.

    static SHORT_ONLY_POLICY: FlagPolicy = FlagPolicy {
        standalone: WordSet::flags(&["--help"]),
        valued: WordSet::flags(&[]),
        bare: false,
        max_positional: None,
        tolerance: FlagTolerance { unknown: UnknownTolerance::Short, numeric_dash: false },
    };

    #[test]
    fn short_only_accepts_unknown_dash_letter() {
        assert!(check(&toks(&["sample", "-mayDie"]), &SHORT_ONLY_POLICY));
    }

    #[test]
    fn short_only_accepts_single_dash_long_word() {
        // pdftotext-style: `-help`, `-layout`, `-version` (single dash + word)
        assert!(check(&toks(&["pdftotext", "-layout"]), &SHORT_ONLY_POLICY));
    }

    #[test]
    fn short_only_denies_unknown_double_dash() {
        // The whole point of the narrow split: --evil-flag must not slip
        // through when only short-tolerance is on.
        assert!(!check(&toks(&["sample", "--evil-flag"]), &SHORT_ONLY_POLICY));
    }

    #[test]
    fn short_only_denies_unknown_eq_form() {
        assert!(!check(&toks(&["sample", "--evil=value"]), &SHORT_ONLY_POLICY));
    }

    #[test]
    fn short_only_known_long_flag_still_works() {
        assert!(check(&toks(&["sample", "--help"]), &SHORT_ONLY_POLICY));
    }

    // ============ Narrow tolerance: long-only ============
    // tolerate_unknown_long = true accepts unknown double-dash tokens as
    // positional. This is the dangerous form; reserved for tools like AWS
    // CLI whose long-flag surface is genuinely unbounded.

    static LONG_ONLY_POLICY: FlagPolicy = FlagPolicy {
        standalone: WordSet::flags(&["--help"]),
        valued: WordSet::flags(&[]),
        bare: false,
        max_positional: None,
        tolerance: FlagTolerance { unknown: UnknownTolerance::Long, numeric_dash: false },
    };

    #[test]
    fn long_only_accepts_unknown_double_dash() {
        assert!(check(&toks(&["aws", "--some-aws-flag"]), &LONG_ONLY_POLICY));
    }

    #[test]
    fn long_only_accepts_unknown_eq_form() {
        assert!(check(
            &toks(&["aws", "--filter=Name=tag,Values=foo"]),
            &LONG_ONLY_POLICY,
        ));
    }

    #[test]
    fn long_only_denies_unknown_short_dash() {
        assert!(!check(&toks(&["aws", "-x"]), &LONG_ONLY_POLICY));
    }

    // ============ Both tolerances false: strict ============

    static STRICT_POLICY: FlagPolicy = FlagPolicy {
        standalone: WordSet::flags(&["--help"]),
        valued: WordSet::flags(&[]),
        bare: false,
        max_positional: None,
        tolerance: FlagTolerance::strict(),
    };

    #[test]
    fn strict_denies_unknown_short() {
        assert!(!check(&toks(&["foo", "-evil"]), &STRICT_POLICY));
    }

    #[test]
    fn strict_denies_unknown_long() {
        assert!(!check(&toks(&["foo", "--evil"]), &STRICT_POLICY));
    }

    #[test]
    fn strict_known_flag_passes() {
        assert!(check(&toks(&["foo", "--help"]), &STRICT_POLICY));
    }

    #[test]
    fn both_tolerances_with_max_positional() {
        let policy = FlagPolicy {
            standalone: WordSet::flags(&["-n"]),
            valued: WordSet::flags(&[]),
            bare: true,
            max_positional: Some(2),
            tolerance: FlagTolerance { unknown: UnknownTolerance::Both, numeric_dash: false },
        };
        assert!(check(&toks(&["echo", "--unknown", "hello"]), &policy));
        assert!(!check(&toks(&["echo", "--a", "--b", "--c"]), &policy));
    }

    static NUMERIC_DASH_POLICY: FlagPolicy = FlagPolicy {
        standalone: WordSet::flags(&[
            "--help", "--quiet", "--verbose", "--version",
            "-V", "-h", "-q", "-v", "-z",
        ]),
        valued: WordSet::flags(&["--bytes", "--lines", "-c", "-n"]),
        bare: true,
        max_positional: None,
        tolerance: FlagTolerance { numeric_dash: true, ..FlagTolerance::strict() },
    };

    #[test]
    fn numeric_dash_single_digit() {
        assert!(check(&toks(&["head", "-5"]), &NUMERIC_DASH_POLICY));
    }

    #[test]
    fn numeric_dash_multi_digit() {
        assert!(check(&toks(&["head", "-20"]), &NUMERIC_DASH_POLICY));
    }

    #[test]
    fn numeric_dash_large_number() {
        assert!(check(&toks(&["head", "-1000"]), &NUMERIC_DASH_POLICY));
    }

    #[test]
    fn numeric_dash_with_file_arg() {
        assert!(check(&toks(&["head", "-20", "file.txt"]), &NUMERIC_DASH_POLICY));
    }

    #[test]
    fn numeric_dash_with_other_flags() {
        assert!(check(&toks(&["head", "-q", "-20", "file.txt"]), &NUMERIC_DASH_POLICY));
    }

    #[test]
    fn numeric_dash_zero() {
        assert!(check(&toks(&["head", "-0"]), &NUMERIC_DASH_POLICY));
    }

    #[test]
    fn numeric_dash_still_rejects_unknown_flags() {
        assert!(!check(&toks(&["head", "-x"]), &NUMERIC_DASH_POLICY));
    }

    #[test]
    fn numeric_dash_rejects_mixed_alpha_num() {
        assert!(!check(&toks(&["head", "-20x"]), &NUMERIC_DASH_POLICY));
    }

    #[test]
    fn numeric_dash_disabled_rejects_multi_digit() {
        assert!(!check(&toks(&["grep", "-20", "pattern"]), &TEST_POLICY));
    }

    #[test]
    fn looks_like_path_accepts_relative() {
        assert!(looks_like_path("./Tiltfile"));
        assert!(looks_like_path("path/to/file"));
    }

    #[test]
    fn looks_like_path_accepts_dotted() {
        assert!(looks_like_path("Tiltfile.dev"));
        assert!(looks_like_path("file.rb"));
    }

    #[test]
    fn looks_like_path_accepts_stdin_dash() {
        assert!(looks_like_path("-"));
    }

    #[test]
    fn looks_like_path_rejects_flag() {
        assert!(!looks_like_path("--help"));
        assert!(!looks_like_path("-x"));
    }

    #[test]
    fn looks_like_path_rejects_bare_word() {
        assert!(!looks_like_path("Tiltfile"));
        assert!(!looks_like_path("up"));
    }

    #[test]
    fn looks_like_path_rejects_empty() {
        assert!(!looks_like_path(""));
    }

    #[test]
    fn positional_shape_path_matches() {
        assert!(PositionalShape::Path.matches("./file.rb"));
        assert!(!PositionalShape::Path.matches("--flag"));
    }

    #[test]
    fn positional_shape_from_name() {
        assert_eq!(PositionalShape::from_name("path"), Some(PositionalShape::Path));
        assert_eq!(PositionalShape::from_name("nope"), None);
    }
}
