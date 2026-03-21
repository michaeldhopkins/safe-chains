use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static CODEX_COMPLETION_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&["--shell", "-s"]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

static CODEX_FEATURES_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(crate) static CODEX: CommandDef = CommandDef {
    name: "codex",
    subs: &[
        SubDef::Policy { name: "completion", policy: &CODEX_COMPLETION_POLICY, level: SafetyLevel::Inert },
        SubDef::Nested { name: "features", subs: &[
            SubDef::Policy { name: "list", policy: &CODEX_FEATURES_LIST_POLICY, level: SafetyLevel::Inert },
        ]},
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://github.com/openai/codex",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        codex_help: "codex --help",
        codex_version: "codex --version",
        codex_completion: "codex completion",
        codex_completion_shell: "codex completion --shell bash",
        codex_features_list: "codex features list",
    }

    denied! {
        codex_bare: "codex",
        codex_exec: "codex exec 'fix the build'",
        codex_resume: "codex resume",
        codex_login: "codex login",
        codex_fork: "codex fork",
    }
}
