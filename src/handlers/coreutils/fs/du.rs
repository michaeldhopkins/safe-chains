use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DU_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--apparent-size", "--bytes", "--count-links",
        "--dereference", "--dereference-args", "--human-readable",
        "--inodes", "--no-dereference", "--null",
        "--one-file-system", "--separate-dirs", "--si",
        "--summarize", "--total",
        "-0", "-D", "-H", "-L", "-P", "-S", "-a", "-b",
        "-c", "-h", "-k", "-l", "-m", "-s", "-x",
    ]),
    standalone_short: b"0DHLPSabchklmsx",
    valued: WordSet::flags(&[
        "--block-size", "--exclude", "--files0-from",
        "--max-depth", "--threshold", "--time",
        "--time-style",
        "-B", "-d", "-t",
    ]),
    valued_short: b"Bdt",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "du", policy: &DU_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#du-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        du_dir: "du -sh /tmp",
        du_human: "du -sh /tmp",
        du_depth: "du -d 1 .",
    }
}
