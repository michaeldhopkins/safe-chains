use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static ID_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--context", "--group", "--groups", "--name",
        "--real", "--user", "--zero",
        "-G", "-Z", "-g", "-n", "-p", "-r", "-u", "-z",
    ]),
    standalone_short: b"GZgnpruz",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(1),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "id", policy: &ID_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        id_bare: "id",
        id_user: "id -u",
        id_name: "id -un",
    }
}
