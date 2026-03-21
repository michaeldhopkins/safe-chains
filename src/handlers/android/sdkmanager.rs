use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SDKMANAGER_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--list", "--version", "-V", "-h"]),
    valued: WordSet::flags(&["--channel", "--sdk_root"]),
    bare: false,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub static DEFS: &[FlatDef] = &[
    FlatDef {
        name: "sdkmanager",
        policy: &SDKMANAGER_POLICY,
        level: SafetyLevel::Inert,
        url: "https://developer.android.com/tools/sdkmanager",
        aliases: &[],
    },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        sdkmanager_list: "sdkmanager --list",
        sdkmanager_version: "sdkmanager --version",
        sdkmanager_list_channel: "sdkmanager --list --channel=3",
        sdkmanager_help: "sdkmanager --help",
    }

    denied! {
        sdkmanager_bare_denied: "sdkmanager",
        sdkmanager_install_denied: "sdkmanager \"platforms;android-34\"",
        sdkmanager_update_denied: "sdkmanager --update",
        sdkmanager_uninstall_denied: "sdkmanager --uninstall \"platforms;android-34\"",
    }
}
