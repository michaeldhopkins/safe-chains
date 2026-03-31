#!/bin/bash
#
# Generate codebase statistics for safe-chains.
#
# Usage: ./scripts/stats.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SRC_DIR="$REPO_ROOT/src"
CMD_DIR="$REPO_ROOT/commands"

# Count Rust source files
rs_file_count=$(find "$SRC_DIR" -name "*.rs" | wc -l | tr -d ' ')

# Count total Rust lines
rs_total_lines=$(find "$SRC_DIR" -name "*.rs" -exec cat {} + | wc -l | tr -d ' ')

# Count test lines (everything after #[cfg(test)] in each file)
test_lines=0
for file in $(find "$SRC_DIR" -name "*.rs"); do
    test_start=$(grep -n "#\[cfg(test)\]" "$file" 2>/dev/null | head -1 | cut -d: -f1)
    if [ -n "$test_start" ]; then
        file_total=$(wc -l < "$file" | tr -d ' ')
        file_test_lines=$((file_total - test_start + 1))
        test_lines=$((test_lines + file_test_lines))
    fi
done

app_lines=$((rs_total_lines - test_lines))

# Count test functions
test_funcs=$(grep -r "#\[test\]" "$SRC_DIR" --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')

# Count TOML command files
toml_file_count=$(find "$CMD_DIR" -name "*.toml" ! -name "SAMPLE.toml" 2>/dev/null | wc -l | tr -d ' ')
toml_total_lines=$(find "$CMD_DIR" -name "*.toml" ! -name "SAMPLE.toml" -exec cat {} + 2>/dev/null | wc -l | tr -d ' ')

# Count commands in COMMANDS.md
command_count=$(grep -c "^### \`" "$REPO_ROOT/COMMANDS.md" 2>/dev/null || echo "0")

# Count structs and enums
types=$(grep -r "^pub struct\|^struct\|^pub enum\|^enum" "$SRC_DIR" --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')

# Count direct dependencies (non-dev, non-build)
deps=$(grep -A 100 "^\[dependencies\]" "$REPO_ROOT/Cargo.toml" | grep -B 100 "^\[" | grep -v "^\[" | grep -v "^#" | grep -v "^$" | grep "=" | wc -l | tr -d ' ')

# Calculate percentages
if [ "$rs_total_lines" -gt 0 ]; then
    app_pct=$((app_lines * 100 / rs_total_lines))
    test_pct=$((test_lines * 100 / rs_total_lines))
else
    app_pct=0
    test_pct=0
fi

combined_lines=$((rs_total_lines + toml_total_lines))

# Format numbers with commas
format_num() {
    printf "%'d" "$1" 2>/dev/null || printf "%d" "$1"
}

echo "## Safe-chains Codebase Statistics"
echo ""
printf "| %-28s | %-20s |\n" "Metric" "Count"
printf "| %-28s | %-20s |\n" "----------------------------" "--------------------"
printf "| %-28s | %-20s |\n" "**Supported commands**" "$command_count"
printf "| %-28s | %-20s |\n" "" ""
printf "| %-28s | %-20s |\n" "**Rust source files**" "$rs_file_count"
printf "| %-28s | %-20s |\n" "**Rust total lines**" "$(format_num $rs_total_lines)"
printf "| %-28s | %-20s |\n" "**Rust application lines**" "~$(format_num $app_lines) (${app_pct}%)"
printf "| %-28s | %-20s |\n" "**Rust test lines**" "~$(format_num $test_lines) (${test_pct}%)"
printf "| %-28s | %-20s |\n" "**Test functions**" "$test_funcs"
printf "| %-28s | %-20s |\n" "" ""
printf "| %-28s | %-20s |\n" "**TOML command files**" "$toml_file_count"
printf "| %-28s | %-20s |\n" "**TOML total lines**" "$(format_num $toml_total_lines)"
printf "| %-28s | %-20s |\n" "" ""
printf "| %-28s | %-20s |\n" "**Combined lines (Rust+TOML)**" "$(format_num $combined_lines)"
printf "| %-28s | %-20s |\n" "**Structs/Enums**" "$types"
printf "| %-28s | %-20s |\n" "**Direct dependencies**" "$deps"
echo ""

echo "### Directory Structure"
echo "\`\`\`"
echo "src/"
for dir in "$SRC_DIR"/*/; do
    if [ -d "$dir" ]; then
        dirname=$(basename "$dir")
        count=$(find "$dir" -name "*.rs" | wc -l | tr -d ' ')
        printf "├── %-16s (%d files)\n" "$dirname/" "$count"
        for subdir in "$dir"/*/; do
            if [ -d "$subdir" ]; then
                subdirname=$(basename "$subdir")
                subcount=$(find "$subdir" -name "*.rs" | wc -l | tr -d ' ')
                printf "│   └── %-12s (%d files)\n" "$subdirname/" "$subcount"
            fi
        done
    fi
done
root_count=$(find "$SRC_DIR" -maxdepth 1 -name "*.rs" | wc -l | tr -d ' ')
printf "└── (%d files in root)\n" "$root_count"
echo ""
echo "commands/"
printf "└── (%d TOML files, %s lines)\n" "$toml_file_count" "$(format_num $toml_total_lines)"
echo "\`\`\`"
echo ""

echo "### Largest Rust Files"
printf "| %-30s | %6s | %11s | %10s |\n" "File" "Lines" "Test Lines" "App Lines"
printf "| %-30s | %6s | %11s | %10s |\n" "------------------------------" "------" "-----------" "----------"

find "$SRC_DIR" -name "*.rs" -exec wc -l {} + | sort -rn | head -11 | tail -10 | while read -r lines file; do
    if [ -n "$file" ] && [ -f "$file" ]; then
        relpath="${file#$SRC_DIR/}"
        test_start=$(grep -n "#\[cfg(test)\]" "$file" 2>/dev/null | head -1 | cut -d: -f1)
        if [ -n "$test_start" ]; then
            file_test=$((lines - test_start + 1))
            file_app=$((test_start - 1))
        else
            file_test=0
            file_app=$lines
        fi
        printf "| \`%-28s\` | %6s | %11s | %10s |\n" "$relpath" "$lines" "$file_test" "$file_app"
    fi
done

echo ""
echo "### Largest TOML Files"
printf "| %-30s | %6s |\n" "File" "Lines"
printf "| %-30s | %6s |\n" "------------------------------" "------"

find "$CMD_DIR" -name "*.toml" ! -name "SAMPLE.toml" -exec wc -l {} + | sort -rn | head -11 | tail -10 | while read -r lines file; do
    if [ -n "$file" ] && [ -f "$file" ]; then
        relpath="${file#$REPO_ROOT/}"
        printf "| \`%-28s\` | %6s |\n" "$relpath" "$lines"
    fi
done
