use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static AG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--ackmate", "--all-text", "--all-types", "--case-sensitive",
        "--color", "--column", "--count", "--filename",
        "--files-with-matches", "--files-without-matches", "--fixed-strings",
        "--follow", "--group", "--heading", "--help", "--hidden", "--ignore-case",
        "--invert-match", "--line-numbers", "--literal",
        "--no-break", "--no-color", "--no-filename", "--no-follow",
        "--no-group", "--no-heading", "--no-numbers", "--nobreak",
        "--nocolor", "--nofilename", "--nofollow", "--nogroup",
        "--noheading", "--nonumbers", "--null", "--numbers",
        "--one-device", "--only-matching", "--print-all-files",
        "--print-long-lines", "--search-binary", "--search-files",
        "--search-zip", "--silent", "--smart-case", "--stats",
        "--unrestricted", "--version", "--vimgrep", "--word-regexp",
        "-0", "-H", "-L", "-Q", "-S", "-U", "-V",
        "-a", "-c", "-f", "-h", "-i", "-l", "-n", "-s", "-u", "-v", "-w",
    ]),
    valued: WordSet::flags(&[
        "--after", "--before", "--context", "--depth",
        "--file-search-regex", "--ignore", "--max-count",
        "--pager", "--path-to-ignore", "--workers",
        "-A", "-B", "-C", "-G", "-g", "-m",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "ag", policy: &AG_POLICY, level: SafetyLevel::Inert, url: "https://github.com/ggreer/the_silver_searcher", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        ag_pattern: "ag pattern",
        ag_pattern_dir: "ag pattern src/",
        ag_ignore_case: "ag -i pattern",
        ag_count: "ag -c pattern",
        ag_files: "ag -l pattern",
        ag_hidden: "ag --hidden pattern",
        ag_literal: "ag -Q 'exact.match'",
        ag_context: "ag -C 3 pattern",
        ag_vimgrep: "ag --vimgrep pattern",
        ag_stats: "ag --stats pattern",
    }

    denied! {
        ag_bare: "ag",
    }
}
