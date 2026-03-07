use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static NVM_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--lts", "--no-colors"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static NVM: CommandDef = CommandDef {
    name: "nvm",
    subs: &[
        SubDef::Policy { name: "current", policy: &NVM_BARE_POLICY },
        SubDef::Policy { name: "list", policy: &NVM_BARE_POLICY },
        SubDef::Policy { name: "ls", policy: &NVM_BARE_POLICY },
        SubDef::Policy { name: "ls-remote", policy: &NVM_BARE_POLICY },
        SubDef::Policy { name: "version", policy: &NVM_BARE_POLICY },
        SubDef::Policy { name: "which", policy: &NVM_BARE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://github.com/nvm-sh/nvm#readme",
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
