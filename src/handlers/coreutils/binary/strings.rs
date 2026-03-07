use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static STRINGS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--include-all-whitespace", "--print-file-name",
        "-a", "-f", "-w",
    ]),
    valued: WordSet::flags(&[
        "--bytes", "--encoding", "--output-separator",
        "--radix", "--target",
        "-T", "-e", "-n", "-o", "-s", "-t",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "strings", policy: &STRINGS_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/strings.1.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        strings_file: "strings binary.exe",
        strings_bytes: "strings -n 8 binary.exe",
    }
}
