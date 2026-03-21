use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static AIDER_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--check-update", "--help", "--just-check-update", "--list-models", "--models",
        "--show-prompts", "--show-release-notes", "--show-repo-map", "--version",
        "-V", "-h",
    ]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::ai) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "aider", policy: &AIDER_POLICY, level: SafetyLevel::Inert, url: "https://aider.chat/docs/", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        aider_version: "aider --version",
        aider_help: "aider --help",
        aider_list_models: "aider --list-models",
        aider_models: "aider --models",
        aider_show_repo_map: "aider --show-repo-map",
        aider_show_prompts: "aider --show-prompts",
        aider_check_update: "aider --check-update",
        aider_just_check_update: "aider --just-check-update",
        aider_show_release_notes: "aider --show-release-notes",
    }

    denied! {
        aider_bare: "aider",
        aider_message: "aider --message 'fix bug'",
        aider_yes_always: "aider --yes-always",
        aider_file: "aider file.py",
        aider_upgrade: "aider --upgrade",
    }
}
