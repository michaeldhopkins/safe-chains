#!/bin/bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
version=$(pandoc --version | awk 'NR==1 {print $2}')
out="$root/tests/fixtures/pandoc/$version"
mkdir -p "$out"
pandoc --list-input-formats > "$out/input-formats.txt"
pandoc --list-output-formats > "$out/output-formats.txt"
pandoc --help > "$out/help.txt"
echo "pandoc $version: captured"
