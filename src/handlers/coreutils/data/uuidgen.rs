use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static UUIDGEN_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--random", "--time", "-r", "-t"]),
    standalone_short: b"rt",
    valued: WordSet::new(&[
        "--md5", "--name", "--namespace", "--sha1", "-N",
        "-m", "-n", "-s",
    ]),
    valued_short: b"mnNs",
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "uuidgen", policy: &UUIDGEN_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        uuidgen_bare: "uuidgen",
        uuidgen_random: "uuidgen -r",
    }
}
