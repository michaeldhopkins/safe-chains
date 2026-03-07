use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static GETCONF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["-a"]),
    standalone_short: b"a",
    valued: WordSet::flags(&["-v"]),
    valued_short: b"v",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "getconf", policy: &GETCONF_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/getconf.1.html" },
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
