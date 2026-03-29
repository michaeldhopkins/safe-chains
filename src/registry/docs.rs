use crate::policy::FlagStyle;

use super::types::*;

impl CommandSpec {
    pub(super) fn to_command_doc(&self) -> crate::docs::CommandDoc {
        let description = match &self.kind {
            CommandKind::Flat { policy, .. } => policy.describe(),
            CommandKind::FlatRequireAny { require_any, policy, .. } => {
                let req = require_any.join(", ");
                let summary = policy.describe();
                if summary.is_empty() {
                    format!("Requires {req}.")
                } else {
                    format!("Requires {req}. {summary}")
                }
            }
            CommandKind::Structured { bare_flags, subs } => {
                let mut lines = Vec::new();
                if !bare_flags.is_empty() {
                    lines.push(format!("- Allowed standalone flags: {}", bare_flags.join(", ")));
                }
                for sub in subs {
                    sub.doc_line("", &mut lines);
                }
                lines.sort();
                lines.join("\n")
            }
            CommandKind::Wrapper { .. } => {
                "- Recursively validates the inner command.".to_string()
            }
            CommandKind::Custom { .. } => String::new(),
        };
        let mut doc = crate::docs::CommandDoc::handler(
            Box::leak(self.name.clone().into_boxed_str()),
            Box::leak(self.url.clone().into_boxed_str()),
            description,
        );
        doc.aliases = self.aliases.iter().map(|a| a.to_string()).collect();
        doc
    }
}

impl OwnedPolicy {
    pub(super) fn describe(&self) -> String {
        let mut lines = Vec::new();
        if !self.standalone.is_empty() {
            lines.push(format!("- Allowed standalone flags: {}", self.standalone.join(", ")));
        }
        if !self.valued.is_empty() {
            lines.push(format!("- Allowed valued flags: {}", self.valued.join(", ")));
        }
        if self.bare {
            lines.push("- Bare invocation allowed".to_string());
        }
        if self.flag_style == FlagStyle::Positional {
            lines.push("- Hyphen-prefixed positional arguments accepted".to_string());
        }
        if lines.is_empty() && !self.bare {
            return "- Positional arguments only".to_string();
        }
        lines.join("\n")
    }

    pub(super) fn flag_summary(&self) -> String {
        let mut parts = Vec::new();
        if !self.standalone.is_empty() {
            parts.push(format!("Flags: {}", self.standalone.join(", ")));
        }
        if !self.valued.is_empty() {
            parts.push(format!("Valued: {}", self.valued.join(", ")));
        }
        if self.flag_style == FlagStyle::Positional {
            parts.push("Positional args accepted".to_string());
        }
        parts.join(". ")
    }
}

impl SubSpec {
    pub(super) fn doc_line(&self, prefix: &str, out: &mut Vec<String>) {
        let label = if prefix.is_empty() {
            self.name.clone()
        } else {
            format!("{prefix} {}", self.name)
        };
        match &self.kind {
            SubKind::Policy { policy, .. } => {
                let summary = policy.flag_summary();
                if summary.is_empty() {
                    out.push(format!("- **{label}**"));
                } else {
                    out.push(format!("- **{label}**: {summary}"));
                }
            }
            SubKind::Guarded { guard_long, policy, .. } => {
                let summary = policy.flag_summary();
                if summary.is_empty() {
                    out.push(format!("- **{label}** (requires {guard_long})"));
                } else {
                    out.push(format!("- **{label}** (requires {guard_long}): {summary}"));
                }
            }
            SubKind::Nested { subs, .. } => {
                for sub in subs {
                    sub.doc_line(&label, out);
                }
            }
            SubKind::AllowAll { .. } => {
                out.push(format!("- **{label}**"));
            }
            SubKind::WriteFlagged { policy, .. } => {
                let summary = policy.flag_summary();
                if summary.is_empty() {
                    out.push(format!("- **{label}**"));
                } else {
                    out.push(format!("- **{label}**: {summary}"));
                }
            }
            SubKind::FirstArgFilter { patterns, .. } => {
                let args = patterns.join(", ");
                out.push(format!("- **{label}**: Allowed arguments: {args}"));
            }
            SubKind::RequireAny { require_any, policy, .. } => {
                let req = require_any.join(", ");
                let summary = policy.flag_summary();
                if summary.is_empty() {
                    out.push(format!("- **{label}** (requires {req})"));
                } else {
                    out.push(format!("- **{label}** (requires {req}): {summary}"));
                }
            }
            SubKind::DelegateAfterSeparator { .. } | SubKind::DelegateSkip { .. } => {}
            SubKind::Custom { .. } => {}
        }
    }
}
