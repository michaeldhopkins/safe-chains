use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static CMP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--print-bytes", "--quiet", "--silent", "--verbose", "--version",
        "-V", "-b", "-h", "-l", "-s",
    ]),
    valued: WordSet::flags(&[
        "--bytes", "--ignore-initial",
        "-i", "-n",
    ]),
    bare: false,
    max_positional: Some(2),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "cmp", policy: &CMP_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/cmp.1.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        cmp_two_files: "cmp file1.txt file2.txt",
        cmp_silent: "cmp -s file1.txt file2.txt",
        cmp_verbose: "cmp -l file1.txt file2.txt",
        cmp_bytes: "cmp -n 100 file1.txt file2.txt",
    }

    denied! {
        cmp_bare: "cmp",
        cmp_too_many_args: "cmp a b c",
    }
}
