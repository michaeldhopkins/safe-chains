use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static MORE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--version",
        "-V", "-c", "-d", "-f", "-h", "-l", "-p", "-s", "-u",
    ]),
    valued: WordSet::flags(&[
        "--lines", "-n",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "more", policy: &MORE_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/more.1.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        more_file: "more file.txt",
        more_bare: "more",
        more_squeeze: "more -s file.txt",
        more_lines: "more -n 20 file.txt",
    }
}
