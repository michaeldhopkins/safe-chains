#!/bin/bash
#
# Print the revision mutants.yml's in-diff job diffs against, and say why on stderr.
#
# Usage: scripts/mutants-base.sh <event_name> <base_ref> <before>
#
#   event_name  github.event_name; a pull_request diffs against origin/<base_ref>
#   base_ref    github.base_ref (empty on a push)
#   before      github.event.before (the commit a push replaced; empty on a PR)
#
# A push diffs against `before` only when it is a commit in this checkout and an ancestor of HEAD.
# Otherwise it falls back to HEAD~1: all zeros for a new branch, and a force-push that rewrote
# history leaves `before` missing from the clone or off HEAD's ancestry (2026-10-07, where
# `git diff <before>..` failed the job). Read-only; run from the repository root with full history.

set -euo pipefail

event="${1:-}"
base_ref="${2:-}"
before="${3:-}"

if [ "$event" = "pull_request" ]; then
    echo "pull request: diffing against origin/$base_ref" >&2
    echo "origin/$base_ref"
    exit 0
fi

fallback() {
    echo "::notice::$1; diffing against HEAD~1" >&2
    echo "HEAD~1"
    exit 0
}

if [ -z "$before" ] || [ "$before" = "0000000000000000000000000000000000000000" ]; then
    fallback "no previous commit for this push (new branch)"
fi
if ! git cat-file -e "$before^{commit}" 2>/dev/null; then
    fallback "previous tip $before is not in this checkout (history rewritten by a force-push)"
fi
if ! git merge-base --is-ancestor "$before" HEAD; then
    fallback "previous tip $before is not an ancestor of HEAD (force-push)"
fi

echo "push: diffing against the previous tip $before" >&2
echo "$before"
