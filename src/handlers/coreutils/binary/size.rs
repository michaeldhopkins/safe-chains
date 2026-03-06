use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SIZE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--common", "--totals",
        "-A", "-B", "-G", "-d", "-o", "-t", "-x",
    ]),
    standalone_short: b"ABGdotx",
    valued: WordSet::new(&[
        "--format", "--radix", "--target",
    ]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "size", policy: &SIZE_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        size_file: "size binary.o",
    }
}
