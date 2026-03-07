use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static UNSET_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["-f", "-n", "-v"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "unset", policy: &UNSET_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/unset.1p.html" },
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
