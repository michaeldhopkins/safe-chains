#!/bin/bash
#
# Print the base revision the pre-release `--in-diff` mutants check diffs against, and say why on
# stderr.
#
# Usage: scripts/mutants-base.sh [<base>]
#
#   base  last commit covered by a clean pre-release check (or the remote tip being replaced)
#
# Diffs against `base` only when it is a commit in this checkout and an ancestor of HEAD. Otherwise
# it falls back to HEAD~1: empty or all zeros when no prior base is recorded, and rewritten history
# leaves `base` missing from the clone or off HEAD's ancestry (2026-10-07, where `git diff <base>..`
# failed). Read-only; run from the repository root with full history.

set -euo pipefail

before="${1:-}"

fallback() {
    echo "::notice::$1; diffing against HEAD~1" >&2
    echo "HEAD~1"
    exit 0
}

if [ -z "$before" ] || [ "$before" = "0000000000000000000000000000000000000000" ]; then
    fallback "no previous base commit recorded"
fi
if ! git cat-file -e "$before^{commit}" 2>/dev/null; then
    fallback "previous base $before is not in this checkout (history rewritten)"
fi
if ! git merge-base --is-ancestor "$before" HEAD; then
    fallback "previous base $before is not an ancestor of HEAD (history rewritten)"
fi

echo "diffing against the previous base $before" >&2
echo "$before"
