use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SHASUM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--binary", "--check", "--help", "--portable", "--status",
        "--strict", "--tag", "--text", "--version", "--warn",
        "-0", "-V", "-b", "-c", "-h", "-p", "-s", "-t",
    ]),
    valued: WordSet::flags(&["--algorithm", "-a"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "shasum", policy: &SHASUM_POLICY, level: SafetyLevel::Inert, url: "https://perldoc.perl.org/shasum", aliases: &[] },
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
