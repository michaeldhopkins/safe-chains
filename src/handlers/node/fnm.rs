use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static FNM_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static FNM: CommandDef = CommandDef {
    name: "fnm",
    subs: &[
        SubDef::Policy { name: "current", policy: &FNM_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "default", policy: &FNM_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "list", policy: &FNM_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "ls-remote", policy: &FNM_BARE_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://github.com/Schniz/fnm#readme",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        fnm_list: "fnm list",
        fnm_current: "fnm current",
        fnm_default: "fnm default",
        fnm_version: "fnm --version",
    }
}
