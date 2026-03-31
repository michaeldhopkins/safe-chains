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

format_num() {
    printf "%'d" "$1" 2>/dev/null || printf "%d" "$1"
}

# Prints a markdown table from arrays of rows.
# Each row is "col1|col2|col3|...". Columns auto-size to widest element.
print_table() {
    local rows=("$@")
    local ncols
    IFS='|' read -ra first <<< "${rows[0]}"
    ncols=${#first[@]}

    local -a widths
    for ((c=0; c<ncols; c++)); do widths[$c]=0; done

    for row in "${rows[@]}"; do
        IFS='|' read -ra cols <<< "$row"
        for ((c=0; c<ncols; c++)); do
            local len=${#cols[$c]}
            if (( len > widths[$c] )); then widths[$c]=$len; fi
        done
    done

    local header="${rows[0]}"
    IFS='|' read -ra hcols <<< "$header"
    printf "|"
    for ((c=0; c<ncols; c++)); do printf " %-${widths[$c]}s |" "${hcols[$c]}"; done
    printf "\n|"
    for ((c=0; c<ncols; c++)); do printf " %${widths[$c]}s |" "" | tr ' ' '-'; done
    printf "\n"

    for ((r=1; r<${#rows[@]}; r++)); do
        IFS='|' read -ra cols <<< "${rows[$r]}"
        printf "|"
        for ((c=0; c<ncols; c++)); do printf " %-${widths[$c]}s |" "${cols[$c]}"; done
        printf "\n"
    done
}

# --- Gather data ---

rs_file_count=$(find "$SRC_DIR" -name "*.rs" | wc -l | tr -d ' ')
rs_total_lines=$(find "$SRC_DIR" -name "*.rs" -exec cat {} + | wc -l | tr -d ' ')

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

test_funcs=$(grep -r "#\[test\]" "$SRC_DIR" --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')

toml_file_count=$(find "$CMD_DIR" -name "*.toml" ! -name "SAMPLE.toml" 2>/dev/null | wc -l | tr -d ' ')
toml_total_lines=$(find "$CMD_DIR" -name "*.toml" ! -name "SAMPLE.toml" -exec cat {} + 2>/dev/null | wc -l | tr -d ' ')

command_count=$(grep -c "^### \`" "$REPO_ROOT/COMMANDS.md" 2>/dev/null || echo "0")

types=$(grep -r "^pub struct\|^struct\|^pub enum\|^enum" "$SRC_DIR" --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
deps=$(grep -A 100 "^\[dependencies\]" "$REPO_ROOT/Cargo.toml" | grep -B 100 "^\[" | grep -v "^\[" | grep -v "^#" | grep -v "^$" | grep "=" | wc -l | tr -d ' ')

if [ "$rs_total_lines" -gt 0 ]; then
    app_pct=$((app_lines * 100 / rs_total_lines))
    test_pct=$((test_lines * 100 / rs_total_lines))
else
    app_pct=0
    test_pct=0
fi
combined_lines=$((rs_total_lines + toml_total_lines))

# --- Output ---

echo "## Safe-chains Codebase Statistics"
echo ""

summary_rows=(
    "Metric|Count"
    "**Supported commands**|$command_count"
    " | "
    "**Rust source files**|$rs_file_count"
    "**Rust total lines**|$(format_num $rs_total_lines)"
    "**Rust application lines**|~$(format_num $app_lines) (${app_pct}%)"
    "**Rust test lines**|~$(format_num $test_lines) (${test_pct}%)"
    "**Test functions**|$test_funcs"
    " | "
    "**TOML command files**|$toml_file_count"
    "**TOML total lines**|$(format_num $toml_total_lines)"
    " | "
    "**Combined lines (Rust+TOML)**|$(format_num $combined_lines)"
    "**Structs/Enums**|$types"
    "**Direct dependencies**|$deps"
)
print_table "${summary_rows[@]}"
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
toml_dir_count=$(find "$CMD_DIR" -mindepth 1 -type d | wc -l | tr -d ' ')
printf "└── (%d directories, %d files, %s lines)\n" "$toml_dir_count" "$toml_file_count" "$(format_num $toml_total_lines)"
echo "\`\`\`"
echo ""

echo "### Largest Rust Files"
rs_rows=("File|Lines|Test Lines|App Lines")
while read -r lines file; do
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
        rs_rows+=("\`$relpath\`|$lines|$file_test|$file_app")
    fi
done < <(find "$SRC_DIR" -name "*.rs" -exec wc -l {} + | sort -rn | head -11 | tail -10)
print_table "${rs_rows[@]}"

echo ""
echo "### Largest TOML Files"
toml_rows=("File|Lines")
while read -r lines file; do
    if [ -n "$file" ] && [ -f "$file" ]; then
        relpath="${file#$REPO_ROOT/}"
        toml_rows+=("\`$relpath\`|$lines")
    fi
done < <(find "$CMD_DIR" -name "*.toml" ! -name "SAMPLE.toml" -exec wc -l {} + | sort -rn | head -11 | tail -10)
print_table "${toml_rows[@]}"
