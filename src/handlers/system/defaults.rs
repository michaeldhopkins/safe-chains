use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DEFAULTS_READ_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-g", "-globalDomain", "-h"]),
    valued: WordSet::flags(&["-app"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DEFAULTS_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static DEFAULTS: CommandDef = CommandDef {
    name: "defaults",
    subs: &[
        SubDef::Policy { name: "read", policy: &DEFAULTS_READ_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "read-type", policy: &DEFAULTS_READ_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "export", policy: &DEFAULTS_READ_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "find", policy: &DEFAULTS_READ_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "domains", policy: &DEFAULTS_SIMPLE_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://ss64.com/mac/defaults.html",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        defaults_read: "defaults read com.apple.finder",
        defaults_read_type: "defaults read-type com.apple.finder ShowPathbar",
        defaults_domains: "defaults domains",
        defaults_find: "defaults find finder",
        defaults_export: "defaults export com.apple.finder -",
        defaults_read_global: "defaults read -g",
    }
}
