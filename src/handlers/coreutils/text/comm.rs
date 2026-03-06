use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static COMM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--check-order", "--nocheck-order", "--total", "--zero-terminated",
        "-1", "-2", "-3", "-i", "-z",
    ]),
    standalone_short: b"123iz",
    valued: WordSet::new(&["--output-delimiter"]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "comm", policy: &COMM_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        comm_default: "comm file1 file2",
        comm_suppress: "comm -23 file1 file2",
        comm_combined: "comm -12 file1 file2",
    }
}
