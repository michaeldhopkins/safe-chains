use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static PWD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-L", "-P"]),
    standalone_short: b"LP",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "pwd", policy: &PWD_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        pwd_bare: "pwd",
        pwd_logical: "pwd -L",
    }
}
