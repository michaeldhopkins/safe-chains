use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static UNSET_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-f", "-n", "-v"]),
    standalone_short: b"fnv",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "unset", policy: &UNSET_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        unset_var: "unset FOO",
        unset_func: "unset -f myfunc",
    }
}
