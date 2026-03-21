use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static ACK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--color", "--column", "--count", "--files-with-matches",
        "--files-without-matches", "--flush", "--follow", "--group",
        "--heading", "--help", "--ignore-case", "--invert-match", "--line",
        "--literal", "--match", "--no-color", "--no-filename",
        "--no-follow", "--no-group", "--no-heading", "--nocolor",
        "--noenv", "--nofilter", "--nofollow", "--nogroup",
        "--noheading", "--nopager", "--nosmart-case", "--passthru",
        "--print0", "--show-types", "--smart-case", "--sort-files",
        "--version", "--with-filename", "--word-regexp",
        "-1", "-H", "-L", "-V", "-c", "-f", "-h", "-i", "-l",
        "-n", "-s", "-v", "-w", "-x",
    ]),
    valued: WordSet::flags(&[
        "--after-context", "--before-context", "--context",
        "--ignore-dir", "--max-count", "--noignore-dir",
        "--output", "--pager", "--type", "--type-add",
        "--type-del", "--type-set",
        "-A", "-B", "-C", "-m",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "ack", policy: &ACK_POLICY, level: SafetyLevel::Inert, url: "https://beyondgrep.com/documentation/", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        ack_pattern: "ack pattern",
        ack_pattern_dir: "ack pattern src/",
        ack_ignore_case: "ack -i pattern",
        ack_count: "ack -c pattern",
        ack_files: "ack -l pattern",
        ack_type: "ack --type=rust pattern",
        ack_context: "ack -C 3 pattern",
        ack_sort: "ack --sort-files pattern",
        ack_literal: "ack --literal 'exact.match'",
    }

    denied! {
        ack_bare: "ack",
    }
}
