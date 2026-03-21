use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static CAT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--number", "--number-nonblank", "--show-all", "--show-ends",
        "--show-nonprinting", "--show-tabs", "--squeeze-blank", "--version",
        "-A", "-E", "-T", "-V",
        "-b", "-e", "-h", "-l", "-n", "-s", "-t", "-u", "-v",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "cat", policy: &CAT_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#cat-invocation", aliases: &[] },
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
