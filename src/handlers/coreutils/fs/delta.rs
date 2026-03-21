use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DELTA_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--blame-code-style", "--blame-palette",
        "--color-only", "--dark", "--diff-highlight",
        "--diff-so-fancy", "--help", "--hyperlinks", "--keep-plus-minus-markers",
        "--light", "--line-numbers", "--list-languages",
        "--list-syntax-themes", "--navigate", "--no-gitconfig",
        "--raw", "--relative-paths", "--show-config",
        "--show-syntax-themes", "--side-by-side", "--version",
        "-V", "-h", "-n", "-s",
    ]),
    valued: WordSet::flags(&[
        "--commit-decoration-style", "--commit-style", "--config",
        "--diff-stat-align-width", "--features", "--file-added-label",
        "--file-decoration-style", "--file-modified-label",
        "--file-removed-label", "--file-renamed-label",
        "--file-style", "--file-transformation",
        "--hunk-header-decoration-style", "--hunk-header-file-style",
        "--hunk-header-line-number-style", "--hunk-header-style",
        "--hunk-label", "--inline-hint-style",
        "--inspect-raw-lines", "--line-buffer-size",
        "--line-fill-method", "--line-numbers-left-format",
        "--line-numbers-left-style", "--line-numbers-minus-style",
        "--line-numbers-plus-style", "--line-numbers-right-format",
        "--line-numbers-right-style", "--line-numbers-zero-style",
        "--map-styles", "--max-line-distance", "--max-line-length",
        "--merge-conflict-begin-symbol", "--merge-conflict-end-symbol",
        "--merge-conflict-ours-diff-header-decoration-style",
        "--merge-conflict-ours-diff-header-style",
        "--merge-conflict-theirs-diff-header-decoration-style",
        "--merge-conflict-theirs-diff-header-style",
        "--minus-emph-style", "--minus-empty-line-marker-style",
        "--minus-non-emph-style", "--minus-style",
        "--paging", "--plus-emph-style",
        "--plus-empty-line-marker-style", "--plus-non-emph-style",
        "--plus-style", "--syntax-theme", "--tabs",
        "--true-color", "--whitespace-error-style", "--width",
        "-w",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "delta", policy: &DELTA_POLICY, level: SafetyLevel::Inert, url: "https://dandavison.github.io/delta/", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        delta_diff: "delta file1 file2",
        delta_files: "delta file1 file2",
    }
}
