#!/usr/bin/env bash
# Grade a blind run against its rubric (rubrics/<scenario>.md, out of the agent's reach).
#
#   eval/blind/grade.sh <run-name> <scenario>
#
# The grader sees the rubric, the agent's final answer, every editing tool call it made and the
# config before and after; never which model ran. An exact model id with no tools. Writes
# grade.json beside the transcript and prints "<total>/<max>  <summary>".
set -euo pipefail
grader=claude-sonnet-5-5

here=$(cd "$(dirname "$0")" && pwd)
runs=${BLIND_RUNS:-$HOME/Library/Caches/safe-chains-blind.noindex}
dir="$runs/${1:?run name}"
rubric="$here/rubrics/${2:?scenario}.md"
[ -f "$dir/transcript.jsonl" ] || { echo "no transcript for run $1" >&2; exit 1; }
[ -f "$rubric" ] || { echo "no rubric $rubric" >&2; exit 1; }
items=$(grep -c '^[0-9]\+\. ' "$rubric")

# Everything the agent said to the user, not only its last message: a rundown given before the
# final tool call is still an answer (finding R2 in results.toml).
answer=$(jq -r 'select(.type == "assistant") | .message.content[]? | select(.type == "text") | .text' "$dir/transcript.jsonl")
# Every tool call with the start of what it returned, in order, so the grader can see what the
# agent checked as well as what it changed. Shown only the edits, it marked down a diagnosis it
# could not see (finding R1 in results.toml).
calls=$(jq -rs '
  (map(select(.type == "user") | .message.content[]? | select(.type == "tool_result")
       | {key: .tool_use_id, value: ((.content | if type == "array" then map(.text // "") | join("") else tostring end)[:700])})
   | from_entries) as $results
  | .[] | select(.type == "assistant") | .message.content[]? | select(.type == "tool_use")
  | "CALL \(.name) \(.input | tostring)\nRESULT \($results[.id] // "(none)")\n"' "$dir/transcript.jsonl" | head -c 12000)

prompt=$(cat <<PROMPT
You grade an AI agent's work for a user against a rubric. You are not told which agent did it.

<rubric>
$(<"$rubric")
</rubric>

<config_before>
$(<"$dir/config-before.toml")
</config_before>

<config_after>
$(cat "$dir/home/.config/safe-chains.toml" 2>/dev/null)
</config_after>

<facts>
- A command safe-chains does not approve goes to the person's normal approval prompt in their coding tool. Saying it "will ask" or "will prompt" is correct.
- Every change to the rules applies from the next command on, with no restart.
- The tool results below are what safe-chains itself said; claims that repeat them are not invented.
</facts>

<tool_calls_and_results>
${calls:-none}
</tool_calls_and_results>

<agent_final_answer>
$answer
</agent_final_answer>

Reply with JSON only:
{"items": {"1": <0-2>, ...}, "total": <sum>, "errors": ["each invented or wrong claim"], "summary": "two sentences"}
PROMPT
)
claude -p --model "$grader" --tools '' --strict-mcp-config --setting-sources project,local "$prompt" |
  sed -n '/^{/,/^}/p' | jq . >"$dir/grade.json"
jq -r --arg max "$((items * 2))" '"\(.total)/\($max)  \(.summary)"' "$dir/grade.json"
