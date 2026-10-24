//! The operands of `export`, `declare -x` and the other builtins that set a variable for every later
//! command. `export LD_PRELOAD=/tmp/evil.so` is the `LD_PRELOAD=/tmp/evil.so cmd` prefix wearing
//! different syntax, so each assignment is judged the same way.
//!
//! An operand whose text could not be pinned (a substitution, or a brace the expander declined)
//! may be any assignment at all once the shell expands it, so it is refused rather than read as a
//! name with no value.

use crate::verdict::{SafetyLevel, Verdict};

pub(crate) fn exported_verdict<'a>(operands: impl Iterator<Item = &'a str>) -> Verdict {
    operands_verdict(operands, |name, value| super::statement_assignment_verdict(name, value, true))
}

/// The operands of a tool that stores each assignment for later shells (`mise set`), judged by
/// `stored_assignment_verdict`.
pub(crate) fn stored_verdict<'a>(operands: impl Iterator<Item = &'a str>) -> Verdict {
    operands_verdict(operands, super::stored_assignment_verdict)
}

fn operands_verdict<'a>(operands: impl Iterator<Item = &'a str>, judge: impl Fn(&str, &str) -> Verdict) -> Verdict {
    let mut verdict = Verdict::Allowed(SafetyLevel::Inert);
    for operand in operands.filter(|word| !word.starts_with('-')) {
        let judged = match operand.split_once('=') {
            Some((name, value)) => judge(name, value),
            None if hides_an_assignment(operand) => Verdict::Denied,
            None => Verdict::Allowed(SafetyLevel::Inert),
        };
        verdict = verdict.combine(judged);
    }
    verdict
}

fn hides_an_assignment(word: &str) -> bool {
    word.contains(crate::cst::eval::UNPINNABLE) || word.contains(crate::cst::eval::UNRESOLVED_BRACE)
}

#[cfg(test)]
#[path = "exported_tests.rs"]
mod tests;
