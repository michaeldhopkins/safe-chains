use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static XCODEBUILD_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-json"]),
    standalone_short: b"",
    valued: WordSet::new(&["-project", "-workspace"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static XCODEBUILD_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-json"]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "-configuration", "-destination", "-project",
        "-scheme", "-sdk", "-target", "-workspace",
    ]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static XCODEBUILD_VERSION_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static XCODEBUILD: CommandDef = CommandDef {
    name: "xcodebuild",
    subs: &[
        SubDef::Policy { name: "-list", policy: &XCODEBUILD_LIST_POLICY },
        SubDef::Policy { name: "-showBuildSettings", policy: &XCODEBUILD_SHOW_POLICY },
        SubDef::Policy { name: "-showdestinations", policy: &XCODEBUILD_SHOW_POLICY },
        SubDef::Policy { name: "-showsdks", policy: &XCODEBUILD_SHOW_POLICY },
        SubDef::Policy { name: "-version", policy: &XCODEBUILD_VERSION_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
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
