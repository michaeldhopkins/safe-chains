use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static READLINK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--canonicalize", "--canonicalize-existing",
        "--canonicalize-missing", "--no-newline", "--verbose", "--zero",
        "-e", "-f", "-m", "-n", "-v", "-z",
    ]),
    standalone_short: b"efmnvz",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "readlink", policy: &READLINK_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        readlink_path: "readlink /usr/bin/python",
        readlink_canon: "readlink -f /usr/bin/python",
    }
}
