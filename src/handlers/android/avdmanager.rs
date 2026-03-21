use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--compact", "--help", "-c", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static AVDMANAGER: CommandDef = CommandDef {
    name: "avdmanager",
    subs: &[
        SubDef::Nested { name: "list", subs: &[
            SubDef::Policy { name: "avd", policy: &LIST_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "device", policy: &LIST_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "target", policy: &LIST_POLICY, level: SafetyLevel::Inert },
        ]},
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://developer.android.com/tools/avdmanager",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        avdmanager_list_avd: "avdmanager list avd",
        avdmanager_list_device: "avdmanager list device",
        avdmanager_list_target: "avdmanager list target",
        avdmanager_list_avd_compact: "avdmanager list avd -c",
        avdmanager_help: "avdmanager --help",
    }

    denied! {
        avdmanager_bare_denied: "avdmanager",
        avdmanager_create_denied: "avdmanager create avd -n test -k system-images",
        avdmanager_delete_denied: "avdmanager delete avd -n test",
        avdmanager_list_bare_denied: "avdmanager list",
    }
}
