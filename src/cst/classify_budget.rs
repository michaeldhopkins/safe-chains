//! The classifier's per-command budget: how many (re-)classifications one top-level check may
//! make, and how many bytes it may hand the parser across all of them.

thread_local! {
    /// Total (re-)classifications spent on one top-level `command_verdict`. Delegating handlers
    /// (`fd -x`, `find -exec`, `xargs`, `sudo`) re-enter here on the wrapped command, and a command
    /// that NESTS them — `fd a b -x fd c d -x …` — branches multiplicatively (one re-check per
    /// pre-exec base × per nesting level), i.e. exponentially. This monotonic counter caps the total
    /// so any such blow-up fails CLOSED (Denied) in bounded time instead of hanging the hook. A depth
    /// cap alone can't help: 3^depth calls explode long before any depth limit bites.
    static CLASSIFY_WORK: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static CLASSIFY_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    /// Bytes handed to the parser across one top-level classification, and the most it may hand.
    /// A unit of `CLASSIFY_WORK` is not a fixed cost: a delegated command is re-joined from tokens
    /// that brace expansion may already have multiplied, so one unit can parse far more than the
    /// whole input. `bunx` with a word ending `/bunx` that expands to hundreds of copies re-parses
    /// every remaining copy once per level, quadratic in the copies, and a 769-byte input
    /// (`level_monotonic` burst, 2026-10-08) parsed tens of megabytes before the unit count ran out.
    static CLASSIFY_BYTES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static CLASSIFY_BYTE_LIMIT: std::cell::Cell<u64> = const { std::cell::Cell::new(u64::MAX) };
}

/// The parser bytes one classification may spend: a fixed allowance plus a multiple of the
/// command's own length. Real delegation re-parses a tail of the input once per wrapper, so it
/// spends a few times the input; this is far above that and far below the quadratic re-parse.
pub(super) const CLASSIFY_BYTE_BASE: u64 = 64 * 1024;
pub(super) const CLASSIFY_BYTES_PER_INPUT_BYTE: u64 = 64;

/// Far above any real command's handful of delegations (a `&&` chain of 50 `fd -x`s spends ~100),
/// far below the exponential explosion. Found by the parse fuzzer (`fd -x fd -x …`). Kept modest so
/// the worst-case CUTOFF is also cheap in wall-clock terms — each unit is a full re-classification
/// (parse + dispatch), so a high ceiling would let a crafted command burn hundreds of ms in the hook
/// (and blow the debug-mode timing of `classifier_terminates_on_adversarial_input`).
const MAX_CLASSIFY_WORK: u32 = 512;

/// RAII budget guard for the classifier recursion. `enter` resets the budget at the OUTERMOST call
/// and charges one unit, plus the `input_len` bytes about to be parsed, per (re-)entry; `None` means
/// the budget is spent and the caller must fail closed. Depth is bumped only on a successful enter,
/// so it stays balanced with the `Drop`.
pub(crate) struct ClassifyGuard;

impl ClassifyGuard {
    pub(crate) fn enter(input_len: usize) -> Option<Self> {
        let len = input_len as u64;
        if CLASSIFY_DEPTH.with(|d| d.get()) == 0 {
            CLASSIFY_WORK.with(|w| w.set(0));
            CLASSIFY_BYTES.with(|b| b.set(0));
            CLASSIFY_BYTE_LIMIT.with(|l| l.set(CLASSIFY_BYTE_BASE.saturating_add(CLASSIFY_BYTES_PER_INPUT_BYTE.saturating_mul(len))));
        }
        let spent = CLASSIFY_WORK.with(|w| {
            let n = w.get().saturating_add(1);
            w.set(n);
            n
        });
        let bytes = CLASSIFY_BYTES.with(|b| {
            let n = b.get().saturating_add(len);
            b.set(n);
            n
        });
        if spent > MAX_CLASSIFY_WORK || bytes > CLASSIFY_BYTE_LIMIT.with(std::cell::Cell::get) {
            return None;
        }
        CLASSIFY_DEPTH.with(|d| d.set(d.get() + 1));
        Some(ClassifyGuard)
    }
}

impl Drop for ClassifyGuard {
    fn drop(&mut self) {
        let depth = CLASSIFY_DEPTH.with(|d| {
            let n = d.get().saturating_sub(1);
            d.set(n);
            n
        });
        // Clearing on the way OUT, not only on the way in, is what makes the budget per-call for
        // callers that never take a guard. `explain()` and `suggest::analyze()` walk and brace-expand
        // a command without entering here, so they used to start with whatever the previous
        // classification had spent and trip `MAX_CLASSIFY_WORK` on work they had not done. That made
        // the verdict ORDER-DEPENDENT: `perl {,} -{,}e{,}{,}{,}\~{,}{,}{,}{,}` was allowed by
        // `is_safe_command` and reported not-allowed by a following `explain` — the hook auto-approving
        // while telling the reader it had not. Found by the `explain_render` fuzz target.
        if depth == 0 {
            CLASSIFY_WORK.with(|w| w.set(0));
            CLASSIFY_BYTES.with(|b| b.set(0));
        }
    }
}

/// Charge `units` of extra work to the shared per-classification budget; `false` once it is spent
/// and the caller must fail closed.
///
/// Brace expansion charges here so its fan-out draws from the SAME pool as delegation and function
/// resolution. Otherwise the two caps MULTIPLY rather than add: a word may expand to
/// `BRACE_EXPANSION_CAP` (256) alternatives and each delegated re-classification re-expands it, so
/// 512 delegations × 256 words is ~131k word checks — seconds of wall clock from a ~200-byte input
/// (found by the nightly fuzzer as a timeout). Neither cap is unreasonable alone; only their product
/// is. Charging fan-out here makes the total additive and keeps the worst case bounded.
pub(crate) fn charge_classify_work(units: u32) -> bool {
    CLASSIFY_WORK.with(|w| {
        let n = w.get().saturating_add(units);
        w.set(n);
        n <= MAX_CLASSIFY_WORK
    })
}
