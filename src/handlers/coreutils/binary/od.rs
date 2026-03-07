use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static OD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--output-duplicates", "--traditional",
        "-b", "-c", "-d", "-f", "-i", "-l", "-o",
        "-s", "-v", "-x",
    ]),
    valued: WordSet::flags(&[
        "--address-radix", "--endian", "--format",
        "--read-bytes", "--skip-bytes", "--strings",
        "--width",
        "-A", "-N", "-S", "-j", "-t", "-w",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "od", policy: &OD_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#od-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        od_file: "od -x file.bin",
    }
}
