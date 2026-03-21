use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static NVM_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--lts", "--no-colors", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static NVM: CommandDef = CommandDef {
    name: "nvm",
    subs: &[
        SubDef::Policy { name: "current", policy: &NVM_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "list", policy: &NVM_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "ls", policy: &NVM_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "ls-remote", policy: &NVM_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "version", policy: &NVM_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "which", policy: &NVM_BARE_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://github.com/nvm-sh/nvm#readme",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        nvm_ls: "nvm ls",
        nvm_list: "nvm list",
        nvm_current: "nvm current",
        nvm_which: "nvm which 18",
        nvm_version: "nvm version",
        nvm_ls_remote: "nvm ls-remote",
        nvm_ls_remote_lts: "nvm ls-remote --lts",
    }
}
