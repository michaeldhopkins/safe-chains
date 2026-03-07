use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static PRINTENV_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--null", "-0"]),
    standalone_short: b"0",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "printenv", policy: &PRINTENV_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#printenv-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        printenv_bare: "printenv",
        printenv_var: "printenv HOME",
        printenv_null: "printenv -0",
    }
}
