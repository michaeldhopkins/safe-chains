use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static STAT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--dereference", "--file-system", "--terse",
        "-F", "-L", "-l", "-n", "-q", "-r", "-s", "-x",
    ]),
    standalone_short: b"FLlnqrsx",
    valued: WordSet::new(&[
        "--format", "--printf",
        "-c", "-f", "-t",
    ]),
    valued_short: b"cft",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "stat", policy: &STAT_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        stat_file: "stat file.txt",
        stat_format: "stat -c '%s' file.txt",
    }
}
