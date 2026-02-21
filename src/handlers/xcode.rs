use std::collections::HashSet;
use std::sync::LazyLock;

static XCODEBUILD_SAFE: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "-version",
        "-showsdks",
        "-showBuildSettings",
        "-showdestinations",
        "-list",
    ])
});

pub fn is_safe_xcodebuild(tokens: &[String]) -> bool {
    tokens.len() >= 2 && XCODEBUILD_SAFE.contains(tokens[1].as_str())
}

static PLUTIL_READ_ONLY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["-lint", "-p", "-type", "-help"]));

pub fn is_safe_plutil(tokens: &[String]) -> bool {
    tokens.len() >= 2 && PLUTIL_READ_ONLY.contains(tokens[1].as_str())
}

pub fn is_safe_xcode_select(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    matches!(tokens[1].as_str(), "-p" | "--print-path" | "-v" | "--version")
}

static XCRUN_SHOW_FLAGS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "--find",
        "--show-sdk-path",
        "--show-sdk-version",
        "--show-sdk-build-version",
        "--show-sdk-platform-path",
        "--show-sdk-platform-version",
        "--show-toolchain-path",
    ])
});

pub fn is_safe_xcrun(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let mut i = 1;
    while i < tokens.len() {
        if tokens[i] == "--sdk" || tokens[i] == "--toolchain" {
            i += 2;
            continue;
        }
        if tokens[i] == "-v" || tokens[i] == "--verbose" || tokens[i] == "-l" || tokens[i] == "--log" || tokens[i] == "-n" || tokens[i] == "--no-cache" {
            i += 1;
            continue;
        }
        break;
    }
    if i >= tokens.len() {
        return false;
    }
    if XCRUN_SHOW_FLAGS.contains(tokens[i].as_str()) {
        return true;
    }
    if tokens[i] == "simctl" {
        return tokens.get(i + 1).is_some_and(|a| a == "list");
    }
    false
}

static PKGUTIL_READ_ONLY: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "--pkgs", "--packages", "--pkgs-plist",
        "--files", "--export-plist",
        "--pkg-info", "--pkg-info-plist",
        "--pkg-groups", "--groups", "--groups-plist", "--group-pkgs",
        "--file-info", "--file-info-plist",
        "--payload-files", "--check-signature",
    ])
});

pub fn is_safe_pkgutil(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    tokens[1..].iter().any(|t| PKGUTIL_READ_ONLY.contains(t.as_str()))
        && !tokens[1..].iter().any(|t| t == "--forget" || t == "--learn" || t == "--expand" || t == "--flatten")
}

static LIPO_READ_ONLY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["-info", "-detailed_info", "-archs", "-verify_arch"]));

pub fn is_safe_lipo(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    tokens[1..].iter().any(|t| LIPO_READ_ONLY.contains(t.as_str()))
        && !tokens[1..].iter().any(|t| t == "-output")
}

pub fn is_safe_codesign(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let has_display = tokens[1..].iter().any(|t| t == "-d" || t == "--display");
    let has_verify = tokens[1..].iter().any(|t| t == "-v" || t == "--verify");
    let has_sign = tokens[1..].iter().any(|t| t == "-s" || t == "--sign" || t == "--remove-signature" || t == "-f" || t == "--force");
    (has_display || has_verify) && !has_sign
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocKind};
    vec![
        CommandDoc {
            name: "xcodebuild",
            kind: DocKind::Handler,
            description: "Allowed: -version, -showsdks, -showBuildSettings, -showdestinations, -list.",
        },
        CommandDoc {
            name: "plutil",
            kind: DocKind::Handler,
            description: "Allowed: -lint, -p, -type, -help. Denied: -convert, -insert, -replace, -remove, -create.",
        },
        CommandDoc {
            name: "xcode-select",
            kind: DocKind::Handler,
            description: "Allowed: -p/--print-path, -v/--version. Denied: -s/--switch, -r/--reset, --install.",
        },
        CommandDoc {
            name: "xcrun",
            kind: DocKind::Handler,
            description: "Allowed: --find, --show-sdk-path, --show-sdk-version, --show-sdk-build-version, \
                          --show-sdk-platform-path, --show-sdk-platform-version, --show-toolchain-path, simctl list. \
                          Skips flags: --sdk/--toolchain (with arg), -v/-l/-n.",
        },
        CommandDoc {
            name: "pkgutil",
            kind: DocKind::Handler,
            description: "Allowed: --pkgs/--packages, --pkgs-plist, --files, --export-plist, --pkg-info, \
                          --pkg-groups, --groups, --group-pkgs, --file-info, --payload-files, --check-signature. \
                          Denied: --forget, --learn, --expand, --flatten.",
        },
        CommandDoc {
            name: "lipo",
            kind: DocKind::Handler,
            description: "Allowed: -info, -detailed_info, -archs, -verify_arch. Denied if -output flag present.",
        },
        CommandDoc {
            name: "codesign",
            kind: DocKind::Handler,
            description: "Allowed: -d/--display, -v/--verify. Denied if -s/--sign, --remove-signature, or -f/--force present.",
        },
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe;

    fn check(cmd: &str) -> bool {
        is_safe(cmd)
    }

    #[test]
    fn xcodebuild_version() {
        assert!(check("xcodebuild -version"));
    }

    #[test]
    fn xcodebuild_showsdks() {
        assert!(check("xcodebuild -showsdks"));
    }

    #[test]
    fn xcodebuild_show_build_settings() {
        assert!(check("xcodebuild -showBuildSettings"));
    }

    #[test]
    fn xcodebuild_list() {
        assert!(check("xcodebuild -list"));
    }

    #[test]
    fn xcodebuild_build_denied() {
        assert!(!check("xcodebuild build"));
    }

    #[test]
    fn xcodebuild_clean_denied() {
        assert!(!check("xcodebuild clean"));
    }

    #[test]
    fn plutil_lint() {
        assert!(check("plutil -lint file.plist"));
    }

    #[test]
    fn plutil_print() {
        assert!(check("plutil -p file.plist"));
    }

    #[test]
    fn plutil_type() {
        assert!(check("plutil -type keypath file.plist"));
    }

    #[test]
    fn plutil_help() {
        assert!(check("plutil -help"));
    }

    #[test]
    fn plutil_convert_denied() {
        assert!(!check("plutil -convert xml1 file.plist"));
    }

    #[test]
    fn plutil_insert_denied() {
        assert!(!check("plutil -insert key -string value file.plist"));
    }

    #[test]
    fn plutil_replace_denied() {
        assert!(!check("plutil -replace key -string value file.plist"));
    }

    #[test]
    fn plutil_remove_denied() {
        assert!(!check("plutil -remove key file.plist"));
    }

    #[test]
    fn plutil_no_args_denied() {
        assert!(!check("plutil"));
    }

    #[test]
    fn xcode_select_print_path() {
        assert!(check("xcode-select -p"));
    }

    #[test]
    fn xcode_select_print_path_long() {
        assert!(check("xcode-select --print-path"));
    }

    #[test]
    fn xcode_select_version() {
        assert!(check("xcode-select -v"));
    }

    #[test]
    fn xcode_select_switch_denied() {
        assert!(!check("xcode-select -s /Applications/Xcode.app"));
    }

    #[test]
    fn xcode_select_install_denied() {
        assert!(!check("xcode-select --install"));
    }

    #[test]
    fn xcode_select_reset_denied() {
        assert!(!check("xcode-select --reset"));
    }

    #[test]
    fn xcode_select_no_args_denied() {
        assert!(!check("xcode-select"));
    }

    #[test]
    fn xcrun_find() {
        assert!(check("xcrun --find clang"));
    }

    #[test]
    fn xcrun_show_sdk_path() {
        assert!(check("xcrun --show-sdk-path"));
    }

    #[test]
    fn xcrun_show_sdk_version() {
        assert!(check("xcrun --show-sdk-version"));
    }

    #[test]
    fn xcrun_show_sdk_platform_path() {
        assert!(check("xcrun --show-sdk-platform-path"));
    }

    #[test]
    fn xcrun_show_toolchain_path() {
        assert!(check("xcrun --show-toolchain-path"));
    }

    #[test]
    fn xcrun_sdk_flag_with_find() {
        assert!(check("xcrun --sdk iphoneos --find clang"));
    }

    #[test]
    fn xcrun_simctl_list() {
        assert!(check("xcrun simctl list"));
    }

    #[test]
    fn xcrun_simctl_delete_denied() {
        assert!(!check("xcrun simctl delete all"));
    }

    #[test]
    fn xcrun_simctl_boot_denied() {
        assert!(!check("xcrun simctl boot DEVICE_ID"));
    }

    #[test]
    fn xcrun_arbitrary_tool_denied() {
        assert!(!check("xcrun clang file.c"));
    }

    #[test]
    fn xcrun_no_args_denied() {
        assert!(!check("xcrun"));
    }

    #[test]
    fn pkgutil_pkgs() {
        assert!(check("pkgutil --pkgs"));
    }

    #[test]
    fn pkgutil_files() {
        assert!(check("pkgutil --files com.apple.pkg.CLTools_Executables"));
    }

    #[test]
    fn pkgutil_pkg_info() {
        assert!(check("pkgutil --pkg-info com.apple.pkg.CLTools_Executables"));
    }

    #[test]
    fn pkgutil_check_signature() {
        assert!(check("pkgutil --check-signature /path/to/pkg"));
    }

    #[test]
    fn pkgutil_groups() {
        assert!(check("pkgutil --groups"));
    }

    #[test]
    fn pkgutil_forget_denied() {
        assert!(!check("pkgutil --forget com.example.pkg"));
    }

    #[test]
    fn pkgutil_expand_denied() {
        assert!(!check("pkgutil --expand pkg.pkg /tmp/expanded"));
    }

    #[test]
    fn pkgutil_no_args_denied() {
        assert!(!check("pkgutil"));
    }

    #[test]
    fn lipo_info() {
        assert!(check("lipo -info /usr/bin/ls"));
    }

    #[test]
    fn lipo_detailed_info() {
        assert!(check("lipo -detailed_info binary"));
    }

    #[test]
    fn lipo_archs() {
        assert!(check("lipo -archs binary"));
    }

    #[test]
    fn lipo_verify_arch() {
        assert!(check("lipo -verify_arch x86_64 arm64 binary"));
    }

    #[test]
    fn lipo_create_denied() {
        assert!(!check("lipo -create a.o b.o -output universal.o"));
    }

    #[test]
    fn lipo_thin_denied() {
        assert!(!check("lipo -thin arm64 -output thin binary"));
    }

    #[test]
    fn lipo_no_args_denied() {
        assert!(!check("lipo"));
    }

    #[test]
    fn codesign_display() {
        assert!(check("codesign -d /Applications/Safari.app"));
    }

    #[test]
    fn codesign_display_long() {
        assert!(check("codesign --display --verbose=4 /usr/bin/ls"));
    }

    #[test]
    fn codesign_verify() {
        assert!(check("codesign -v /usr/bin/ls"));
    }

    #[test]
    fn codesign_verify_long() {
        assert!(check("codesign --verify --deep /Applications/Xcode.app"));
    }

    #[test]
    fn codesign_sign_denied() {
        assert!(!check("codesign -s - binary"));
    }

    #[test]
    fn codesign_remove_signature_denied() {
        assert!(!check("codesign --remove-signature binary"));
    }

    #[test]
    fn codesign_force_denied() {
        assert!(!check("codesign -f -s - binary"));
    }

    #[test]
    fn codesign_no_args_denied() {
        assert!(!check("codesign"));
    }
}
