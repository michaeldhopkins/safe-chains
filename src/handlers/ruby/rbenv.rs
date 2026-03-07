use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static RBENV_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static RBENV: CommandDef = CommandDef {
    name: "rbenv",
    subs: &[
        SubDef::Policy { name: "help", policy: &RBENV_BARE_POLICY },
        SubDef::Policy { name: "root", policy: &RBENV_BARE_POLICY },
        SubDef::Policy { name: "shims", policy: &RBENV_BARE_POLICY },
        SubDef::Policy { name: "version", policy: &RBENV_BARE_POLICY },
        SubDef::Policy { name: "versions", policy: &RBENV_BARE_POLICY },
        SubDef::Policy { name: "which", policy: &RBENV_BARE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://github.com/rbenv/rbenv#readme",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        rbenv_versions: "rbenv versions",
        rbenv_version: "rbenv version",
        rbenv_which: "rbenv which ruby",
        rbenv_root: "rbenv root",
        rbenv_shims: "rbenv shims",
        rbenv_version_flag: "rbenv --version",
        rbenv_help: "rbenv help",
    }
}
