use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static CLAUDE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--version", "-V", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::ai) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "claude", policy: &CLAUDE_POLICY, level: SafetyLevel::Inert, url: "https://docs.anthropic.com/en/docs/claude-code", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        claude_version: "claude --version",
        claude_help: "claude --help",
    }

    denied! {
        claude_bare_denied: "claude",
        claude_prompt_denied: "claude 'explain this code'",
        claude_print_denied: "claude --print 'hello'",
        claude_dangerous_denied: "claude --dangerously-skip-permissions",
    }
}
