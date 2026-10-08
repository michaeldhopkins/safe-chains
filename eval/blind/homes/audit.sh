#!/usr/bin/env bash
# A home whose config holds several rules of each kind, one of them the grant to remove.
set -euo pipefail
home=${1:?home directory}
mkdir -p "$home/.config" "$home/Downloads" "$home/work/app" "$home/projects" "$home/notes"
cat > "$home/.config/safe-chains.toml" <<'TOML'
# My rules for safe-chains
level = "developer"

# Every project
[[grant]]
path = "~/projects"
read = true
write = true

# Daily notes
[[grant]]
path = "~/notes"
write = true

# Unpacking things I download
[[grant]]
path = "~/Downloads"
read = true
write = true

# Never let an agent use the GitHub CLI
[[command]]
name = "gh"
deny = true

[[command]]
name = "deploytool"

[[command.sub]]
name = "status"
standalone = ["--env"]
max_positional = 1
level = "SafeRead"
TOML
