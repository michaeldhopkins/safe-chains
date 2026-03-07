use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SIMCTL_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json", "--verbose", "-j", "-v"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static SIMCTL: CommandDef = CommandDef {
    name: "simctl",
    subs: &[
        SubDef::Policy { name: "list", policy: &SIMCTL_LIST_POLICY },
    ],
    bare_flags: &[],
    help_eligible: false,
    url: "https://developer.apple.com/documentation/xcode/simctl",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        simctl_list: "simctl list",
        simctl_list_devices: "simctl list devices",
        simctl_list_runtimes: "simctl list runtimes",
        simctl_list_json: "simctl list --json",
        simctl_list_json_short: "simctl list -j",
        simctl_list_verbose: "simctl list --verbose",
        simctl_list_verbose_short: "simctl list -v",
        simctl_list_available: "simctl list devices available",
    }

    denied! {
        simctl_bare_denied: "simctl",
        simctl_delete_denied: "simctl delete all",
        simctl_boot_denied: "simctl boot DEVICE_ID",
        simctl_create_denied: "simctl create TestDevice",
        simctl_erase_denied: "simctl erase all",
    }
}
