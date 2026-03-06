use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static HEAD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--quiet", "--silent", "--verbose", "--zero-terminated",
        "-q", "-v", "-z",
    ]),
    standalone_short: b"0123456789qvz",
    valued: WordSet::new(&[
        "--bytes", "--lines",
        "-c", "-n",
    ]),
    valued_short: b"cn",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "head", policy: &HEAD_POLICY, help_eligible: false },
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
