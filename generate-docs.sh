#!/bin/bash
set -e
cargo build --quiet
target/debug/safe-chains --list-commands > COMMANDS.md
echo "Generated COMMANDS.md"
