use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static POSITIONAL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static FEATURES_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--not-required", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DEX_PACKAGES_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--defined-only", "--help", "-h"]),
    valued: WordSet::flags(&["--files", "--proguard-folder", "--proguard-mappings", "--proguard-seeds", "--proguard-usages"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static RESOURCES_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&["--config", "--name", "--type"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static APKANALYZER: CommandDef = CommandDef {
    name: "apkanalyzer",
    subs: &[
        SubDef::Nested { name: "apk", subs: &[
            SubDef::Policy { name: "compare", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "download-size", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "features", policy: &FEATURES_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "file-size", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "summary", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
        ]},
        SubDef::Nested { name: "dex", subs: &[
            SubDef::Policy { name: "code", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "list", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "packages", policy: &DEX_PACKAGES_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "references", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
        ]},
        SubDef::Nested { name: "files", subs: &[
            SubDef::Policy { name: "cat", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "list", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
        ]},
        SubDef::Nested { name: "manifest", subs: &[
            SubDef::Policy { name: "application-id", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "debuggable", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "min-sdk", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "permissions", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "print", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "target-sdk", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "version-code", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "version-name", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
        ]},
        SubDef::Nested { name: "resources", subs: &[
            SubDef::Policy { name: "configs", policy: &RESOURCES_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "names", policy: &RESOURCES_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "value", policy: &RESOURCES_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "xml", policy: &POSITIONAL_POLICY, level: SafetyLevel::Inert },
        ]},
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://developer.android.com/tools/apkanalyzer",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        apkanalyzer_apk_summary: "apkanalyzer apk summary app.apk",
        apkanalyzer_apk_file_size: "apkanalyzer apk file-size app.apk",
        apkanalyzer_apk_download_size: "apkanalyzer apk download-size app.apk",
        apkanalyzer_apk_features: "apkanalyzer apk features app.apk",
        apkanalyzer_apk_features_not_required: "apkanalyzer apk features --not-required app.apk",
        apkanalyzer_apk_compare: "apkanalyzer apk compare old.apk new.apk",
        apkanalyzer_manifest_print: "apkanalyzer manifest print app.apk",
        apkanalyzer_manifest_app_id: "apkanalyzer manifest application-id app.apk",
        apkanalyzer_manifest_version_name: "apkanalyzer manifest version-name app.apk",
        apkanalyzer_manifest_version_code: "apkanalyzer manifest version-code app.apk",
        apkanalyzer_manifest_min_sdk: "apkanalyzer manifest min-sdk app.apk",
        apkanalyzer_manifest_target_sdk: "apkanalyzer manifest target-sdk app.apk",
        apkanalyzer_manifest_permissions: "apkanalyzer manifest permissions app.apk",
        apkanalyzer_manifest_debuggable: "apkanalyzer manifest debuggable app.apk",
        apkanalyzer_dex_list: "apkanalyzer dex list app.apk",
        apkanalyzer_dex_references: "apkanalyzer dex references app.apk",
        apkanalyzer_dex_packages: "apkanalyzer dex packages app.apk",
        apkanalyzer_dex_code: "apkanalyzer dex code app.apk com.example.Foo",
        apkanalyzer_files_list: "apkanalyzer files list app.apk",
        apkanalyzer_files_cat: "apkanalyzer files cat app.apk AndroidManifest.xml",
        apkanalyzer_resources_names: "apkanalyzer resources names --type string app.apk",
        apkanalyzer_resources_value: "apkanalyzer resources value --name app_name app.apk",
        apkanalyzer_resources_configs: "apkanalyzer resources configs --type layout app.apk",
        apkanalyzer_resources_xml: "apkanalyzer resources xml app.apk res/layout/main.xml",
        apkanalyzer_help: "apkanalyzer --help",
        apkanalyzer_version: "apkanalyzer --version",
    }

    denied! {
        apkanalyzer_bare_denied: "apkanalyzer",
        apkanalyzer_unknown_denied: "apkanalyzer --unknown",
        apkanalyzer_apk_bare_denied: "apkanalyzer apk",
        apkanalyzer_manifest_bare_denied: "apkanalyzer manifest",
    }
}
