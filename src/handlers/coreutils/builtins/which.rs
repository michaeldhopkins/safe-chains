use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static WHICH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--all", "-a", "-s"]),
    standalone_short: b"as",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "which", policy: &WHICH_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/which.1.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        which_cmd: "which ls",
        which_all: "which -a ls",
    }
}
