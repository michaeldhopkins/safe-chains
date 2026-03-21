use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SIZE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--common", "--help", "--totals", "--version",
        "-A", "-B", "-G", "-V", "-d", "-h", "-o", "-t", "-x",
    ]),
    valued: WordSet::flags(&[
        "--format", "--radix", "--target",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "size", policy: &SIZE_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/size.1.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        size_file: "size binary.o",
    }
}
