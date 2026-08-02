#!/bin/bash
set -e

cargo build --quiet

# Via a temp file, not `> COMMANDS.md`: the shell truncates a redirect target BEFORE running the
# command, so an interrupted run leaves the committed file empty. That happened, and nothing local
# noticed — the freshness check lives in CI.
target/debug/safe-chains --list-commands > COMMANDS.md.tmp
mv COMMANDS.md.tmp COMMANDS.md
echo "Generated COMMANDS.md"

target/debug/safe-chains --generate-book
echo "Generated command reference in docs/src/commands/"

if command -v mdbook &> /dev/null; then
    mdbook build docs/
    echo "Built book in docs/book/"

    SITE_DIR="$HOME/projects/michaeldhopkins.com/public/docs/safe-chains"
    mkdir -p "$SITE_DIR"
    rsync -a --delete docs/book/ "$SITE_DIR/"
    echo "Deployed to $SITE_DIR"
else
    echo "mdbook not found — skipping book build (install: cargo install mdbook)"
fi
