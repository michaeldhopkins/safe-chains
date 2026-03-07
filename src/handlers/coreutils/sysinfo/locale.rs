use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static LOCALE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all-locales", "--category-name", "--charmaps",
        "--keyword-name", "--verbose",
        "-a", "-c", "-k", "-m", "-v",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "locale", policy: &LOCALE_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/locale.1.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        locale_bare: "locale",
        locale_all: "locale -a",
    }
}
