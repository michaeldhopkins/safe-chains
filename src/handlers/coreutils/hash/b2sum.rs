use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static B2SUM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--binary", "--check", "--help", "--ignore-missing", "--quiet",
        "--status", "--strict", "--tag", "--text", "--version", "--warn",
        "--zero",
        "-V", "-b", "-c", "-h", "-t", "-w", "-z",
    ]),
    valued: WordSet::flags(&["--length", "-l"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "b2sum", policy: &B2SUM_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#b2sum-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        b2sum_file: "b2sum file.txt",
    }
}
