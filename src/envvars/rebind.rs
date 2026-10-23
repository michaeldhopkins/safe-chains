use crate::cst::Word;
use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

pub(crate) fn loop_var_verdict(var: &str, items: &[Word]) -> Verdict {
    crate::cst::arith::loop_verdict(var, items).combine(listed_loop_verdict(var, items))
}

fn listed_loop_verdict(var: &str, items: &[Word]) -> Verdict {
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
    let literal: Vec<bool> = words.iter().map(crate::cst::opaque::glob::is_literal).collect();
    let words: Vec<String> = words.iter().map(Word::eval).collect();
    let start = words.iter().take_while(|w| matches!(command_name(w).as_str(), "builtin" | "command")).count();
    let Some(name) = words.get(start).map(|w| command_name(w)) else {
        return Verdict::Allowed(SafetyLevel::Inert);
    };
    let args = &words[start + 1..];
    let literal = &literal[start + 1..];
    // An operand that is not one literal text could name any variable: `x=HOME; read $x`,
    // `read HOM?`, `read $(echo HOME)`, `read {HOME,x}`.
    let unread = || args.iter().zip(literal).any(|(w, &lit)| !lit && !w.starts_with('-'));
    // An option that is not one literal text could be any option: `typeset -$x` with x=T is `-T`,
    // `declare -$(echo n)` is a nameref, `read -a$(echo HOME)` names the array.
    let unread_option = || args.iter().zip(literal).any(|(w, &lit)| !lit && w.starts_with(['-', '+']));
    let tie = || ties(args) && (unread() || ties_a_listed_variable(args));
    let refused = match name.as_str() {
        "read" => unread() || unread_option() || args.iter().any(|w| names_a_listed_variable(w)),
        "declare" | "typeset" | "local" => {
            (args.iter().any(|w| w.starts_with('-') && w.contains('n')) && args.iter().any(|w| !w.starts_with(['-', '+'])))
                || unread_option()
                || tie()
                || args.iter().any(|w| unread_name(w))
        }
        "export" | "readonly" => unread_option() || tie() || args.iter().any(|w| unread_name(w)),
        _ => false,
    };
    if refused { Verdict::Denied } else { Verdict::Allowed(SafetyLevel::Inert) }
}

fn command_name(word: &str) -> String {
    Token::from_raw(word.to_string()).command_name().to_string()
}

/// zsh's `typeset -T X chpwd_functions` ties a scalar to an array, so a later `X=f` sets the array
/// through a name no table lists.
fn ties(args: &[String]) -> bool {
    args.iter().any(|w| w.starts_with('-') && w.contains('T'))
}

fn ties_a_listed_variable(args: &[String]) -> bool {
    args.iter()
        .filter(|w| !w.starts_with(['-', '+']))
        .any(|w| super::rule_of(w.split('=').next().unwrap_or_default()).is_some())
}

/// A declared name whose spelling is not its text: `declare HOM?=x` beside a file named `HOME=x`.
/// Only the part before `=` counts, so `export FOO="$BAR"` is untouched.
fn unread_name(arg: &str) -> bool {
    let name = arg.split('=').next().unwrap_or_default();
    !arg.starts_with(['-', '+']) && (name.contains(['$', '`', '*', '?', '[', '{']) || name.contains("__SAFE_CHAINS_"))
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
