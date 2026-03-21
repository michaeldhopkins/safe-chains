use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static XCODEBUILD_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h", "-json"]),
    valued: WordSet::flags(&["-project", "-workspace"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static XCODEBUILD_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h", "-json"]),
    valued: WordSet::flags(&[
        "-configuration", "-destination", "-project",
        "-scheme", "-sdk", "-target", "-workspace",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static XCODEBUILD_VERSION_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static XCODEBUILD: CommandDef = CommandDef {
    name: "xcodebuild",
    subs: &[
        SubDef::Policy { name: "-list", policy: &XCODEBUILD_LIST_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "-showBuildSettings", policy: &XCODEBUILD_SHOW_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "-showdestinations", policy: &XCODEBUILD_SHOW_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "-showsdks", policy: &XCODEBUILD_SHOW_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "-version", policy: &XCODEBUILD_VERSION_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://developer.apple.com/documentation/xcode/xcodebuild",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        xcodebuild_version: "xcodebuild -version",
        xcodebuild_showsdks: "xcodebuild -showsdks",
        xcodebuild_show_build_settings: "xcodebuild -showBuildSettings",
        xcodebuild_show_build_settings_scheme: "xcodebuild -showBuildSettings -scheme MyApp",
        xcodebuild_show_build_settings_json: "xcodebuild -showBuildSettings -json",
        xcodebuild_list: "xcodebuild -list",
        xcodebuild_list_project: "xcodebuild -list -project MyApp.xcodeproj",
        xcodebuild_list_json: "xcodebuild -list -json",
        xcodebuild_showdestinations: "xcodebuild -showdestinations -scheme MyApp",
    }
}
