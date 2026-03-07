use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static RG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--binary", "--block-buffered", "--byte-offset", "--case-sensitive",
        "--column", "--count", "--count-matches", "--crlf", "--debug",
        "--files", "--files-with-matches", "--files-without-match",
        "--fixed-strings", "--follow", "--glob-case-insensitive", "--heading",
        "--hidden", "--ignore-case", "--ignore-file-case-insensitive",
        "--include-zero", "--invert-match", "--json", "--line-buffered",
        "--line-number", "--line-regexp", "--max-columns-preview", "--mmap",
        "--multiline", "--multiline-dotall", "--no-config", "--no-filename",
        "--no-heading", "--no-ignore", "--no-ignore-dot", "--no-ignore-exclude",
        "--no-ignore-files", "--no-ignore-global", "--no-ignore-messages",
        "--no-ignore-parent", "--no-ignore-vcs", "--no-line-number",
        "--no-messages", "--no-mmap", "--no-pcre2-unicode", "--no-require-git",
        "--no-unicode", "--null", "--null-data", "--one-file-system",
        "--only-matching", "--passthru", "--pcre2", "--pcre2-version",
        "--pretty", "--quiet", "--search-zip", "--smart-case", "--sort-files",
        "--stats", "--text", "--trim", "--type-list", "--unicode",
        "--unrestricted", "--vimgrep", "--with-filename", "--word-regexp",
        "-F", "-H", "-I", "-L", "-N", "-P", "-S", "-U", "-V",
        "-a", "-b", "-c", "-h", "-i", "-l", "-n", "-o", "-p", "-q",
        "-s", "-u", "-v", "-w", "-x", "-z",
    ]),
    standalone_short: b"FHILNPSUVabchilnopqsuvwxz",
    valued: WordSet::flags(&[
        "--after-context", "--before-context", "--color", "--colors",
        "--context", "--context-separator", "--dfa-size-limit", "--encoding",
        "--engine", "--field-context-separator", "--field-match-separator",
        "--file", "--glob", "--iglob", "--ignore-file", "--max-columns",
        "--max-count", "--max-depth", "--max-filesize", "--path-separator",
        "--regex-size-limit", "--regexp", "--replace", "--sort", "--sortr",
        "--threads", "--type", "--type-add", "--type-clear", "--type-not",
        "-A", "-B", "-C", "-E", "-M", "-T",
        "-e", "-f", "-g", "-j", "-m", "-r", "-t",
    ]),
    valued_short: b"ABCEMTefgjmrt",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "rg", policy: &RG_POLICY, help_eligible: false, url: "https://github.com/BurntSushi/ripgrep/blob/master/GUIDE.md" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        rg_pattern: "rg pattern",
        rg_fixed: "rg -F literal .",
        rg_context: "rg -A 5 -B 5 pattern",
        rg_type: "rg -t rust pattern",
        rg_glob: "rg -g '*.rs' pattern",
        rg_max_count: "rg -m 10 pattern",
        rg_replace: "rg -r replacement pattern",
        rg_json: "rg --json pattern",
        rg_multiline: "rg -U pattern",
        rg_files: "rg --files",
        rg_type_list: "rg --type-list",
        rg_combined: "rg -inl pattern .",
        rg_color: "rg --color always pattern",
        rg_threads: "rg -j 4 pattern",
    }

    denied! {
        rg_pre_denied: "rg --pre cat pattern",
        rg_pre_glob_denied: "rg --pre cat --pre-glob '*.pdf' pattern",
    }
}
