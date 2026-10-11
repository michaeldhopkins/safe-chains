use crate::parse::Token;
use crate::policy::{FlagSet, check_flags};

use super::types::OwnedPolicy;

pub(super) fn check_owned(tokens: &[Token], policy: &OwnedPolicy) -> bool {
    check_flags(tokens, policy.standalone.as_slice(), policy.valued.as_slice(), policy.bare, policy.max_positional, policy.tolerance)
}

/// Returns the first non-flag token after `tokens[0]`, treating the
/// policy's `valued` flags as consuming the next argument and its
/// `standalone` flags as zero-arg. `--` ends flag scanning. `-` is a
/// positional (the conventional stdin marker). Used by fallback
/// grammars that want to apply a `PositionalShape` predicate to the
/// first positional without re-implementing flag walking.
pub(super) fn first_positional<'a>(tokens: &'a [Token], policy: &OwnedPolicy) -> Option<&'a str> {
    first_positional_at(tokens, policy).map(|(_, s)| s)
}

/// [`first_positional`], plus WHERE it was found.
///
/// The index is what lets a caller split an invocation at the executor: everything up to and
/// including that token is the command's own grammar and can be checked against its flag policy,
/// while everything after it is the SCRIPT's argv and cannot. `dispatch_executor` needs exactly
/// that split — see the comment there.
///
/// Expressed as one walk with `first_positional` delegating to it, rather than two functions that
/// scan independently: the value and its index must never disagree about which token is the
/// executor, and two copies of this loop is how they would.
pub(super) fn first_positional_at<'a>(tokens: &'a [Token], policy: &OwnedPolicy) -> Option<(usize, &'a str)> {
    let standalone = policy.standalone.as_slice();
    let valued = policy.valued.as_slice();
    let mut i = 1;
    while i < tokens.len() {
        let t = &tokens[i];
        let s = t.as_str();
        if s == "--" {
            return tokens.get(i + 1).map(|t| (i + 1, t.as_str()));
        }
        if s == "-" || !s.starts_with('-') {
            return Some((i, s));
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

    /// Property tests for `first_positional` / `first_positional_at` / `check_owned`.
    ///
    /// Each property pins an invariant of the function pair rather than an example. They share
    /// the in-module `policy`/`toks` builders and run under `proptest` so the synthesized tokens
    /// stay inside the alphabet those builders already understand.
    mod prop {
        use super::*;
        use proptest::prelude::*;

        /// A flag-shaped tail token (regex covers `-`, `--`, and `--name=value` shapes).
        const FLAG_RE: &str = "--?[a-zA-Z][a-zA-Z0-9._-]{0,7}(=[a-zA-Z0-9._-]{1,8})?";
        /// A never-flag-shaped token (no leading `-`, otherwise printable).
        const NON_FLAG_RE: &str = "[a-zA-Z][a-zA-Z0-9._]{0,7}";

        fn toks_owned(words: &[String]) -> Vec<Token> {
            words.iter().cloned().map(Token::from_raw).collect()
        }

        /// Reference implementation of `first_positional_at` for the case where BOTH flag lists
        /// are empty. In that mode the documented behavior reduces to: walk forward, return
        /// `(i + 1, next)` after a bare `--`, return `(i, s)` for `-` or any non-`-`-starting
        /// token, and return `None` on any other flag-shaped token. Used as the oracle.
        fn oracle_first_positional_at(tokens: &[Token]) -> Option<(usize, &str)> {
            // Skips index 0 and returns the same answer the documented walk would on an empty
            // grammar: look at one token, then stop. Encoded as `if let` rather than a loop
            // because every branch decides the result on the first token and a loop would loop
            // at most once.
            let (offset, t) = tokens.iter().enumerate().nth(1)?;
            let s = t.as_str();
            if s == "--" {
                let i = offset + 1;
                return tokens.get(i).map(|t| (i, t.as_str()));
            }
            if s == "-" || !s.starts_with('-') {
                return Some((offset, s));
            }
            None
        }

        proptest! {
            /// Consistency / roundtrip: `first_positional` delegates to `first_positional_at`,
            /// so the string it returns must equal the string half of the indexed pair for
            /// EVERY (tokens, policy) pair. A regression that splits the two walks apart —
            /// the very thing the docstring warns against — fails here.
            #[test]
            fn first_positional_and_first_positional_at_agree_on_the_returned_string(
                tail in proptest::collection::vec("(-|--|--?[a-zA-Z][a-zA-Z0-9._-]{0,7}(=[a-zA-Z0-9._-]{1,8})?|[a-zA-Z][a-zA-Z0-9._]{0,7})", 0..8),
                standalone in proptest::collection::vec(FLAG_RE, 0..4),
                valued in proptest::collection::vec(FLAG_RE, 0..4),
            ) {
                let mut words = vec!["cmd".to_string()];
                words.extend(tail);
                let tokens = toks_owned(&words);
                let p = policy(
                    &standalone.iter().map(String::as_str).collect::<Vec<_>>(),
                    &valued.iter().map(String::as_str).collect::<Vec<_>>(),
                    None,
                );
                let expected = first_positional_at(&tokens, &p).map(|(_, s)| s.to_string());
                let actual = first_positional(&tokens, &p).map(|s| s.to_string());
                proptest::prop_assert_eq!(actual, expected);
            }

            /// Index honesty: the index returned by `first_positional_at` must really point
            /// at the token whose text it pairs with. The walk stores `(i, s)` as one
            /// expression so they cannot disagree by construction, and this property pins
            /// that coupling against any future change that recomputes one half from a
            /// different loop variable.
            #[test]
            fn first_positional_at_index_points_at_the_returned_token(
                standalone in proptest::collection::vec(FLAG_RE, 0..3),
                valued in proptest::collection::vec(FLAG_RE, 0..3),
                tail in proptest::collection::vec(
                    "(--|--?[a-zA-Z][a-zA-Z0-9._-]{0,7}(=[a-zA-Z0-9._-]{1,8})?|[a-zA-Z][a-zA-Z0-9._]{0,7})",
                    1..6,
                ),
            ) {
                let mut words = vec!["cmd".to_string()];
                words.extend(tail);
                let tokens = toks_owned(&words);
                let p = policy(
                    &standalone.iter().map(String::as_str).collect::<Vec<_>>(),
                    &valued.iter().map(String::as_str).collect::<Vec<_>>(),
                    None,
                );
                if let Some((i, s)) = first_positional_at(&tokens, &p) {
                    proptest::prop_assert!(i < tokens.len(), "index {} past tokens.len()={}", i, tokens.len());
                    proptest::prop_assert_eq!(tokens[i].as_str(), s);
                }
            }

            /// Oracle / differential: with both `standalone` and `valued` empty the walk
            /// collapses to a four-line scan. This property states that the contracted
            /// implementation equals the inlined reference for EVERY generated shape, so
            /// a refactor that introduced extra logic for the empty-grammar case would be
            /// caught — the empty case is the one that has to match the docstring.
            #[test]
            fn first_positional_with_empty_grammars_matches_a_simple_scan(
                tail in proptest::collection::vec(
                    "(-|--|--?[a-zA-Z][a-zA-Z0-9._-]{0,7}(=[a-zA-Z0-9._-]{1,8})?|[a-zA-Z][a-zA-Z0-9._]{0,7})",
                    0..6,
                ),
            ) {
                let mut words = vec!["cmd".to_string()];
                words.extend(tail);
                let tokens = toks_owned(&words);
                let p = policy(&[], &[], None);
                let oracle = oracle_first_positional_at(&tokens).map(|(i, s)| (i, s.to_string()));
                let actual = first_positional_at(&tokens, &p).map(|(i, s)| (i, s.to_string()));
                proptest::prop_assert_eq!(actual, oracle);
            }

            /// Monotonicity: prepending a token that the policy declares as a STANDALONE flag
            /// does not change the first positional found later. The walk consumes a standalone
            /// in exactly one step (`i += 1`), so the rest of the scan is identical; the
            /// property locks that "additive, not destructive" behavior in. The prepended flag
            /// is taken from the SAME generated standalone list so we are asserting on real
            /// acceptance, not on unknown-flag bailouts.
            #[test]
            fn prepending_a_known_standalone_flag_does_not_change_the_first_positional(
                standalone in proptest::collection::vec(FLAG_RE, 1..4),
                tail in proptest::collection::vec(NON_FLAG_RE, 0..5),
            ) {
                let prefix_token = standalone[0].clone();
                let mut base = vec!["cmd".to_string()];
                base.extend(tail.clone());
                let base_tokens = toks_owned(&base);
                let p = policy(
                    &standalone.iter().map(String::as_str).collect::<Vec<_>>(),
                    &[],
                    None,
                );
                let before = first_positional(&base_tokens, &p).map(|s| s.to_string());

                let mut prefixed = vec!["cmd".to_string(), prefix_token];
                prefixed.extend(tail);
                let prefixed_tokens = toks_owned(&prefixed);
                let after = first_positional(&prefixed_tokens, &p).map(|s| s.to_string());

                proptest::prop_assert_eq!(after, before);
            }
        }
    }
}
