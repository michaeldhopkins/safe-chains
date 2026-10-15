use crate::cst::Word;
use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

pub(crate) fn loop_var_verdict(var: &str, items: &[Word]) -> Verdict {
    if super::rule_of(var).is_none() {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    if items.is_empty() {
        return Verdict::Denied;
    }
    items
        .iter()
        .map(Word::eval)
        .map(|v| item_verdict(var, &v))
        .fold(Verdict::Allowed(SafetyLevel::Inert), Verdict::combine)
}

fn item_verdict(var: &str, value: &str) -> Verdict {
    let tilde_elsewhere = value.get(1..).is_some_and(|rest| rest.contains('~'));
    if tilde_elsewhere || value.contains(['*', '?', '[', '$', '{', '}', '\\']) || value.contains("__SAFE_CHAINS_") {
        return Verdict::Denied;
    }
    super::assignment_verdict(var, value)
}

pub(crate) fn rebinding_verdict(words: &[Word]) -> Verdict {
    let words: Vec<String> = words.iter().map(Word::eval).collect();
    let start = words.iter().take_while(|w| matches!(command_name(w).as_str(), "builtin" | "command")).count();
    let Some(name) = words.get(start).map(|w| command_name(w)) else {
        return Verdict::Allowed(SafetyLevel::Inert);
    };
    let args = &words[start + 1..];
    let refused = match name.as_str() {
        "read" => args.iter().any(|w| names_a_listed_variable(w)),
        "declare" | "typeset" | "local" => {
            args.iter().any(|w| w.starts_with('-') && w.contains('n')) && args.iter().any(|w| !w.starts_with(['-', '+']))
        }
        _ => false,
    };
    if refused { Verdict::Denied } else { Verdict::Allowed(SafetyLevel::Inert) }
}

fn command_name(word: &str) -> String {
    Token::from_raw(word.to_string()).command_name().to_string()
}

fn names_a_listed_variable(arg: &str) -> bool {
    let arg = arg.split('?').next().unwrap_or_default();
    let Some(cluster) = arg.strip_prefix('-') else {
        return super::rule_of(arg).is_some();
    };
    (0..cluster.len())
        .filter(|&i| cluster.is_char_boundary(i))
        .any(|i| super::rule_of(&cluster[i..]).is_some())
}

#[cfg(test)]
#[path = "rebind_tests.rs"]
mod tests;
