use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static IMPORTMAP_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(crate) static IMPORTMAP: CommandDef = CommandDef {
    name: "importmap",
    subs: &[
        SubDef::Policy { name: "audit", policy: &IMPORTMAP_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "json", policy: &IMPORTMAP_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "outdated", policy: &IMPORTMAP_BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "packages", policy: &IMPORTMAP_BARE_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "-h"],
    url: "https://github.com/rails/importmap-rails",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        importmap_help: "importmap --help",
        importmap_json: "importmap json",
        importmap_audit: "importmap audit",
        importmap_outdated: "importmap outdated",
        importmap_packages: "importmap packages",
        importmap_json_help: "importmap json --help",
        importmap_audit_help: "importmap audit -h",
        importmap_via_bin: "./bin/importmap json",
        importmap_via_bin_audit: "./bin/importmap audit",
    }

    denied! {
        importmap_bare_denied: "importmap",
        importmap_pin_denied: "importmap pin slim-select",
        importmap_pin_version_denied: "importmap pin slim-select@2.9.2",
        importmap_unpin_denied: "importmap unpin foo",
        importmap_update_denied: "importmap update",
        importmap_pristine_denied: "importmap pristine",
    }
}
