use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static HEAD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--quiet", "--silent", "--verbose", "--version", "--zero-terminated",
        "-0", "-1", "-2", "-3", "-4", "-5", "-6", "-7", "-8", "-9",
        "-V", "-h", "-q", "-v", "-z",
    ]),
    valued: WordSet::flags(&[
        "--bytes", "--lines",
        "-c", "-n",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "head", policy: &HEAD_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#head-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        head_default: "head file.txt",
        head_lines: "head -n 20 file.txt",
        head_bytes: "head -c 100 file.txt",
        head_numeric: "head -5 file.txt",
        head_bare: "head",
        head_quiet: "head -q file1 file2",
    }
}
