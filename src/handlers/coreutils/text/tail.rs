use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static TAIL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--follow", "--help", "--quiet", "--retry", "--silent", "--verbose",
        "--version", "--zero-terminated",
        "-0", "-1", "-2", "-3", "-4", "-5", "-6", "-7", "-8", "-9",
        "-F", "-V", "-f", "-h", "-q", "-r", "-v", "-z",
    ]),
    valued: WordSet::flags(&[
        "--bytes", "--lines", "--max-unchanged-stats", "--pid",
        "--sleep-interval",
        "-b", "-c", "-n",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "tail", policy: &TAIL_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#tail-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        tail_default: "tail file.txt",
        tail_lines: "tail -n 20 file.txt",
        tail_follow: "tail -f logfile",
        tail_follow_upper: "tail -F logfile",
        tail_numeric: "tail -20 logfile",
        tail_bare: "tail",
    }
}
