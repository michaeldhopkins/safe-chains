use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static ALIAS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["-p"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Positional,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "alias", policy: &ALIAS_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/alias.1p.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        alias_bare: "alias",
        alias_name: "alias ll",
        alias_print: "alias -p",
        alias_set: "alias ll='ls -la'",
    }
}
