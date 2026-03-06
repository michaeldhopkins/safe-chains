use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static W_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--from", "--ip-addr", "--no-current", "--no-header",
        "--old-style", "--short",
        "-f", "-h", "-i", "-o", "-s", "-u",
    ]),
    standalone_short: b"fhiosu",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(1),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "w", policy: &W_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        w_bare: "w",
        w_short: "w -s",
    }
}
