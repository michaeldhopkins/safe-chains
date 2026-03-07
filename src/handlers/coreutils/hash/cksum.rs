use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static CKSUM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--base64", "--check", "--raw", "--strict",
        "--tag", "--untagged", "--warn", "--zero",
        "-c", "-w", "-z",
    ]),
    valued: WordSet::flags(&["--algorithm", "--length", "-a", "-l"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "cksum", policy: &CKSUM_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#cksum-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        cksum_file: "cksum file.txt",
    }
}
