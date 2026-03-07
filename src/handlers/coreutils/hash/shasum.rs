use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SHASUM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--binary", "--check", "--portable", "--status",
        "--strict", "--tag", "--text", "--warn",
        "-0", "-b", "-c", "-p", "-s", "-t",
    ]),
    standalone_short: b"0bcpst",
    valued: WordSet::flags(&["--algorithm", "-a"]),
    valued_short: b"a",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "shasum", policy: &SHASUM_POLICY, help_eligible: false, url: "https://perldoc.perl.org/shasum" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        shasum_file: "shasum file.txt",
        shasum_algo: "shasum -a 256 file.txt",
    }
}
