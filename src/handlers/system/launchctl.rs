use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static LAUNCHCTL_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static LAUNCHCTL: CommandDef = CommandDef {
    name: "launchctl",
    subs: &[
        SubDef::Policy { name: "blame", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "dumpstate", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "error", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "examine", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "help", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "hostinfo", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "list", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "print", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "print-cache", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "print-disabled", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "resolveport", policy: &LAUNCHCTL_SIMPLE_POLICY },
        SubDef::Policy { name: "version", policy: &LAUNCHCTL_SIMPLE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://ss64.com/mac/launchctl.html",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        launchctl_list: "launchctl list",
        launchctl_print: "launchctl print system",
        launchctl_blame: "launchctl blame system/com.apple.Finder",
        launchctl_version: "launchctl version",
        launchctl_help: "launchctl help",
        launchctl_hostinfo: "launchctl hostinfo",
    }
}
