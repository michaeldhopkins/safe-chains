use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static PROCS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--no-header", "--or", "--tree", "--watch-interval",
        "-l", "-t",
    ]),
    standalone_short: b"lt",
    valued: WordSet::new(&[
        "--color", "--completion", "--config", "--gen-completion",
        "--insert", "--only", "--pager", "--sorta", "--sortd",
        "--theme",
        "-i", "-w",
    ]),
    valued_short: b"iw",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "procs", policy: &PROCS_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        procs_bare: "procs",
    }
}
