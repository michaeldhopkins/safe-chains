#!/usr/bin/env bash
# One fuzz burst: mutate a target for BUDGET seconds, then fold what it found into its corpus.
# Run by .github/workflows/fuzz.yml; runs the same locally.
#
#   fuzz/burst.sh <libfuzzer-binary> <target> <budget-seconds> [dict]
#
# Works in fuzz/corpus/<target> (the corpus, replaced by the minimized union), fuzz/new (this
# burst's finds) and fuzz/artifacts/<target>/ (crash-/timeout-/oom-/slow-unit- inputs).
#
# Why the budget is timed here and not with -max_total_time alone: libFuzzer counts that budget
# from process start, and loading the corpus counts. Measured locally: -max_total_time=2 over a 4k
# `parse` corpus spent ~10s loading and then stopped at INITED having mutated nothing. On a runner
# at ~46 exec/s a 12k corpus takes ~4.5 min to load, so a flat 180s would buy no mutation at all.
# So the clock starts at INITED, and SIGINT ends the run. libFuzzer exits 72 on an interrupt (the
# cargo-fuzz build does not accept -interrupted_exit_code), which is mapped to 0 here, and only when
# this script sent the interrupt.
#
# Exit status: the fuzzer's, if it stopped on its own (a crash, timeout, oom, or leak is
# non-zero); 0 after a clean interrupt; 2 if only the merge failed. The merge runs regardless, so
# the corpus is kept after a crash too. Under Actions, a successful merge sets the step output
# `merged=true`, which is what lets the workflow save the corpus.
set -uo pipefail

BIN="$1"; T="$2"; BUDGET="$3"; DICT_FILE="${4:-}"
LOAD_CAP=3600

mkdir -p "fuzz/corpus/$T" fuzz/new "fuzz/artifacts/$T"
DICT=()
if [ -n "$DICT_FILE" ] && [ -s "$DICT_FILE" ]; then DICT=(-dict="$DICT_FILE"); fi
echo "Restored corpus: $(find "fuzz/corpus/$T" -type f | wc -l | tr -d ' ') inputs; budget ${BUDGET}s after load"

LOG="$(mktemp)"
"$BIN" "${DICT[@]}" \
  -max_total_time=$((BUDGET + LOAD_CAP)) \
  -timeout=25 \
  -rss_limit_mb=4096 \
  -print_final_stats=1 \
  -artifact_prefix="fuzz/artifacts/$T/" \
  fuzz/new "fuzz/corpus/$T" 2>"$LOG" &
PID=$!

START=$SECONDS
while kill -0 "$PID" 2>/dev/null && ! grep -q 'INITED' "$LOG"; do sleep 1; done
echo "Loaded in $((SECONDS - START))s"

END=$((SECONDS + BUDGET))
while kill -0 "$PID" 2>/dev/null && [ "$SECONDS" -lt "$END" ]; do sleep 1; done
INTERRUPTED=0
if kill -INT "$PID" 2>/dev/null; then INTERRUPTED=1; fi
wait "$PID"
RC=$?
if [ "$INTERRUPTED" -eq 1 ] && [ "$RC" -eq 72 ]; then RC=0; fi
grep -E 'INITED|DONE|interrupted|Dictionary|stat::number_of_executed_units|ERROR|SUMMARY|Test unit written' "$LOG" || true
if [ "$RC" -ne 0 ]; then tail -40 "$LOG"; fi
rm -f "$LOG"
echo "fuzzer exit status: $RC"

mkdir -p minimized
echo "Prior corpus + seeds: $(find "fuzz/corpus/$T" -type f | wc -l | tr -d ' ') inputs"
echo "New this burst: $(find fuzz/new -type f | wc -l | tr -d ' ')"
if ! "$BIN" -merge=1 -timeout=25 -rss_limit_mb=4096 \
  -artifact_prefix="fuzz/artifacts/$T/" \
  minimized "fuzz/corpus/$T" fuzz/new 2>merge.log; then
  tail -40 merge.log
  echo "::error::merge failed; corpus left as restored and not marked for saving"
  rm -rf minimized merge.log
  [ "$RC" -ne 0 ] && exit "$RC"
  exit 2
fi
rm -rf "fuzz/corpus/$T" merge.log
mv minimized "fuzz/corpus/$T"
echo "Minimized corpus: $(find "fuzz/corpus/$T" -type f | wc -l | tr -d ' ') inputs"
if [ -n "${GITHUB_OUTPUT:-}" ]; then echo "merged=true" >> "$GITHUB_OUTPUT"; fi

exit "$RC"
