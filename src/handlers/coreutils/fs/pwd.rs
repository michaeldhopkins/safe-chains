use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static PWD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--version", "-L", "-P", "-V", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "pwd", policy: &PWD_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#pwd-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        pwd_bare: "pwd",
        pwd_logical: "pwd -L",
    }
}
