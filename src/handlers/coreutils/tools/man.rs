use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static MAN_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--apropos", "--default", "--help", "--local-file",
        "--regex", "--update", "--version", "--whatis", "--where", "--where-cat",
        "--wildcard",
        "-V", "-a", "-f", "-h", "-k", "-l", "-u", "-w",
    ]),
    valued: WordSet::flags(&[
        "--config-file", "--encoding", "--extension", "--locale",
        "--manpath", "--sections", "--systems",
        "-C", "-E", "-L", "-M", "-S", "-e", "-m",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "man", policy: &MAN_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/man.1.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        man_page: "man ls",
        man_section: "man 3 printf",
        man_keyword_search: "man -k printf",
        man_whatis: "man -f ls",
        man_all: "man -a ls",
        man_sections_flag: "man -S 1:8 intro",
        man_where: "man --where ls",
        man_where_short: "man -w ls",
        man_local_file: "man -l /usr/share/man/man1/ls.1",
        man_manpath: "man -M /usr/share/man ls",
        man_encoding: "man -E utf-8 ls",
    }

    denied! {
        man_bare_denied: "man",
        man_pager_denied: "man -P /bin/evil ls",
        man_pager_long_denied: "man --pager evil ls",
        man_html_denied: "man -H ls",
        man_preprocessor_denied: "man -p tbl ls",
    }
}
