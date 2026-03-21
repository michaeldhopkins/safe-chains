use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static OPENCODE_MODELS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(1),
    flag_style: FlagStyle::Strict,
};

static OPENCODE_SESSION_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static OPENCODE_STATS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(crate) static OPENCODE: CommandDef = CommandDef {
    name: "opencode",
    subs: &[
        SubDef::Policy { name: "models", policy: &OPENCODE_MODELS_POLICY, level: SafetyLevel::Inert },
        SubDef::Nested { name: "session", subs: &[
            SubDef::Policy { name: "list", policy: &OPENCODE_SESSION_LIST_POLICY, level: SafetyLevel::Inert },
        ]},
        SubDef::Policy { name: "stats", policy: &OPENCODE_STATS_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://opencode.ai/docs/",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        opencode_help: "opencode --help",
        opencode_version: "opencode --version",
        opencode_models: "opencode models",
        opencode_models_provider: "opencode models anthropic",
        opencode_session_list: "opencode session list",
        opencode_stats: "opencode stats",
    }

    denied! {
        opencode_bare: "opencode",
        opencode_run: "opencode run 'fix the build'",
        opencode_serve: "opencode serve",
        opencode_upgrade: "opencode upgrade",
        opencode_uninstall: "opencode uninstall",
    }
}
