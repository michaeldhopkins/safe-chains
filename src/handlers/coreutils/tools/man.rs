use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static MAN_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--apropos", "--default", "--local-file",
        "--regex", "--update", "--whatis", "--where", "--where-cat",
        "--wildcard",
        "-a", "-f", "-k", "-l", "-u", "-w",
    ]),
    standalone_short: b"afkluw",
    valued: WordSet::flags(&[
        "--config-file", "--encoding", "--extension", "--locale",
        "--manpath", "--sections", "--systems",
        "-C", "-E", "-L", "-M", "-S", "-e", "-m",
    ]),
    valued_short: b"CELMS",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "man", policy: &MAN_POLICY, help_eligible: true, url: "https://man7.org/linux/man-pages/man1/man.1.html" },
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
