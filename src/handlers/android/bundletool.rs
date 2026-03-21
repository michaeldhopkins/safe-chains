use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static VALIDATE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&["--bundle"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DUMP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&["--bundle", "--module", "--xpath"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GET_SIZE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&["--apks", "--device-spec", "--dimensions", "--modules"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static VERSION_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(crate) static BUNDLETOOL: CommandDef = CommandDef {
    name: "bundletool",
    subs: &[
        SubDef::Nested { name: "dump", subs: &[
            SubDef::Policy { name: "config", policy: &DUMP_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "manifest", policy: &DUMP_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "resources", policy: &DUMP_POLICY, level: SafetyLevel::Inert },
        ]},
        SubDef::Nested { name: "get-size", subs: &[
            SubDef::Policy { name: "total", policy: &GET_SIZE_POLICY, level: SafetyLevel::Inert },
        ]},
        SubDef::Policy { name: "validate", policy: &VALIDATE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "version", policy: &VERSION_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://developer.android.com/tools/bundletool",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        bundletool_validate: "bundletool validate --bundle app.aab",
        bundletool_dump_manifest: "bundletool dump manifest --bundle app.aab",
        bundletool_dump_resources: "bundletool dump resources --bundle app.aab",
        bundletool_dump_config: "bundletool dump config --bundle app.aab",
        bundletool_dump_manifest_module: "bundletool dump manifest --bundle app.aab --module base",
        bundletool_get_size_total: "bundletool get-size total --apks app.apks",
        bundletool_version: "bundletool version",
        bundletool_help: "bundletool --help",
    }

    denied! {
        bundletool_bare_denied: "bundletool",
        bundletool_build_apks_denied: "bundletool build-apks --bundle app.aab --output app.apks",
        bundletool_install_apks_denied: "bundletool install-apks --apks app.apks",
        bundletool_dump_bare_denied: "bundletool dump",
        bundletool_dump_unknown_denied: "bundletool dump manifest --unknown-flag",
    }
}
