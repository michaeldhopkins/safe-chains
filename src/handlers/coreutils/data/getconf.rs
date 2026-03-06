use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static GETCONF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-a"]),
    standalone_short: b"a",
    valued: WordSet::new(&["-v"]),
    valued_short: b"v",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "getconf", policy: &GETCONF_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        getconf_bare: "getconf",
        getconf_var: "getconf PAGE_SIZE",
    }
}
