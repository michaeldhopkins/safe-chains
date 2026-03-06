use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static HEXDUMP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-C", "-b", "-c", "-d", "-o", "-v", "-x",
    ]),
    standalone_short: b"Cbcdovx",
    valued: WordSet::new(&[
        "-L", "-e", "-f", "-n", "-s",
    ]),
    valued_short: b"Lefns",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "hexdump", policy: &HEXDUMP_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        hexdump_file: "hexdump -C file.bin",
    }
}
