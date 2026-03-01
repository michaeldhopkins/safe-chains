use crate::parse::{FlagCheck, Token, WordSet};

static XCODEBUILD_SAFE: WordSet = WordSet::new(&[
    "-list", "-showBuildSettings", "-showdestinations", "-showsdks", "-version",
]);

pub fn is_safe_xcodebuild(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && XCODEBUILD_SAFE.contains(&tokens[1])
}

static PLUTIL_READ_ONLY: WordSet =
    WordSet::new(&["-help", "-lint", "-p", "-type"]);

pub fn is_safe_plutil(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && PLUTIL_READ_ONLY.contains(&tokens[1])
}

pub fn is_safe_xcode_select(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    tokens[1].is_one_of(&["-p", "--print-path", "-v", "--version"])
}

static XCRUN_SHOW_FLAGS: WordSet = WordSet::new(&[
    "--find", "--show-sdk-build-version", "--show-sdk-path",
    "--show-sdk-platform-path", "--show-sdk-platform-version",
    "--show-sdk-version", "--show-toolchain-path",
]);

pub fn is_safe_xcrun(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let mut i = 1;
    while i < tokens.len() {
        if tokens[i] == "--sdk" || tokens[i] == "--toolchain" {
            i += 2;
            continue;
        }
        if tokens[i].is_one_of(&["-v", "--verbose", "-l", "--log", "-n", "--no-cache"]) {
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
    false
}

static PKGUTIL_CHECK: FlagCheck = FlagCheck::new(
    &[
        "--check-signature", "--export-plist",
        "--file-info", "--file-info-plist",
        "--files", "--group-pkgs", "--groups", "--groups-plist",
        "--packages", "--payload-files",
        "--pkg-groups", "--pkg-info", "--pkg-info-plist",
        "--pkgs", "--pkgs-plist",
    ],
    &["--expand", "--flatten", "--forget", "--learn"],
);

pub fn is_safe_pkgutil(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    PKGUTIL_CHECK.is_safe(&tokens[1..])
}

static LIPO_CHECK: FlagCheck = FlagCheck::new(
    &["-archs", "-detailed_info", "-info", "-verify_arch"],
    &["-output"],
);

pub fn is_safe_lipo(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    LIPO_CHECK.is_safe(&tokens[1..])
}

static CODESIGN_CHECK: FlagCheck = FlagCheck::new(
    &["--display", "--verify", "-d", "-v"],
    &["--force", "--remove-signature", "--sign", "-f", "-s"],
);

pub fn is_safe_codesign(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    CODESIGN_CHECK.is_safe(&tokens[1..])
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, doc};
    vec![
        CommandDoc::wordset("xcodebuild", &XCODEBUILD_SAFE),
        CommandDoc::wordset("plutil", &PLUTIL_READ_ONLY),
        CommandDoc::handler("xcode-select",
            "Allowed: -p/--print-path, -v/--version. Denied: -s/--switch, -r/--reset, --install."),
        CommandDoc::handler("xcrun",
            doc(&XCRUN_SHOW_FLAGS)
                .multi_word(&[("simctl", WordSet::new(&["list"]))])
                .section("Skips flags: --sdk/--toolchain (with arg), -v/-l/-n.")
                .build()),
        CommandDoc::flagcheck("pkgutil", &PKGUTIL_CHECK),
        CommandDoc::flagcheck("lipo", &LIPO_CHECK),
        CommandDoc::flagcheck("codesign", &CODESIGN_CHECK),
    ]
}

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
        xcodebuild_list: "xcodebuild -list",
        plutil_lint: "plutil -lint file.plist",
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
        codesign_verify_long: "codesign --verify --deep /Applications/Xcode.app",
    }

    denied! {
        xcodebuild_build_denied: "xcodebuild build",
        xcodebuild_clean_denied: "xcodebuild clean",
        plutil_convert_denied: "plutil -convert xml1 file.plist",
        plutil_insert_denied: "plutil -insert key -string value file.plist",
        plutil_replace_denied: "plutil -replace key -string value file.plist",
        plutil_remove_denied: "plutil -remove key file.plist",
        plutil_no_args_denied: "plutil",
        xcode_select_switch_denied: "xcode-select -s /Applications/Xcode.app",
        xcode_select_install_denied: "xcode-select --install",
        xcode_select_reset_denied: "xcode-select --reset",
        xcode_select_no_args_denied: "xcode-select",
        xcrun_simctl_delete_denied: "xcrun simctl delete all",
        xcrun_simctl_boot_denied: "xcrun simctl boot DEVICE_ID",
        xcrun_arbitrary_tool_denied: "xcrun clang file.c",
        xcrun_no_args_denied: "xcrun",
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
