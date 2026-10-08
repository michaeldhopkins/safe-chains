#!/usr/bin/env bash
# Run one scenario across models, several times each, and grade every run.
#
#   eval/blind/matrix.sh <scenario> <runs-per-model> <model>...
#
# A model is an exact Claude model id on the subscription (the mid reference, e.g.
# claude-haiku-5-5) or `openrouter:<slug>`, which needs BLIND_OPENROUTER_KEY. Before each
# OpenRouter run the key's spend is read, and the matrix stops once the spend since
# BLIND_SPEND_BASELINE (the key's spend when the round began; default: when this matrix began)
# reaches BLIND_BUDGET_USD (default 3). Export the baseline once to cap a whole round. A run that never
# starts stops the matrix. Results: matrix-<scenario>.tsv in the runs directory, one line a run:
# model, n, score, tool calls, seconds, cost, run name.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=eval/blind/lib.sh
source "$here/lib.sh"
runs=${BLIND_RUNS:-$HOME/Library/Caches/safe-chains-blind.noindex}
scenario=${1:?scenario}
count=${2:?runs per model}
shift 2
[ $# -gt 0 ] || { echo "name at least one model" >&2; exit 2; }
for spec in "$@"; do
  [[ $spec == openrouter:* ]] || refuse_alias "$spec" || exit 2
done
budget=${BLIND_BUDGET_USD:-3}
baseline=${BLIND_SPEND_BASELINE:-}
results="$runs/matrix-$scenario.tsv"
mkdir -p "$runs"

openrouter_spent() { # the key reaches curl on stdin, never argv
  printf 'header = "Authorization: Bearer %s"\n' "${BLIND_OPENROUTER_KEY:?}" |
    curl -sS --max-time 20 -K - https://openrouter.ai/api/v1/key | jq -r '.data.usage // 0'
}

calls() { # run: how many tool calls its agent made
  jq -s '[.[] | select(.type == "assistant") | .message.content[]? | select(.type == "tool_use")] | length' \
    "$runs/$1/transcript.jsonl"
}

for spec in "$@"; do
  provider=subscription model=$spec
  [[ $spec == openrouter:* ]] && provider=openrouter model=${spec#openrouter:}
  for n in $(seq 1 "$count"); do
    [ "$provider" = openrouter ] && [ -z "$baseline" ] && baseline=$(openrouter_spent)
    if [ "$provider" = openrouter ] && awk -v s="$(openrouter_spent)" -v z="$baseline" -v b="$budget" 'BEGIN { exit !(s - z >= b) }'; then
      echo "budget of \$$budget reached since the round began; stopping" >&2
      exit 0
    fi
    name="$scenario-$(echo "$model" | tr '/:.' '---')-$n-$(date +%H%M%S)"
    before=0
    [ "$provider" = openrouter ] && before=$(openrouter_spent)
    started=$(date +%s)
    BLIND_PROVIDER=$provider BLIND_MODEL=$model "$here/run.sh" start "$scenario" "$name" >/dev/null 2>&1 || true
    if [ ! -s "$runs/$name/transcript.jsonl" ]; then
      echo "$name never started (no transcript); stopping. Run it by hand to see why:" >&2
      echo "  BLIND_PROVIDER=$provider BLIND_MODEL=$model $here/run.sh start $scenario debug" >&2
      exit 1
    fi
    seconds=$(($(date +%s) - started))
    cost=0
    [ "$provider" = openrouter ] && cost=$(awk -v a="$(openrouter_spent)" -v b="$before" 'BEGIN { printf "%.4f", a - b }')
    score=$("$here/grade.sh" "$name" "$scenario" 2>/dev/null | cut -d' ' -f1 || echo "?")
    line=$(printf '%s\t%s\t%s\t%s\t%s\t%s\t%s' "$model" "$n" "$score" "$(calls "$name")" "$seconds" "$cost" "$name")
    echo "$line" >>"$results"
    echo "$line"
  done
done
