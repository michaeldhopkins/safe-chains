use crate::parse::Token;
use crate::policy::{FlagSet, check_flags};

use super::types::OwnedPolicy;

pub(super) fn check_owned(tokens: &[Token], policy: &OwnedPolicy) -> bool {
    check_flags(
        tokens,
        policy.standalone.as_slice(),
        policy.valued.as_slice(),
        policy.bare,
        policy.max_positional,
        policy.tolerance,
    )
}

/// Returns the first non-flag token after `tokens[0]`, treating the
/// policy's `valued` flags as consuming the next argument and its
/// `standalone` flags as zero-arg. `--` ends flag scanning. `-` is a
/// positional (the conventional stdin marker). Used by fallback
/// grammars that want to apply a `PositionalShape` predicate to the
/// first positional without re-implementing flag walking.
pub(super) fn first_positional<'a>(
    tokens: &'a [Token],
    policy: &OwnedPolicy,
) -> Option<&'a str> {
    let standalone = policy.standalone.as_slice();
    let valued = policy.valued.as_slice();
    let mut i = 1;
    while i < tokens.len() {
        let t = &tokens[i];
        let s = t.as_str();
        if s == "--" {
            return tokens.get(i + 1).map(Token::as_str);
        }
        if s == "-" || !s.starts_with('-') {
            return Some(s);
        }
        if standalone.contains_flag(s) {
            i += 1;
            continue;
        }
        if valued.contains_flag(s) {
            i += 2;
            continue;
        }
        if let Some((flag, _)) = s.split_once('=')
            && valued.contains_flag(flag)
        {
            i += 1;
            continue;
        }
        // Unknown flag: bail out — it's the policy validator's job to
        // accept or reject. Either way, we've reached the end of the
        // recognized flag run without finding a positional.
        return None;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::Token;
    use crate::policy::FlagTolerance;

    fn toks(words: &[&str]) -> Vec<Token> {
        words.iter().map(|s| Token::from_test(s)).collect()
    }

    fn policy(standalone: &[&str], valued: &[&str], max_positional: Option<usize>) -> OwnedPolicy {
        OwnedPolicy {
            standalone: standalone.iter().map(|s| s.to_string()).collect(),
            valued: valued.iter().map(|s| s.to_string()).collect(),
            bare: true,
            max_positional,
            tolerance: FlagTolerance::strict(),
        }
    }

    #[test]
    fn first_positional_finds_bare_arg() {
        let p = policy(&[], &[], None);
        let tokens = toks(&["cmd", "arg"]);
        assert_eq!(first_positional(&tokens, &p), Some("arg"));
    }

    #[test]
    fn first_positional_skips_standalone_flags() {
        let p = policy(&["--verbose"], &[], None);
        let tokens = toks(&["cmd", "--verbose", "file"]);
        assert_eq!(first_positional(&tokens, &p), Some("file"));
    }

    #[test]
    fn first_positional_skips_valued_flag_value() {
        let p = policy(&[], &["--type"], None);
        let tokens = toks(&["cmd", "--type", "erb", "file.erb"]);
        assert_eq!(first_positional(&tokens, &p), Some("file.erb"));
    }

    #[test]
    fn first_positional_skips_valued_flag_eq_form() {
        let p = policy(&[], &["--type"], None);
        let tokens = toks(&["cmd", "--type=erb", "file.erb"]);
        assert_eq!(first_positional(&tokens, &p), Some("file.erb"));
    }

    #[test]
    fn first_positional_treats_double_dash_as_terminator() {
        let p = policy(&["--verbose"], &[], None);
        let tokens = toks(&["cmd", "--", "--verbose"]);
        // After --, the very next token (even if flag-shaped) is positional.
        assert_eq!(first_positional(&tokens, &p), Some("--verbose"));
    }

    #[test]
    fn first_positional_treats_lone_dash_as_positional() {
        let p = policy(&[], &[], None);
        let tokens = toks(&["cmd", "-"]);
        assert_eq!(first_positional(&tokens, &p), Some("-"));
    }

    #[test]
    fn first_positional_returns_none_for_no_args() {
        let p = policy(&[], &[], None);
        let tokens = toks(&["cmd"]);
        assert_eq!(first_positional(&tokens, &p), None);
    }

    #[test]
    fn first_positional_returns_none_for_only_flags() {
        let p = policy(&["--help"], &[], None);
        let tokens = toks(&["cmd", "--help"]);
        assert_eq!(first_positional(&tokens, &p), None);
    }

    #[test]
    fn first_positional_returns_none_on_unknown_flag() {
        // Bails out at the unknown flag rather than guessing — the policy
        // validator is responsible for rejecting or accepting it.
        let p = policy(&[], &[], None);
        let tokens = toks(&["cmd", "--unknown", "file"]);
        assert_eq!(first_positional(&tokens, &p), None);
    }
}
