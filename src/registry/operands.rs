//! The `refuse_operands` gate of a sub: arguments whose text a sub never accepts.

use crate::parse::Token;
use crate::verdict::Verdict;

pub(super) fn glob_matches(pattern: &str, text: &str) -> bool {
    let (pattern, text) = (pattern.as_bytes(), text.as_bytes());
    let (mut p, mut t) = (0, 0);
    let mut resume: Option<(usize, usize)> = None;
    while t < text.len() {
        if p < pattern.len() && pattern[p] == b'*' {
            resume = Some((p, t));
            p += 1;
        } else if p < pattern.len() && pattern[p] == text[t] {
            p += 1;
            t += 1;
        } else if let Some((star, matched)) = resume {
            p = star + 1;
            t = matched + 1;
            resume = Some((star, matched + 1));
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|&b| b == b'*')
}

fn candidates(arg: &str) -> impl Iterator<Item = &str> {
    let is_flag = arg.starts_with('-') && arg != "-";
    let (name, value) = match arg.split_once('=') {
        Some((name, value)) => (Some(name), Some(value)),
        None => (None, None),
    };
    (!is_flag).then_some(arg).into_iter().chain((!is_flag).then_some(name).flatten()).chain(value)
}

pub(super) fn presents_refused_operand(tokens: &[Token], patterns: &[String]) -> bool {
    !patterns.is_empty()
        && tokens
            .iter()
            .skip(1)
            .flat_map(|token| candidates(token.as_str()))
            .any(|text| patterns.iter().any(|pattern| glob_matches(pattern, text)))
}

pub(super) fn unless_refused(tokens: &[Token], patterns: &[String], otherwise: impl FnOnce() -> Verdict) -> Verdict {
    if presents_refused_operand(tokens, patterns) { Verdict::Denied } else { otherwise() }
}

pub(super) fn check_declaration(parent: &str, name: &str, patterns: &[String], profiled: bool) -> Result<(), String> {
    if patterns.iter().any(|pattern| pattern.is_empty()) {
        return Err(format!(
            "command '{parent}' sub `{name}`: `refuse_operands` holds an empty glob, which would match only an empty argument"
        ));
    }
    if profiled && !patterns.is_empty() {
        return Err(format!(
            "command '{parent}' sub `{name}`: `refuse_operands` is not read on a profiled sub, whose engine classification never reaches the operand gate"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::types::DispatchKind;
    use super::*;
    use proptest::prelude::*;

    fn tokens(words: &[String]) -> Vec<Token> {
        words.iter().cloned().map(Token::from_raw).collect()
    }

    fn line(sub: &str, operands: &[String]) -> Vec<Token> {
        let mut words = vec![sub.to_string()];
        words.extend(operands.iter().cloned());
        tokens(&words)
    }

    #[test]
    fn a_star_matches_any_run_and_other_bytes_are_literal() {
        assert!(glob_matches("*{{*", "a{{b"));
        assert!(glob_matches("*{{*", "{{"));
        assert!(glob_matches("_.*", "_.source"));
        assert!(glob_matches("_", "_"));
        assert!(glob_matches("*", ""));
        assert!(!glob_matches("_", "_x"));
        assert!(!glob_matches("_.*", "_"));
        assert!(!glob_matches("*{{*", "{ {"));
        assert!(!glob_matches("a*b", "ac"));
    }

    #[test]
    fn the_name_and_the_value_of_an_assignment_are_each_tested() {
        let patterns = ["_".to_string(), "*{{*".to_string()];
        assert!(presents_refused_operand(&line("set", &["_=x".into()]), &patterns));
        assert!(presents_refused_operand(&line("set", &["FOO={{ exec() }}".into()]), &patterns));
        assert!(presents_refused_operand(&line("set", &["--file=a{{b".into()]), &patterns));
        assert!(!presents_refused_operand(&line("set", &["FOO=bar".into()]), &patterns));
        assert!(!presents_refused_operand(&line("set", &["--file".into(), "FOO=bar".into()]), &patterns));
    }

    #[test]
    fn a_flag_name_is_not_an_operand() {
        let patterns = ["--force".to_string(), "*[*".to_string()];
        assert!(!presents_refused_operand(&line("use", &["--force".into(), "ruby@4".into()]), &patterns));
        assert!(presents_refused_operand(&line("use", &["--".into(), "x[y".into()]), &patterns));
    }

    #[test]
    fn the_sub_name_itself_is_not_an_operand() {
        assert!(!presents_refused_operand(&line("_", &[]), &["_".to_string()]));
    }

    fn reference(pattern: &str, text: &str) -> bool {
        match pattern.split_at_checked(1) {
            None => text.is_empty(),
            Some(("*", rest)) => (0..=text.len()).any(|cut| reference(rest, &text[cut..])),
            Some((head, rest)) => text.starts_with(head) && reference(rest, &text[1..]),
        }
    }

    fn refusing_subs(path: &mut Vec<String>, kind: &DispatchKind, out: &mut Vec<(Vec<String>, Vec<String>)>) {
        let DispatchKind::Branching { subs, .. } = kind else { return };
        for sub in subs {
            path.push(sub.name.clone());
            if !sub.refuse_operands.is_empty() {
                out.push((path.clone(), sub.refuse_operands.clone()));
            }
            refusing_subs(path, &sub.kind, out);
            path.pop();
        }
    }

    #[test]
    fn every_declared_refusal_denies_an_operand_that_matches_it() {
        let mut found = Vec::new();
        for (name, spec) in super::super::TOML_REGISTRY.iter() {
            let mut declared = Vec::new();
            refusing_subs(&mut vec![name.clone()], &spec.kind, &mut declared);
            found.extend(declared);
        }
        assert!(found.iter().any(|(path, _)| path.join(" ") == "mise set"), "mise set declares no refuse_operands");
        for (path, patterns) in found {
            for pattern in patterns {
                let operand = pattern.replace('*', "x");
                let mut words = path.clone();
                words.push(operand.clone());
                let line = shell_words::join(&words);
                assert!(!crate::command_verdict(&line).is_allowed(), "`{line}` matches `{pattern}` and must be refused");
            }
        }
    }

    #[test]
    fn a_declaration_is_checked_when_the_registry_is_built() {
        let empty = check_declaration("cmd", "sub", &[String::new()], false);
        assert!(empty.is_err_and(|e| e.contains("empty glob")));
        let profiled = check_declaration("cmd", "sub", &["*x*".to_string()], true);
        assert!(profiled.is_err_and(|e| e.contains("profiled")));
        assert!(check_declaration("cmd", "sub", &["*x*".to_string()], false).is_ok());
        assert!(check_declaration("cmd", "sub", &[], true).is_ok());
    }

    fn verdict_of(spec: &super::super::CommandSpec, words: &[&str]) -> Verdict {
        let tokens: Vec<Token> = words.iter().map(|w| Token::from_raw(w.to_string())).collect();
        super::super::dispatch_spec(&tokens, spec)
    }

    #[test]
    fn a_declared_refusal_gates_the_sub_its_aliases_and_a_nested_sub() {
        let source = r#"
[[command]]
name = "writer"
description = "A test command whose subs write text read back later."

[[command.sub]]
name = "set"
aliases = ["s"]
level = "SafeWrite"
valued = ["--file"]
refuse_operands = ["*{{*", "_", "_.*"]

[[command.sub]]
name = "group"
nested_bare = true

[[command.sub.sub]]
name = "add"
level = "SafeWrite"
refuse_operands = ["*[*"]
"#;
        let spec = super::super::load_toml(source, "test").unwrap().remove(0);
        assert!(verdict_of(&spec, &["writer", "set", "FOO=bar"]).is_allowed());
        assert!(verdict_of(&spec, &["writer", "set", "--file", "a.toml", "FOO=bar"]).is_allowed());
        for refused in ["FOO={{ x }}", "{{x}}", "_", "_=1", "_.source=./a.sh"] {
            assert!(!verdict_of(&spec, &["writer", "set", refused]).is_allowed(), "set {refused}");
            assert!(!verdict_of(&spec, &["writer", "s", refused]).is_allowed(), "s {refused}");
        }
        assert!(!verdict_of(&spec, &["writer", "set", "--file", "{{x}}", "FOO=bar"]).is_allowed());
        assert!(!verdict_of(&spec, &["writer", "set", "--file={{x}}", "FOO=bar"]).is_allowed());
        assert!(verdict_of(&spec, &["writer", "group", "add", "a@1"]).is_allowed());
        assert!(!verdict_of(&spec, &["writer", "group", "add", "a[x=1]"]).is_allowed());
    }

    #[test]
    fn a_profiled_sub_cannot_declare_a_refusal() {
        let source = "[[command]]\nname = \"writer\"\ndescription = \"x\"\n\n[[command.sub]]\nname = \"set\"\nprofile = \"remote-read\"\nfact = \"f\"\nsource = \"s\"\nrefuse_operands = [\"*x*\"]\n";
        assert!(super::super::load_toml(source, "test").is_err_and(|e| e.contains("profiled")));
    }

    proptest! {
        /// An operand holding a refused marker anywhere is refused, whatever surrounds it and
        /// wherever it sits among the other operands.
        #[test]
        fn an_operand_holding_a_marker_is_refused(
            before in "[A-Za-z0-9_ =.@/]{0,8}",
            after in "[A-Za-z0-9_ =.@/]{0,8}",
            marker in prop::sample::select(vec!["{{", "{%", "{#", "["]),
            others in proptest::collection::vec("[A-Za-z0-9_.@/]{1,6}", 0..3),
            slot in 0usize..3,
        ) {
            let patterns: Vec<String> = ["*{{*", "*{%*", "*{#*", "*[*"].map(String::from).to_vec();
            let mut operands = others.clone();
            let at = slot.min(operands.len());
            operands.insert(at, format!("{before}{marker}{after}"));
            prop_assert!(presents_refused_operand(&line("set", &operands), &patterns));
        }

        /// An operand with none of the markers is never refused by them, and the verdict does not
        /// depend on the other operands.
        #[test]
        fn operands_without_a_marker_are_unaffected(
            operands in proptest::collection::vec("[A-Za-z0-9_.@/=-]{1,10}", 0..4),
        ) {
            let patterns: Vec<String> = ["*{{*", "*{%*", "*{#*", "*[*"].map(String::from).to_vec();
            prop_assert!(!presents_refused_operand(&line("set", &operands), &patterns));
        }

        /// No patterns refuse nothing.
        #[test]
        fn no_patterns_refuse_nothing(operands in proptest::collection::vec("[ -~]{0,10}", 0..4)) {
            prop_assert!(!presents_refused_operand(&line("set", &operands), &[]));
        }

        /// The matcher agrees with a plain recursive reading of the same glob.
        #[test]
        fn glob_agrees_with_a_recursive_reading(pattern in "[a-c*]{0,6}", text in "[a-c]{0,8}") {
            prop_assert_eq!(glob_matches(&pattern, &text), reference(&pattern, &text));
        }

        /// A name given as the whole pattern matches only that operand name, glued or not.
        #[test]
        fn a_literal_name_matches_its_assignment_only(name in "[a-z_]{1,5}", other in "[a-z_]{1,5}", value in "[a-z0-9]{0,5}") {
            let patterns = [name.clone()];
            let own = format!("{name}={value}");
            prop_assert!(presents_refused_operand(&line("set", &[own]), &patterns));
            prop_assert_eq!(presents_refused_operand(&line("set", &[format!("{other}={value}")]), &patterns), other == name || value == name);
        }
    }
}
