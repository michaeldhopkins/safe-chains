use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static CD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--version", "-L", "-P", "-V", "-e", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(1),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "cd", policy: &CD_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/cd.1p.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        cd_dir: "cd /tmp",
        cd_bare: "cd",
    }
}
