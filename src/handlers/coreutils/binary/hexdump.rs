use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static HEXDUMP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--version",
        "-C", "-V", "-b", "-c", "-d", "-h", "-o", "-v", "-x",
    ]),
    valued: WordSet::flags(&[
        "-L", "-e", "-f", "-n", "-s",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "hexdump", policy: &HEXDUMP_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/hexdump.1.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        hexdump_file: "hexdump -C file.bin",
    }
}
