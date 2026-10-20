#!/bin/bash
# Capture grok hook envelopes and the hook's replies as fixtures under tests/fixtures/grok/<version>/.
#
# Usage: scripts/capture-grok-fixtures.sh [--mode MODE ...]   (default: default auto bypassPermissions)
#
# Runs `grok -p` once per mode in a scratch directory, with a `safe-chains` wrapper first on PATH so
# the installed global hook (`safe-chains hook grok`) runs this checkout's build and logs what it
# saw and said. No grok config or hook file is changed (grok -p still writes its own session
# files). It captures nothing if the installed hook names safe-chains by an absolute path, and
# reports "the hook was not called". Each run spends one short grok turn. Writes only the
# fixture directory; the interactive outcomes in outcomes.toml are recorded by hand.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
modes=()
while [[ $# -gt 0 ]]; do
  case $1 in
    --mode) modes+=("$2"); shift 2 ;;
    --mode=*) modes+=("${1#--mode=}"); shift ;;
    *) echo "unknown argument: $1" >&2; exit 64 ;;
  esac
done
[[ ${#modes[@]} -gt 0 ]] || modes=(default auto bypassPermissions)

version=$(grok --version | awk '{print $2}')
out="$root/tests/fixtures/grok/$version"
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$scratch/bin" "$scratch/log" "$scratch/ws" "$out"
cargo build --manifest-path "$root/Cargo.toml" --quiet

cat > "$scratch/bin/safe-chains" <<EOF
#!/bin/bash
in=\$(cat)
printf '%s' "\$in" > "$scratch/log/in.json"
reply=\$(printf '%s' "\$in" | "$root/target/debug/safe-chains" "\$@")
code=\$?
printf '%s' "\$reply" > "$scratch/log/out.json"
echo "\$code" > "$scratch/log/code"
printf '%s' "\$reply"
exit "\$code"
EOF
chmod +x "$scratch/bin/safe-chains"

sanitize='.cwd="/Users/me/project" | .workspaceRoot="/Users/me/project"
  | .sessionId="00000000-0000-0000-0000-000000000000" | .session_id=.sessionId
  | .transcriptPath="/Users/me/.grok/sessions/project/session/updates.jsonl" | .transcript_path=.transcriptPath
  | .toolUseId="call-0" | .tool_use_id="call-0" | .timestamp="2026-01-01T00:00:00+00:00"'
prompt='Use your shell tool to run exactly this command and nothing else: zzgatetool sync. Do not retry.'

for mode in "${modes[@]}"; do
  rm -f "$scratch/log/"*
  (cd "$scratch/ws" && PATH="$scratch/bin:$PATH" grok -p "$prompt" --permission-mode "$mode" > /dev/null 2>&1) || true
  if [[ ! -f "$scratch/log/in.json" ]]; then
    echo "$mode: the hook was not called" >&2
    continue
  fi
  jq --sort-keys "$sanitize" "$scratch/log/in.json" > "$out/$mode.envelope.json"
  jq --sort-keys . "$scratch/log/out.json" > "$out/$mode.reply.json"
  cp "$scratch/log/code" "$out/$mode.exit"
  echo "$mode: captured"
done
