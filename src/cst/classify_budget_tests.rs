use super::budget::parsed_bytes;
use super::check::command_verdict;
use super::classify_budget::{CLASSIFY_BYTE_BASE, CLASSIFY_BYTES_PER_INPUT_BYTE};
use crate::verdict::Verdict;
use proptest::prelude::*;

const BRACE_FANOUT_SEED: &[u8] = include_bytes!("../../fuzz/corpus/level_monotonic/seed-timeout-brace-glob");

fn allowance(input: &str) -> u64 {
    CLASSIFY_BYTE_BASE + CLASSIFY_BYTES_PER_INPUT_BYTE * input.len() as u64
}

fn parsed_while(f: impl FnOnce()) -> u64 {
    let before = parsed_bytes();
    f();
    parsed_bytes() - before
}

fn fanned_out_wrapper(wrapper: &str, filler: &str, fanout: usize, tail: &str) -> String {
    format!("{filler}{}/{wrapper} {tail}", "{a,b}".repeat(fanout))
}

#[test]
fn a_brace_fanned_runner_chain_parses_within_the_byte_allowance() {
    let input = String::from_utf8_lossy(BRACE_FANOUT_SEED).into_owned();
    let start = std::time::Instant::now();
    let mut verdict = None;
    let parsed = parsed_while(|| verdict = Some(command_verdict(&input)));
    let explained = parsed_while(|| {
        let _ = super::explain(&input).render();
    });
    assert_eq!(verdict, Some(Verdict::Denied));
    assert!(parsed <= allowance(&input), "classifying parsed {parsed} bytes, over the {} allowance", allowance(&input));
    assert!(explained <= allowance(&input), "explaining parsed {explained} bytes, over the {} allowance", allowance(&input));
    let elapsed = start.elapsed();
    assert!(elapsed < std::time::Duration::from_secs(5), "the fan-out seed took {elapsed:?}");
}

#[test]
fn repeated_backtick_substitutions_parse_the_body_at_most_once() {
    let once = "`echo ok`";
    let expected = (once.len() + "echo ok".len()) as u64;
    assert_eq!(parsed_while(|| assert_eq!(command_verdict(once), Verdict::Denied)), expected);
    assert_eq!(parsed_while(|| assert_eq!(command_verdict(once), Verdict::Denied)), expected);

    for (input, body_len) in [
        ("`".repeat(100_000), 0),
        ("`a`".repeat(33_333), 1),
        (format!("{}{}", "`echo`".repeat(200), " a".repeat(8_000)), 4),
        (format!("{}{}{}", "{ ".repeat(40), "`".repeat(10_000), "; }".repeat(40)), 0),
    ] {
        let limit = (input.len() + body_len) as u64;
        let parsed = parsed_while(|| {
            assert_eq!(command_verdict(&input), Verdict::Denied);
        });
        assert_eq!(parsed, limit, "classifying len {} parsed {parsed} bytes (expected {limit})", input.len());
        let explained = parsed_while(|| {
            let exp = super::explain(&input);
            assert!(!exp.is_allowed());
            let _ = exp.render();
        });
        assert_eq!(explained, limit, "explaining len {} parsed {explained} bytes (expected {limit})", input.len());
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn delegation_never_parses_more_than_the_allowance(
        wrapper in prop::sample::select(&["bunx", "npx", "env", "nice", "nohup", "time", "sudo"][..]),
        filler in "[a-z]{0,40}",
        fanout in 1usize..=8,
        tail in prop::sample::select(&[":", "ls", "echo hi", "$x"][..]),
    ) {
        let input = fanned_out_wrapper(wrapper, &filler, fanout, tail);
        let parsed = parsed_while(|| {
            let _ = command_verdict(&input);
        });
        prop_assert!(parsed <= allowance(&input), "`{input}` parsed {parsed} bytes, over the {} allowance", allowance(&input));
        let explained = parsed_while(|| {
            let _ = super::explain(&input).render();
        });
        prop_assert!(explained <= allowance(&input), "explaining `{input}` parsed {explained} bytes, over the {} allowance", allowance(&input));
    }

    #[test]
    fn backtick_substitutions_never_parse_more_than_the_allowance(
        body in "[a-z0-9 _-]{1,16}",
        repeats in 1usize..=24,
        args in 0usize..=24,
    ) {
        let input = format!("{}{}", format!("`{body}`").repeat(repeats), " a".repeat(args));
        let limit = (input.len() + body.len()) as u64;
        let parsed = parsed_while(|| {
            let _ = command_verdict(&input);
        });
        prop_assert!(parsed <= limit, "classifying len {} parsed {parsed} bytes (limit {limit})", input.len());
        let explained = parsed_while(|| {
            let _ = super::explain(&input).render();
        });
        prop_assert!(explained <= limit, "explaining len {} parsed {explained} bytes (limit {limit})", input.len());
    }
}

#[test]
fn a_backtick_body_is_remembered_only_for_the_classification_that_read_it() {
    use super::classify_budget::{ClassifyGuard, MAX_CLASSIFY_WORK};
    use super::escape::parse_backtick;
    use std::rc::Rc;

    let bodies: Vec<String> = (0..=MAX_CLASSIFY_WORK).map(|i| format!("echo {i}")).collect();

    for body in &bodies {
        assert!(parse_backtick(body).is_some(), "`{body}` is refused outside a classification");
    }
    let twice = parsed_while(|| {
        let _ = parse_backtick("echo 1");
        let _ = parse_backtick("echo 1");
    });
    assert_eq!(twice, 2 * "echo 1".len() as u64, "outside a classification each read parses afresh");

    let guard = ClassifyGuard::enter(0).expect("a fresh budget");
    let first = parse_backtick(&bodies[0]).expect("parses");
    let again = parsed_while(|| assert!(Rc::ptr_eq(&first, &parse_backtick(&bodies[0]).expect("cached"))));
    assert_eq!(again, 0, "a body already read is not parsed again");
    for body in &bodies[1..MAX_CLASSIFY_WORK as usize] {
        assert!(parse_backtick(body).is_some(), "`{body}` is within the allowance");
    }
    assert!(parse_backtick(&bodies[MAX_CLASSIFY_WORK as usize]).is_none(), "one distinct body past the allowance is refused");
    assert!(parse_backtick(&bodies[0]).is_some(), "a remembered body is still read once the memory is full");
    drop(guard);

    let _guard = ClassifyGuard::enter(0).expect("a fresh budget");
    assert!(parse_backtick(&bodies[MAX_CLASSIFY_WORK as usize]).is_some(), "the memory is emptied when the classification ends");
}
