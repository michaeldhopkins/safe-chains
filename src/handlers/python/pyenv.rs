use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static PYENV_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--bare", "--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static PYENV: CommandDef = CommandDef {
    name: "pyenv",
    subs: &[
        SubDef::Policy { name: "help", policy: &PYENV_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "root", policy: &PYENV_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "shims", policy: &PYENV_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "version", policy: &PYENV_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "versions", policy: &PYENV_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "which", policy: &PYENV_BARE_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://github.com/pyenv/pyenv#readme",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        pyenv_versions: "pyenv versions",
        pyenv_versions_bare: "pyenv versions --bare",
        pyenv_version: "pyenv version",
        pyenv_which: "pyenv which python",
        pyenv_root: "pyenv root",
        pyenv_shims: "pyenv shims",
        pyenv_version_flag: "pyenv --version",
        pyenv_help: "pyenv help",
    }
}
