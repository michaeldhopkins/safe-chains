use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static CKSUM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--base64", "--check", "--raw", "--strict",
        "--tag", "--untagged", "--warn", "--zero",
        "-c", "-w", "-z",
    ]),
    standalone_short: b"cwz",
    valued: WordSet::new(&["--algorithm", "--length", "-a", "-l"]),
    valued_short: b"al",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "cksum", policy: &CKSUM_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        cksum_file: "cksum file.txt",
    }
}
