use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static REALPATH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--canonicalize-existing", "--canonicalize-missing",
        "--help", "--logical", "--no-symlinks", "--physical", "--quiet",
        "--strip", "--version", "--zero",
        "-L", "-P", "-V", "-e", "-h", "-m", "-q", "-s", "-z",
    ]),
    valued: WordSet::flags(&["--relative-base", "--relative-to"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "realpath", policy: &REALPATH_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#realpath-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        realpath_path: "realpath ./relative",
    }
}
