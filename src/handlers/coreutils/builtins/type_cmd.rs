use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static TYPE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["-P", "-a", "-f", "-p", "-t"]),
    standalone_short: b"Pafpt",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "type", policy: &TYPE_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/type.1p.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        type_cmd: "type ls",
        type_all: "type -a ls",
    }
}
