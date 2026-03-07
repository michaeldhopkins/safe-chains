use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static CAT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--number", "--number-nonblank", "--show-all", "--show-ends",
        "--show-nonprinting", "--show-tabs", "--squeeze-blank",
        "-A", "-E", "-T",
        "-b", "-e", "-l", "-n", "-s", "-t", "-u", "-v",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "cat", policy: &CAT_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#cat-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        cat_file: "cat file.txt",
        cat_number: "cat -n file.txt",
        cat_bare: "cat",
        cat_show_all: "cat -A file.txt",
        cat_combined: "cat -bns file.txt",
    }
}
