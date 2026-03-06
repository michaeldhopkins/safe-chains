use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static WHEREIS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-b", "-l", "-m", "-s", "-u"]),
    standalone_short: b"blmsu",
    valued: WordSet::new(&["-B", "-M", "-S", "-f"]),
    valued_short: b"BMSf",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "whereis", policy: &WHEREIS_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        whereis_cmd: "whereis ls",
    }
}
