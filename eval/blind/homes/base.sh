#!/usr/bin/env bash
# A home with a small ~/.config/safe-chains.toml and the folders the scenarios name.
set -euo pipefail
home=${1:?home directory}
mkdir -p "$home/.config" "$home/notes" "$home/work/app" "$home/projects"
cat > "$home/.config/safe-chains.toml" <<'TOML'
# My rules for safe-chains
level = "developer"

# Every project
[[grant]]
path = "~/projects"
read = true
write = true
TOML
