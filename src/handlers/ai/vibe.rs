use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static VIBE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--version", "-V", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::ai) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "vibe", policy: &VIBE_POLICY, level: SafetyLevel::Inert, url: "https://docs.mistral.ai/mistral-vibe/", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        vibe_version: "vibe --version",
        vibe_help: "vibe --help",
    }

    denied! {
        vibe_bare: "vibe",
        vibe_prompt: "vibe 'explain this code'",
        vibe_auto_approve: "vibe --auto-approve",
        vibe_prompt_flag: "vibe --prompt 'fix bug'",
    }
}
