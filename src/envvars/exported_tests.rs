use proptest::prelude::*;

use super::exported_verdict;
use crate::cst::eval::{UNPINNABLE, UNRESOLVED_BRACE};

const NAMES: &[&str] = &["PATH", "LD_PRELOAD", "GIT_DIR", "FOO", "EDITOR", "PAGER", "X"];
const VALUES: &[&str] = &["", "1", "/usr/bin", "/tmp/evil", "./bin", "$PATH:/tmp/x", "sh -c evil"];

fn arb_assignment() -> impl Strategy<Value = String> {
    (prop::sample::select(NAMES), prop::sample::select(VALUES)).prop_map(|(name, value)| format!("{name}={value}"))
}

fn arb_operand() -> impl Strategy<Value = String> {
    prop_oneof![arb_assignment(), prop::sample::select(NAMES).prop_map(str::to_string), Just("-x".to_string())]
}

fn hidden() -> impl Strategy<Value = String> {
    (prop::sample::select(&[UNPINNABLE, UNRESOLVED_BRACE][..]), "[A-Z_:/]{0,4}", "[A-Z_:/]{0,4}")
        .prop_map(|(marker, before, after)| format!("{before}{marker}{after}"))
}

proptest! {
    /// An operand the shell could expand into any assignment refuses the whole statement, wherever
    /// it sits among ordinary operands.
    #[test]
    fn an_operand_that_could_be_any_assignment_is_refused(
        before in prop::collection::vec(arb_operand(), 0..3),
        word in hidden(),
        after in prop::collection::vec(arb_operand(), 0..3),
    ) {
        let mut operands = before;
        operands.push(word);
        operands.extend(after);
        prop_assert!(!exported_verdict(operands.iter().map(String::as_str)).is_allowed(), "{:?}", operands);
    }

    /// Each assignment operand is judged exactly as the same assignment written as a statement.
    #[test]
    fn an_exported_assignment_agrees_with_the_statement(name in prop::sample::select(NAMES), value in prop::sample::select(VALUES)) {
        let word = format!("{name}={value}");
        let expected = super::super::statement_assignment_verdict(name, value, true);
        prop_assert_eq!(exported_verdict(std::iter::once(word.as_str())), expected);
    }
}

#[test]
fn a_plain_name_and_a_flag_are_inert() {
    assert!(exported_verdict(["-x", "FOO", "-n"].into_iter()).is_allowed());
}
