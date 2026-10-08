#!/usr/bin/env bash
# A home directory with a small ~/.config/safe-chains.toml, a notes folder, and a project to run
# from. HOME is the fixture's own, so nothing reads the recording machine's config. Record with
# RECORD_ROOT outside the temp directory: a fixture under /tmp would make every write there an
# approved temp-file write and hide the refusals being shown.
set -euo pipefail
# shellcheck source=/dev/null
source "$RECORD_HELPERS"
use_tool jq
mkdir -p "$HOME/.config" "$HOME/notes" "$HOME/work/app"
cat > "$HOME/.config/safe-chains.toml" <<'TOML'
# My rules for safe-chains
level = "developer"

# Every project
[[grant]]
path = "~/projects"
read = true
write = true
TOML
cat > "$FIXTURE/bin/mcp-call" <<'SH'
#!/bin/sh
# mcp-call TOOL JSON-ARGUMENTS: one tool call to `safe-chains mcp`, printing the text it returns.
printf '%s\n%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}' \
  "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"$1\",\"arguments\":$2}}" |
  safe-chains mcp | tail -n 1 | jq -r '(if .result.isError then "(error) " else "" end) + .result.content[0].text'
SH
cat > "$FIXTURE/bin/mcp-tools" <<'SH'
#!/bin/sh
# mcp-tools: the server's instructions and each tool's description, as a client is shown them.
printf '%s\n%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' | safe-chains mcp |
  jq -r 'if .id == 1 then "INSTRUCTIONS\n" + .result.instructions + "\n" else (.result.tools[] | "\n" + .name + (if .annotations.destructiveHint then "  (destructive)" else "" end) + "\n" + .description + "\n" + ([.inputSchema.properties | to_entries[] | "  " + .key + ": " + (.value.description // "")] | join("\n"))) end'
SH
chmod +x "$FIXTURE/bin/mcp-call" "$FIXTURE/bin/mcp-tools"
