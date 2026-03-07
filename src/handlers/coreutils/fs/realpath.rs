use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static REALPATH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--canonicalize-existing", "--canonicalize-missing",
        "--logical", "--no-symlinks", "--physical", "--quiet",
        "--strip", "--zero",
        "-L", "-P", "-e", "-m", "-q", "-s", "-z",
    ]),
    valued: WordSet::flags(&["--relative-base", "--relative-to"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "realpath", policy: &REALPATH_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#realpath-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        realpath_path: "realpath ./relative",
    }
}
