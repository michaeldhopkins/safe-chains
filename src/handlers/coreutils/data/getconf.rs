use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static GETCONF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--version", "-V", "-a", "-h"]),
    valued: WordSet::flags(&["-v"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "getconf", policy: &GETCONF_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/getconf.1.html", aliases: &[] },
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
