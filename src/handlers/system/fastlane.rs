use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static ACTION_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: Some(1),
    flag_style: FlagStyle::Strict,
};

pub(crate) static FASTLANE: CommandDef = CommandDef {
    name: "fastlane",
    subs: &[
        SubDef::Policy { name: "action", policy: &ACTION_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "actions", policy: &BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "env", policy: &BARE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "lanes", policy: &BARE_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://docs.fastlane.tools/",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        fastlane_lanes: "fastlane lanes",
        fastlane_actions: "fastlane actions",
        fastlane_action: "fastlane action deliver",
        fastlane_env: "fastlane env",
        fastlane_help: "fastlane --help",
        fastlane_version: "fastlane --version",
    }

    denied! {
        fastlane_bare_denied: "fastlane",
        fastlane_run_lane_denied: "fastlane beta",
        fastlane_supply_denied: "fastlane supply",
        fastlane_init_denied: "fastlane init",
        fastlane_action_two_args_denied: "fastlane action deliver extra",
    }
}
