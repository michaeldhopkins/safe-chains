#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"
cargo build --quiet 2>/dev/null

output=$(target/debug/safe-chains --list-commands 2>/dev/null)

commands=$(echo "$output" | grep -c '^### ')

subcommands=$(echo "$output" | grep -c '^- \*\*')

read standalone valued sub_standalone sub_valued <<< "$(echo "$output" | awk '
/^- Allowed standalone flags:/ {
    s = $0; sub(/^- Allowed standalone flags: /, "", s)
    n = split(s, arr, ", "); standalone += n
}
/^- Allowed valued flags:/ {
    s = $0; sub(/^- Allowed valued flags: /, "", s)
    n = split(s, arr, ", "); valued += n
}
/^- \*\*/ {
    s = $0
    if (index(s, "Flags: ") > 0) {
        t = s; sub(/.*Flags: /, "", t); sub(/\..*/, "", t)
        n = split(t, arr, ", "); sub_standalone += n
    }
    if (index(s, "Valued: ") > 0) {
        t = s; sub(/.*Valued: /, "", t); sub(/\..*/, "", t)
        n = split(t, arr, ", "); sub_valued += n
    }
}
END {
    printf "%d %d %d %d", standalone, valued, sub_standalone, sub_valued
}
')"

total_standalone=$((standalone + sub_standalone))
total_valued=$((valued + sub_valued))
total_flags=$((total_standalone + total_valued))

unique_flags=$(echo "$output" | grep -oE -- '--[a-zA-Z][a-zA-Z0-9_-]*|-[a-zA-Z]\b' | sort -u | wc -l | tr -d ' ')

format_num() {
    printf "%'d" "$1" 2>/dev/null || printf "%d" "$1"
}

echo "## Safe-chains Coverage"
echo ""
printf "| %-26s | %10s |\n" "Metric" "Count"
printf "| %-26s | %10s |\n" "--------------------------" "----------"
printf "| %-26s | %10s |\n" "Commands"                   "$(format_num $commands)"
printf "| %-26s | %10s |\n" "Subcommands"                "$(format_num $subcommands)"
printf "| %-26s | %10s |\n" "Standalone flag entries"    "$(format_num $total_standalone)"
printf "| %-26s | %10s |\n" "Valued flag entries"        "$(format_num $total_valued)"
printf "| %-26s | %10s |\n" "**Total flag entries**"     "$(format_num $total_flags)"
printf "| %-26s | %10s |\n" "Unique flag names"          "$(format_num $unique_flags)"
