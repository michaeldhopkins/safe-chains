use crate::parse::Token;
use crate::policy::{FlagSet, FlagTolerance, UnknownTolerance, check_flags, consumes_next_value, count_positionals};

#[derive(Debug, Clone)]
pub struct OwnedPolicy {
    pub standalone: Vec<String>,
    pub valued: Vec<String>,
    pub bare: bool,
    pub max_positional: Option<usize>,
    pub min_positional: Option<usize>,
    pub tolerance: FlagTolerance,
}

impl OwnedPolicy {
    pub(super) fn allow_all() -> Self {
        OwnedPolicy {
            standalone: Vec::new(),
            valued: Vec::new(),
            bare: true,
            max_positional: None,
            min_positional: None,
            tolerance: FlagTolerance { unknown: UnknownTolerance::Both, numeric_dash: false, single_dash_words: false },
        }
    }
}

pub(super) fn check_owned(tokens: &[Token], policy: &OwnedPolicy) -> bool {
    let (standalone, valued) = (policy.standalone.as_slice(), policy.valued.as_slice());
    check_flags(tokens, standalone, valued, policy.bare, policy.max_positional, policy.tolerance)
        && policy
            .min_positional
            .is_none_or(|min| count_positionals(tokens, standalone, valued, policy.tolerance).is_some_and(|n| n >= min))
}

/// Whether one of `wanted` is given AS A FLAG: not as the value of the flag before it, not glued
/// into a short cluster after a flag that takes the rest as its value (`-rm` is `-r m`), and not
/// after `--`.
///
/// A required flag that only appears to be present is how a guarded sub runs its unguarded form:
/// `jj describe -rm` names revision `m` and opens the editor.
pub(super) fn flag_given(tokens: &[Token], policy: &OwnedPolicy, wanted: &[String]) -> bool {
    let (standalone, valued) = (policy.standalone.as_slice(), policy.valued.as_slice());
    let wants = |flag: &str| wanted.iter().any(|w| w == flag);
    let mut i = 1;
    while i < tokens.len() {
        let t = tokens[i].as_str();
        if t == "--" {
            return false;
        }
        i += 1;
        if wants(t) {
            return true;
        }
        if !t.starts_with('-') || t == "-" {
            continue;
        }
        if t.starts_with("--") {
            let name = t.split_once('=').map_or(t, |(name, _)| name);
            if wants(name) {
                return true;
            }
            if !t.contains('=') && valued.contains_flag(name) && consumes_next_value(tokens.get(i)) {
                i += 1;
            }
            continue;
        }
        if policy.tolerance.single_dash_words || valued.contains_flag(t) {
            if valued.contains_flag(t) && consumes_next_value(tokens.get(i)) {
                i += 1;
            }
            continue;
        }
        let bytes = t.as_bytes();
        for (j, &b) in bytes.iter().enumerate().skip(1) {
            if wanted.iter().any(|w| w.as_bytes() == [b'-', b]) {
                return true;
            }
            if valued.contains_short(b) {
                if j == bytes.len() - 1 && consumes_next_value(tokens.get(i)) {
                    i += 1;
                }
                break;
            }
            if !standalone.contains_short(b) {
                break;
            }
        }
    }
    false
}

/// Returns the first non-flag token after `tokens[0]`, treating the
/// policy's `valued` flags as consuming the next argument and its
/// `standalone` flags as zero-arg. `--` ends flag scanning. `-` is a
/// positional (the conventional stdin marker). Used by fallback
/// grammars that want to apply a `PositionalShape` predicate to the
/// first positional without re-implementing flag walking.
pub(super) fn min_positional_fits(parent: &str, toml: &super::types::TomlSub, policy_free: bool) -> Result<(), String> {
    let Some(min) = toml.min_positional else { return Ok(()) };
    if policy_free || !toml.sub.is_empty() || !toml.first_arg.is_empty() || !toml.credential_first_arg.is_empty() {
        return Err(format!("command '{parent}' sub `{}`: min_positional needs a flag policy to count operands against", toml.name));
    }
    match toml.max_positional {
        Some(max) if min > max => Err(format!("command '{parent}' sub `{}`: min_positional {min} exceeds max_positional {max}", toml.name)),
        _ => Ok(()),
    }
}

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
            min_positional: None,
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

#[cfg(test)]
mod flag_given_tests {
    use super::*;
    use proptest::prelude::*;

    fn jj_like(min_positional: Option<usize>) -> OwnedPolicy {
        OwnedPolicy {
            standalone: vec!["--quiet".into(), "-a".into(), "-k".into()],
            valued: vec!["--message".into(), "--revision".into(), "-m".into(), "-r".into()],
            bare: true,
            max_positional: None,
            min_positional,
            tolerance: FlagTolerance::strict(),
        }
    }

    fn toks(words: &[&str]) -> Vec<Token> {
        std::iter::once("cmd").chain(words.iter().copied()).map(Token::from_test).collect()
    }

    fn given(words: &[&str]) -> bool {
        flag_given(&toks(words), &jj_like(None), &["--message".to_string(), "-m".to_string()])
    }

    #[test]
    fn a_flag_is_given_in_each_of_its_spellings() {
        for words in [&["-m", "x"][..], &["-mx"], &["--message", "x"], &["--message=x"], &["-km", "x"], &["-r", "@", "-m", "x"]] {
            assert!(given(words), "{words:?}");
        }
    }

    #[test]
    fn a_flag_that_only_looks_given_is_not() {
        for words in [&[][..], &["-rm"], &["-r", "m"], &["--", "-m", "x"], &["--revision=-m"], &["--revision", "m"], &["m"]] {
            assert!(!given(words), "{words:?}");
        }
    }

    #[test]
    fn a_required_word_matches_whole() {
        let mut words = jj_like(None);
        words.tolerance.single_dash_words = true;
        let want = |w: &str| vec![w.to_string()];
        assert!(flag_given(&toks(&["-statistics"]), &words, &want("-statistics")));
        assert!(!flag_given(&toks(&["-statsonly"]), &words, &want("-s")));
        assert!(flag_given(&toks(&["-", "bash"]), &jj_like(None), &want("-")));
        assert!(!flag_given(&toks(&["--", "-"]), &jj_like(None), &want("-")));
    }

    #[test]
    fn min_positional_counts_operands_not_flag_values() {
        let p = jj_like(Some(1));
        assert!(!check_owned(&toks(&["-m", "x"]), &p));
        assert!(!check_owned(&toks(&["-m", "x", "-r", "src/a"]), &p));
        assert!(check_owned(&toks(&["-m", "x", "src/a"]), &p));
        assert!(check_owned(&toks(&["-m", "x", "--", "src/a"]), &p));
        assert!(check_owned(&toks(&["-m", "x"]), &jj_like(None)));
    }

    #[test]
    fn min_positional_is_named_in_the_reference() {
        assert!(jj_like(Some(1)).flag_summary().ends_with("At least 1 positional argument"));
        assert!(jj_like(Some(2)).flag_summary().ends_with("At least 2 positional arguments"));
        assert!(!jj_like(None).flag_summary().contains("positional argument"));
    }

    fn demo(sub_fields: &str) -> Result<Vec<crate::registry::CommandSpec>, String> {
        crate::registry::load_toml(&format!("[[command]]\nname = \"demo\"\n[[command.sub]]\nname = \"split\"\n{sub_fields}\n"), "test")
    }

    #[test]
    fn min_positional_dispatches_through_a_loaded_definition() {
        let specs = demo("level = \"SafeWrite\"\nrequire_any = [\"-m\"]\nmin_positional = 1\nvalued = [\"-m\", \"-r\"]").expect("loads");
        let verdict = |words: &[&str]| crate::registry::dispatch_spec(&toks(words), &specs[0]);
        assert_eq!(verdict(&["split", "-m", "x", "a"]), crate::verdict::Verdict::Allowed(crate::verdict::SafetyLevel::SafeWrite));
        assert_eq!(verdict(&["split", "-m", "x"]), crate::verdict::Verdict::Denied);
        assert_eq!(verdict(&["split", "-m", "x", "-r", "a"]), crate::verdict::Verdict::Denied);
        assert_eq!(verdict(&["split", "a"]), crate::verdict::Verdict::Denied);
    }

    #[test]
    fn min_positional_is_refused_where_nothing_would_count_operands() {
        for fields in [
            "min_positional = 1\nallow_all = true",
            "min_positional = 1\nfirst_arg = [\"x\"]",
            "min_positional = 1\nhandler = \"demo\"",
            "level = \"SafeWrite\"\nmin_positional = 2\nmax_positional = 1",
        ] {
            let err = demo(fields).err().unwrap_or_else(|| panic!("{fields:?} loaded"));
            assert!(err.contains("min_positional"), "{fields:?}: {err}");
        }
    }

    #[derive(Debug, Clone)]
    enum Unit {
        Standalone(&'static str),
        Valued(&'static str, &'static str),
        Glued(&'static str),
        Operand(&'static str),
        Wanted(Vec<&'static str>),
        End,
    }

    fn unit() -> impl Strategy<Value = Unit> {
        let value = prop::sample::select(vec!["m", "x", "mx", "@-"]);
        prop_oneof![
            prop::sample::select(vec!["-a", "-k", "-ak", "--quiet"]).prop_map(Unit::Standalone),
            (prop::sample::select(vec!["-r", "--revision"]), value.clone()).prop_map(|(f, v)| Unit::Valued(f, v)),
            prop::sample::select(vec!["-rm", "-rx", "-arm", "--revision=m", "--revision=-m"]).prop_map(Unit::Glued),
            value.prop_map(Unit::Operand),
            prop::sample::select(vec![vec!["-m", "x"], vec!["-mx"], vec!["-am", "x"], vec!["--message", "m"], vec!["--message=m"]])
                .prop_map(Unit::Wanted),
            Just(Unit::End),
        ]
    }

    proptest! {
        #[test]
        fn flag_given_matches_a_reading_unit_by_unit(units in prop::collection::vec(unit(), 0..8)) {
            let mut words = Vec::new();
            let mut expected = false;
            let mut ended = false;
            for u in &units {
                match u {
                    Unit::Standalone(f) | Unit::Glued(f) | Unit::Operand(f) => words.push(*f),
                    Unit::Valued(f, v) => words.extend([*f, *v]),
                    Unit::Wanted(w) => {
                        expected |= !ended;
                        words.extend(w.iter().copied());
                    }
                    Unit::End => {
                        ended = true;
                        words.push("--");
                    }
                }
            }
            prop_assert_eq!(given(&words), expected, "{:?}", words);
        }
    }
}
