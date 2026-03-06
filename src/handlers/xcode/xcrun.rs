use crate::parse::{Segment, Token, WordSet};

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

pub(in crate::handlers::xcode) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    if cmd == "xcrun" {
        Some(is_safe_xcrun(tokens))
    } else {
        None
    }
}

pub(in crate::handlers::xcode) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("xcrun",
            "Allowed: --find, --show-sdk-*, --show-toolchain-path. \
             Multi-level: notarytool history/info/log, simctl list, stapler validate. \
             Prefix flags --sdk/--toolchain (with arg), -v/-l/-n are skipped."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::xcode) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Positional { cmd: "xcrun" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
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
    }

    denied! {
        xcrun_simctl_delete_denied: "xcrun simctl delete all",
        xcrun_simctl_boot_denied: "xcrun simctl boot DEVICE_ID",
        xcrun_arbitrary_tool_denied: "xcrun clang file.c",
        xcrun_no_args_denied: "xcrun",
        xcrun_stapler_staple_denied: "xcrun stapler staple /tmp/app",
        xcrun_notarytool_submit_denied: "xcrun notarytool submit app.zip",
        xcrun_notarytool_bare_denied: "xcrun notarytool",
    }
}
