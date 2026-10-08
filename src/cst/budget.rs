//! The parser's bounds: how deep it may recurse and how much work one `parse()` may do.
//!
//! Two counters bound work, because they fail differently. `WORK` counts `script()` entries and has
//! a flat ceiling, which stops a runaway nest early. It cannot bound time on its own: one entry may
//! re-read the rest of the input, so 20,000 entries over a 100 KB tail took 15 seconds. `STEPS`
//! counts what the parser actually reads, one step per lexing attempt plus one per `STEP_BYTES`
//! bytes lexed or scanned, against an allowance that grows with the input. A parse that reads each
//! byte a constant number of times fits, and anything super-linear runs out.
//!
//! Running out of either is sticky: `parse()` returns `None` once a budget is spent, whatever
//! alternative the parser found afterwards, so an exhausted parse always fails closed.

use std::cell::Cell;

thread_local! {
    static DEPTH: Cell<u32> = const { Cell::new(0) };
    static WORK: Cell<u64> = const { Cell::new(0) };
    static WORK_LIMIT: Cell<u64> = const { Cell::new(u64::MAX) };
    static STEPS: Cell<u64> = const { Cell::new(0) };
    static STEP_LIMIT: Cell<u64> = const { Cell::new(u64::MAX) };
    static SPENT: Cell<bool> = const { Cell::new(false) };
    #[cfg(test)]
    static PARSED: Cell<u64> = const { Cell::new(0) };
}

/// Bytes handed to `parse()` on this thread so far, for tests that bound a whole classification's
/// parsing rather than one parse.
#[cfg(test)]
pub(super) fn parsed_bytes() -> u64 {
    PARSED.with(Cell::get)
}

/// Nesting depth beyond which the parser bails instead of recursing further. Every recursion
/// source funnels through `script()` or `arith_sub`, so bounding them caps stack depth. A deep
/// adversarial nest (`"$("` × 100 000) would otherwise overflow the stack and abort the process,
/// a fail-open crash of the hook that `catch_unwind` cannot recover. Kept low because winnow's
/// combinator frames are fat (~200 levels overflowed a 2 MB stack), yet far beyond any real
/// command, which nests a handful of levels at most.
const MAX_DEPTH: u32 = 48;

/// Absolute ceiling on `script()` entries, whatever the input length. Measured across all 1338
/// registry examples the most any real command needs is 2, so a ceiling four orders of magnitude
/// above that cannot refuse anything real.
pub(super) const MAX_PARSE_WORK_CEILING: u64 = 20_000;
const WORK_BASE: u64 = 2_048;
const WORK_PER_BYTE: u64 = 8;

/// Bytes lexed or scanned per step.
const STEP_BYTES: usize = 16;
/// Steps allowed per input byte, on top of `STEP_BASE`. A linear parse spends well under one step
/// per byte (see the budget test in `parse.rs`), so this leaves each byte room to be read by every
/// enclosing construct many times over while a doubling nest exhausts it within a few levels.
pub(super) const STEPS_PER_BYTE: u64 = 8;
const STEP_BASE: u64 = 4_096;

pub(super) fn reset(input_len: usize) {
    let len = input_len as u64;
    #[cfg(test)]
    PARSED.with(|p| p.set(p.get().saturating_add(len)));
    DEPTH.with(|d| d.set(0));
    WORK.with(|w| w.set(0));
    WORK_LIMIT.with(|l| l.set((WORK_BASE + WORK_PER_BYTE * len).min(MAX_PARSE_WORK_CEILING)));
    STEPS.with(|s| s.set(0));
    STEP_LIMIT.with(|l| l.set(STEP_BASE + STEPS_PER_BYTE * len));
    SPENT.with(|s| s.set(false));
}

/// Whether a budget ran out during this parse.
pub(super) fn spent() -> bool {
    SPENT.with(Cell::get)
}

/// Charge `attempts` lexing attempts and `bytes` bytes read. `false` once the step budget is spent.
pub(super) fn charge(attempts: u64, bytes: usize) -> bool {
    let over = STEPS.with(|s| {
        let n = s.get().saturating_add(attempts + (bytes / STEP_BYTES) as u64);
        s.set(n);
        n > STEP_LIMIT.with(Cell::get)
    });
    if over {
        SPENT.with(|s| s.set(true));
    }
    !over
}

#[cfg(test)]
pub(super) fn work() -> u64 {
    WORK.with(Cell::get)
}

#[cfg(test)]
pub(super) fn steps() -> u64 {
    STEPS.with(Cell::get)
}

/// RAII depth counter for the recursive descent, which also counts a `script()`-level entry
/// against the work budget. `None` means bail: the nest is too deep or the budget is spent.
pub(super) struct DepthGuard;

impl DepthGuard {
    pub(super) fn enter() -> Option<Self> {
        let over = WORK.with(|w| {
            let n = w.get().saturating_add(1);
            w.set(n);
            n > WORK_LIMIT.with(Cell::get)
        });
        if over {
            SPENT.with(|s| s.set(true));
            return None;
        }
        DEPTH.with(|d| {
            if d.get() >= MAX_DEPTH {
                None
            } else {
                d.set(d.get() + 1);
                Some(DepthGuard)
            }
        })
    }
}

impl Drop for DepthGuard {
    fn drop(&mut self) {
        DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_charge_counts_its_attempts_plus_one_step_per_step_bytes() {
        reset(0);
        assert!(charge(1, 0));
        assert!(charge(1, STEP_BYTES * 3));
        assert_eq!(steps(), 1 + 4);
    }

    #[test]
    fn running_out_of_steps_is_sticky_until_the_next_reset() {
        reset(0);
        assert!(!spent());
        assert!(!charge(0, STEP_BYTES * (STEP_BASE as usize + 1)));
        assert!(spent());
        assert!(!charge(1, 0), "a spent budget must keep refusing");
        reset(0);
        assert!(!spent());
        assert!(charge(1, 0));
    }

    #[test]
    fn the_step_allowance_grows_with_the_input() {
        reset(1_000);
        let allowance = (STEP_BASE + STEPS_PER_BYTE * 1_000) as usize;
        assert!(charge(0, STEP_BYTES * allowance));
        assert!(!charge(1, 0));
    }

    #[test]
    fn running_out_of_work_is_sticky_too() {
        reset(0);
        let guards: Vec<_> = (0..MAX_DEPTH).map_while(|_| DepthGuard::enter()).collect();
        assert_eq!(guards.len(), MAX_DEPTH as usize, "the depth cap is not the budget");
        assert!(!spent(), "hitting the depth cap alone must not mark the budget spent");
        drop(guards);
        while DepthGuard::enter().is_some() {}
        assert!(spent());
    }
}
