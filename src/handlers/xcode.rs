use crate::command::{CommandDef, SubDef};
use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

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

static PLUTIL_LINT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["-s"]),
    standalone_short: b"s",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static PLUTIL_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static PLUTIL: CommandDef = CommandDef {
    name: "plutil",
    subs: &[
        SubDef::Policy { name: "-lint", policy: &PLUTIL_LINT_POLICY },
        SubDef::Policy { name: "-p", policy: &PLUTIL_SIMPLE_POLICY },
        SubDef::Policy { name: "-type", policy: &PLUTIL_SIMPLE_POLICY },
    ],
    bare_flags: &["-help"],
    help_eligible: true,
};

pub(crate) static XCODE_SELECT: CommandDef = CommandDef {
    name: "xcode-select",
    subs: &[],
    bare_flags: &["-p", "--print-path", "-v", "--version"],
    help_eligible: true,
};

static XCRUN_SHOW_FLAGS: WordSet = WordSet::new(&[
    "--find", "--show-sdk-build-version", "--show-sdk-path",
    "--show-sdk-platform-path", "--show-sdk-platform-version",
    "--show-sdk-version", "--show-toolchain-path",
]);

static NOTARYTOOL_SAFE: WordSet = WordSet::new(&["history", "info", "log"]);

pub fn is_safe_xcrun(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let mut i = 1;
    while i < tokens.len() {
        let t = &tokens[i];
        if t == "--sdk" || t == "--toolchain" {
            i += 2;
            continue;
        }
        if t.is_one_of(&["-v", "--verbose", "-l", "--log", "-n", "--no-cache"]) {
            i += 1;
            continue;
        }
        break;
    }
    if i >= tokens.len() {
        return false;
    }
    if XCRUN_SHOW_FLAGS.contains(&tokens[i]) {
        return true;
    }
    if tokens[i] == "simctl" {
        return tokens.get(i + 1).is_some_and(|a| a == "list");
    }
    if tokens[i] == "stapler" {
        return tokens.get(i + 1).is_some_and(|a| a == "validate");
    }
    if tokens[i] == "notarytool" {
        return tokens.get(i + 1).is_some_and(|a| NOTARYTOOL_SAFE.contains(a));
    }
    false
}

static PKGUTIL_SAFE: WordSet = WordSet::new(&[
    "--check-signature", "--export-plist",
    "--file-info", "--file-info-plist",
    "--files", "--group-pkgs", "--groups", "--groups-plist",
    "--packages", "--payload-files",
    "--pkg-groups", "--pkg-info", "--pkg-info-plist",
    "--pkgs", "--pkgs-plist",
]);

static PKGUTIL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--check-signature", "--export-plist",
        "--file-info", "--file-info-plist",
        "--files", "--group-pkgs", "--groups", "--groups-plist",
        "--packages", "--payload-files",
        "--pkg-groups", "--pkg-info", "--pkg-info-plist",
        "--pkgs", "--pkgs-plist",
        "--regexp",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&["--volume"]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_pkgutil(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if !tokens[1..].iter().any(|t| PKGUTIL_SAFE.contains(t)) {
        return false;
    }
    policy::check(tokens, &PKGUTIL_POLICY)
}

static LIPO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-archs", "-detailed_info", "-info", "-verify_arch",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_lipo(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    static LIPO_SAFE: WordSet =
        WordSet::new(&["-archs", "-detailed_info", "-info", "-verify_arch"]);
    if !tokens[1..].iter().any(|t| LIPO_SAFE.contains(t)) {
        return false;
    }
    policy::check(tokens, &LIPO_POLICY)
}

static CODESIGN_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--deep", "--display", "--verify",
        "-R", "-d", "-v",
    ]),
    standalone_short: b"Rdv",
    valued: WordSet::new(&["--verbose"]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_codesign(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    static CODESIGN_SAFE: WordSet = WordSet::new(&["--display", "--verify", "-d", "-v"]);
    if !tokens[1..].iter().any(|t| CODESIGN_SAFE.contains(t)) {
        return false;
    }
    policy::check(tokens, &CODESIGN_POLICY)
}

static SPCTL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--assess", "--verbose",
        "-a", "-v",
    ]),
    standalone_short: b"av",
    valued: WordSet::new(&[
        "--context", "--type",
    ]),
    valued_short: b"t",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_spctl(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    static SPCTL_SAFE: WordSet = WordSet::new(&["--assess", "-a"]);
    if !tokens[1..].iter().any(|t| SPCTL_SAFE.contains(t)) {
        return false;
    }
    policy::check(tokens, &SPCTL_POLICY)
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    XCODEBUILD.dispatch(cmd, tokens, is_safe)
        .or_else(|| PLUTIL.dispatch(cmd, tokens, is_safe))
        .or_else(|| XCODE_SELECT.dispatch(cmd, tokens, is_safe))
        .or_else(|| match cmd {
            "xcrun" => Some(is_safe_xcrun(tokens)),
            "pkgutil" => Some(is_safe_pkgutil(tokens)),
            "lipo" => Some(is_safe_lipo(tokens)),
            "codesign" => Some(is_safe_codesign(tokens)),
            "spctl" => Some(is_safe_spctl(tokens)),
            _ => None,
        })
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        XCODEBUILD.to_doc(),
        PLUTIL.to_doc(),
        XCODE_SELECT.to_doc(),
        CommandDoc::handler("xcrun",
            "Allowed: --find, --show-sdk-*, --show-toolchain-path. \
             Multi-level: notarytool history/info/log, simctl list, stapler validate. \
             Prefix flags --sdk/--toolchain (with arg), -v/-l/-n are skipped."),
        CommandDoc::handler("pkgutil",
            "Requires a read-only flag (--pkgs, --files, --pkg-info, etc.)."),
        CommandDoc::handler("lipo",
            "Requires a read-only flag (-info, -archs, -detailed_info, -verify_arch)."),
        CommandDoc::handler("codesign",
            "Requires --display/-d or --verify/-v."),
        CommandDoc::handler("spctl",
            "Requires --assess/-a."),
    ]
}

#[cfg(test)]
pub(super) const REGISTRY: &[super::CommandEntry] = &[
    super::CommandEntry::Positional { cmd: "xcrun" },
    super::CommandEntry::Custom { cmd: "pkgutil", valid_prefix: Some("pkgutil --pkgs") },
    super::CommandEntry::Custom { cmd: "lipo", valid_prefix: Some("lipo -info /usr/bin/ls") },
    super::CommandEntry::Custom { cmd: "codesign", valid_prefix: Some("codesign -d /usr/bin/ls") },
    super::CommandEntry::Custom { cmd: "spctl", valid_prefix: Some("spctl --assess /tmp/binary") },
];

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
        plutil_lint: "plutil -lint file.plist",
        plutil_lint_silent: "plutil -lint -s file.plist",
        plutil_print: "plutil -p file.plist",
        plutil_type: "plutil -type keypath file.plist",
        plutil_help: "plutil -help",
        xcode_select_print_path: "xcode-select -p",
        xcode_select_print_path_long: "xcode-select --print-path",
        xcode_select_version: "xcode-select -v",
        xcrun_find: "xcrun --find clang",
        xcrun_show_sdk_path: "xcrun --show-sdk-path",
        xcrun_show_sdk_version: "xcrun --show-sdk-version",
        xcrun_show_sdk_platform_path: "xcrun --show-sdk-platform-path",
        xcrun_show_toolchain_path: "xcrun --show-toolchain-path",
        xcrun_sdk_flag_with_find: "xcrun --sdk iphoneos --find clang",
        xcrun_simctl_list: "xcrun simctl list",
        xcrun_stapler_validate: "xcrun stapler validate /tmp/app",
        xcrun_notarytool_history: "xcrun notarytool history",
        xcrun_notarytool_info: "xcrun notarytool info abc-123",
        xcrun_notarytool_log: "xcrun notarytool log abc-123",
        spctl_assess: "spctl --assess -v /tmp/binary",
        spctl_assess_short: "spctl -a /tmp/binary",
        spctl_assess_type: "spctl --assess --type execute -v /tmp/binary",
        pkgutil_pkgs: "pkgutil --pkgs",
        pkgutil_files: "pkgutil --files com.apple.pkg.CLTools_Executables",
        pkgutil_pkg_info: "pkgutil --pkg-info com.apple.pkg.CLTools_Executables",
        pkgutil_check_signature: "pkgutil --check-signature /path/to/pkg",
        pkgutil_groups: "pkgutil --groups",
        lipo_info: "lipo -info /usr/bin/ls",
        lipo_detailed_info: "lipo -detailed_info binary",
        lipo_archs: "lipo -archs binary",
        lipo_verify_arch: "lipo -verify_arch x86_64 arm64 binary",
        codesign_display: "codesign -d /Applications/Safari.app",
        codesign_display_long: "codesign --display --verbose=4 /usr/bin/ls",
        codesign_verify: "codesign -v /usr/bin/ls",
        codesign_verify_long: "codesign --verify --deep /usr/bin/ls",
    }

    denied! {
        xcode_select_switch_denied: "xcode-select -s /Applications/Xcode.app",
        xcode_select_install_denied: "xcode-select --install",
        xcode_select_reset_denied: "xcode-select --reset",
        xcode_select_no_args_denied: "xcode-select",
        xcrun_simctl_delete_denied: "xcrun simctl delete all",
        xcrun_simctl_boot_denied: "xcrun simctl boot DEVICE_ID",
        xcrun_arbitrary_tool_denied: "xcrun clang file.c",
        xcrun_no_args_denied: "xcrun",
        xcrun_stapler_staple_denied: "xcrun stapler staple /tmp/app",
        xcrun_notarytool_submit_denied: "xcrun notarytool submit app.zip",
        xcrun_notarytool_bare_denied: "xcrun notarytool",
        spctl_add_denied: "spctl --add /tmp/binary",
        spctl_remove_denied: "spctl --remove /tmp/binary",
        spctl_enable_denied: "spctl --enable",
        spctl_master_disable_denied: "spctl --master-disable",
        spctl_no_args_denied: "spctl",
        pkgutil_forget_denied: "pkgutil --forget com.example.pkg",
        pkgutil_expand_denied: "pkgutil --expand pkg.pkg /tmp/expanded",
        pkgutil_no_args_denied: "pkgutil",
        lipo_create_denied: "lipo -create a.o b.o -output universal.o",
        lipo_thin_denied: "lipo -thin arm64 -output thin binary",
        lipo_no_args_denied: "lipo",
        codesign_sign_denied: "codesign -s - binary",
        codesign_remove_signature_denied: "codesign --remove-signature binary",
        codesign_force_denied: "codesign -f -s - binary",
        codesign_no_args_denied: "codesign",
    }
}
