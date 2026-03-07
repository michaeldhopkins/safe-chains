use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static HTOP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--no-color", "--no-mouse", "--no-unicode", "--tree",
        "-C", "-H", "-M", "-t",
    ]),
    valued: WordSet::flags(&[
        "--delay", "--filter", "--highlight-changes",
        "--pid", "--sort-key", "--user",
        "-F", "-d", "-p", "-s", "-u",
    ]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "htop", policy: &HTOP_POLICY, help_eligible: false, url: "https://htop.dev/" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        htop_bare: "htop",
    }
}
