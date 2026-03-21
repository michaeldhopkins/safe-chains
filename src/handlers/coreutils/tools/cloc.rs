use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static CLOC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--3", "--autoconf", "--by-file", "--by-file-by-lang", "--by-percent",
        "--categorized", "--counted", "--diff", "--diff-list-file", "--docstring-as-code",
        "--follow-links", "--force-lang-def", "--found-langs", "--git", "--help", "--hide-rate",
        "--ignored", "--include-content", "--json", "--md", "--no-autogen",
        "--no3", "--opt-match-d", "--opt-match-f", "--opt-not-match-d", "--opt-not-match-f",
        "--original-dir", "--progress-rate", "--quiet", "--sdir", "--show-ext",
        "--show-lang", "--show-os", "--show-stored-lang", "--skip-uniqueness", "--sql-append",
        "--strip-comments", "--sum-one", "--sum-reports", "--unicode", "--use-sloccount",
        "--v", "--vcs", "--version", "--xml", "--yaml",
        "-V", "-h", "-v",
    ]),
    valued: WordSet::flags(&[
        "--config", "--csv-delimiter", "--diff-alignment",
        "--diff-timeout", "--exclude-content",
        "--exclude-dir", "--exclude-ext",
        "--exclude-lang", "--exclude-list-file",
        "--force-lang", "--fullpath",
        "--include-ext", "--include-lang",
        "--lang-no-ext", "--list-file", "--match-d",
        "--match-f", "--not-match-d", "--not-match-f",
        "--out", "--read-binary-files", "--read-lang-def",
        "--report-file", "--script-lang", "--skip-archive",
        "--sql", "--sql-project", "--sql-style",
        "--timeout", "--write-lang-def",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "cloc", policy: &CLOC_POLICY, level: SafetyLevel::Inert, url: "https://github.com/AlDanial/cloc#readme", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        cloc_dir: "cloc src/",
    }
}
