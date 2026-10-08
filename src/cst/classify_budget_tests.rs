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
}
