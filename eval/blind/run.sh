#!/usr/bin/env bash
# Run one blind agent against `safe-chains mcp`: a fresh Claude Code session that knows nothing
# about safe-chains, given only a user's opening message and the server's tools.
#
#   eval/blind/run.sh start <scenario> [<run-name>]   # a new session from scenarios/<scenario>.md
#   eval/blind/run.sh say <run-name> "<reply>"        # the user's next message, same session
#
# Blind: the agent runs from an empty directory outside the repository, with the user setting
# source left out, slash commands off and no tools but the server's, so no project or machine
# instructions, skills, hooks or memories reach it. The server gets a home of its own, seeded by
# the scenario's `<!-- home: NAME -->` directive (homes/NAME.sh), so nothing touches the real
# ~/.config/safe-chains.toml. The run's home and transcript sit beside the agent's directory,
# out of its reach.
#
# Models are exact ids, never an alias (`haiku` moves with Claude Code):
#   BLIND_MODEL=claude-haiku-5-5 eval/blind/run.sh start a-unknown-tool
#   BLIND_PROVIDER=openrouter BLIND_MODEL=<slug> BLIND_OPENROUTER_KEY=... eval/blind/run.sh start …
# An OpenRouter run uses the Anthropic-compatible endpoint with every model tier pointed at the
# run's model, and stops at BLIND_MAX_TURNS (default 40). The key arrives in the environment and
# must be one whose spend can be told apart: Claude Code cannot tag OpenRouter requests itself, so
# BLIND_OPENROUTER_BASE can point at a local proxy that adds the `user` and `trace` tags.
#
# Runs live under BLIND_RUNS (default ~/Library/Caches/safe-chains-blind.noindex), outside any
# temporary directory: safe-chains approves writes in temp directories, which would hide what the
# scenarios test. The server is the build at target/debug/safe-chains; build it first.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=eval/blind/lib.sh
source "$here/lib.sh"
repo=$(cd "$here/../.." && pwd)
runs=${BLIND_RUNS:-$HOME/Library/Caches/safe-chains-blind.noindex}
binary="$repo/target/debug/safe-chains"

blind() { # dir [claude args...]: one turn, events appended to the transcript
  local dir=$1
  shift
  local model=() turns=()
  [ -s "$dir/model" ] && model=(--model "$(<"$dir/model")")
  [ "$(<"$dir/provider")" = openrouter ] && turns=(--max-turns "${BLIND_MAX_TURNS:-40}")
  (cd "$dir/work" && provider_env "$dir" claude -p --setting-sources project,local --disable-slash-commands \
    --strict-mcp-config --mcp-config "$dir/mcp.json" --tools '' --allowedTools mcp__safechains \
    "${model[@]}" "${turns[@]}" --output-format stream-json --verbose "$@") >>"$dir/transcript.jsonl"
  jq -rs '[.[] | select(.type == "result")] | last | .result' "$dir/transcript.jsonl"
}

provider_env() { # dir command...
  local dir=$1 model
  shift
  if [ "$(<"$dir/provider")" != openrouter ]; then "$@"; return; fi
  model=$(<"$dir/model")
  ANTHROPIC_BASE_URL="${BLIND_OPENROUTER_BASE:-https://openrouter.ai/api}" ANTHROPIC_AUTH_TOKEN="${BLIND_OPENROUTER_KEY:?the OpenRouter key for this evaluation}" \
    ANTHROPIC_API_KEY="" ANTHROPIC_DEFAULT_OPUS_MODEL="$model" ANTHROPIC_DEFAULT_SONNET_MODEL="$model" \
    ANTHROPIC_DEFAULT_HAIKU_MODEL="$model" CLAUDE_CODE_SUBAGENT_MODEL="$model" "$@"
}

case "${1:-}" in
  start)
    [ "${BLIND_PROVIDER:-subscription}" = openrouter ] || refuse_alias "${BLIND_MODEL:-}" || exit 2
    [ -x "$binary" ] || { echo "no $binary; run cargo build first" >&2; exit 1; }
    scenario="$here/scenarios/${2:?scenario}.md"
    [ -f "$scenario" ] || { echo "no scenario $scenario" >&2; exit 1; }
    name=${3:-$2-$(date +%Y%m%d-%H%M%S)}
    dir="$runs/$name"
    [ ! -e "$dir" ] || { echo "run $name exists; use: say $name" >&2; exit 1; }
    mkdir -p "$dir/work" "$dir/home"
    message=$(<"$scenario")
    directive='^<!-- home: ([a-z-]+) -->'
    [[ $message =~ $directive ]] || { echo "$scenario names no home" >&2; exit 1; }
    "$here/homes/${BASH_REMATCH[1]}.sh" "$dir/home"
    message=${message#*$'\n'}
    cp "$dir/home/.config/safe-chains.toml" "$dir/config-before.toml"
    jq -n --arg bin "$binary" --arg home "$dir/home" \
      '{mcpServers: {safechains: {type: "stdio", command: $bin, args: ["mcp"], env: {HOME: $home}}}}' >"$dir/mcp.json"
    printf '%s' "${BLIND_MODEL:-}" >"$dir/model"
    printf '%s' "${BLIND_PROVIDER:-subscription}" >"$dir/provider"
    session=$(uuidgen | tr '[:upper:]' '[:lower:]')
    echo "$session" >"$dir/session"
    echo "run $name, session $session" >&2
    blind "$dir" --session-id "$session" "$message"
    ;;
  say)
    dir="$runs/${2:?run name}"
    [ -f "$dir/session" ] || { echo "no run $2" >&2; exit 1; }
    blind "$dir" --resume "$(<"$dir/session")" "${3:?reply}"
    ;;
  *) sed -n '2,26p' "$0" >&2; exit 2 ;;
esac
