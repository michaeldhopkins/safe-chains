use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static OD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--output-duplicates", "--traditional",
        "-b", "-c", "-d", "-f", "-i", "-l", "-o",
        "-s", "-v", "-x",
    ]),
    standalone_short: b"bcdfilosvx",
    valued: WordSet::new(&[
        "--address-radix", "--endian", "--format",
        "--read-bytes", "--skip-bytes", "--strings",
        "--width",
        "-A", "-N", "-S", "-j", "-t", "-w",
    ]),
    valued_short: b"ANSjtw",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "od", policy: &OD_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        od_file: "od -x file.bin",
    }
}
