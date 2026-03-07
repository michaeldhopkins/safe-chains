use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SEQ_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--equal-width",
        "-w",
    ]),
    standalone_short: b"w",
    valued: WordSet::flags(&[
        "--format", "--separator",
        "-f", "-s", "-t",
    ]),
    valued_short: b"fst",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "seq", policy: &SEQ_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#seq-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        seq_range: "seq 1 10",
        seq_step: "seq 1 2 10",
        seq_format: "seq -f '%.2f' 1 0.5 5",
        seq_separator: "seq -s, 1 5",
        seq_equal_width: "seq -w 1 10",
    }
}
