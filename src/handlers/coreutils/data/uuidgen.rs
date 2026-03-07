use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static UUIDGEN_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--random", "--time", "-r", "-t"]),
    valued: WordSet::flags(&[
        "--md5", "--name", "--namespace", "--sha1", "-N",
        "-m", "-n", "-s",
    ]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "uuidgen", policy: &UUIDGEN_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/uuidgen.1.html" },
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
