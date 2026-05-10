use super::types::*;

fn describe_handler_policies(
    policies: &std::collections::HashMap<String, OwnedPolicy>,
) -> Option<String> {
    if policies.is_empty() {
        return None;
    }
    let mut keys: Vec<&String> = policies.keys().collect();
    keys.sort();
    let mut lines = vec!["**Handler-side flag policies:**".to_string()];
    for key in keys {
        let summary = policies[key].flag_summary();
        if summary.is_empty() {
            lines.push(format!("- **{key}**"));
        } else {
            lines.push(format!("- **{key}**: {summary}"));
        }
    }
    Some(lines.join("\n"))
}

fn describe_handler_data(
    data: &std::collections::HashMap<String, Vec<String>>,
) -> Option<String> {
    if data.is_empty() {
        return None;
    }
    let mut keys: Vec<&String> = data.keys().collect();
    keys.sort();
    let mut lines = vec!["**Handler-side data:**".to_string()];
    for key in keys {
        lines.push(format!("- **{key}**: {}", data[key].join(", ")));
    }
    Some(lines.join("\n"))
}

fn describe_matrices(matrices: &[MatrixSpec]) -> Option<String> {
    if matrices.is_empty() {
        return None;
    }
    let mut lines = vec!["**Sub × action matrix:**".to_string()];
    for matrix in matrices {
        let parents = matrix.parents.join(", ");
        lines.push(format!("- Parents ({:?}): {parents}", matrix.level));
        let mut actions: Vec<&String> = matrix.actions.keys().collect();
        actions.sort();
        for name in actions {
            let action = &matrix.actions[name];
            let guard = match (&action.guard, &action.guard_short) {
                (Some(long), Some(short)) => format!(" (requires {short}/{long})"),
                (Some(long), None) => format!(" (requires {long})"),
                _ => String::new(),
            };
            lines.push(format!("  - **{name}** → policy `{}`{guard}", action.policy_key));
        }
    }
    Some(lines.join("\n"))
}

fn describe_fallback(f: &FallbackSpec) -> String {
    let mut lines = vec!["**Fallback grammar (engaged when no sub matches):**".to_string()];
    if f.policy.bare {
        lines.push("- Bare invocation allowed".to_string());
    }
    if !f.policy.standalone.is_empty() {
        lines.push(format!("- Allowed standalone flags: {}", f.policy.standalone.join(", ")));
    }
    if !f.policy.valued.is_empty() {
        lines.push(format!("- Allowed valued flags: {}", f.policy.valued.join(", ")));
    }
    match f.positional_shape {
        Some(crate::policy::PositionalShape::Path) => {
            lines.push("- First positional must look like a path (contains `/`, `.`, or is `-` for stdin)".to_string());
        }
        None => {}
    }
    lines.join("\n")
}

impl CommandSpec {
    pub(super) fn to_command_doc(&self) -> crate::docs::CommandDoc {
        let description = match &self.kind {
            DispatchKind::Policy { policy, .. } => policy.describe(),
            DispatchKind::RequireAny { require_any, policy, .. } => {
                let req = require_any.join(", ");
                let summary = policy.describe();
                if summary.is_empty() {
                    format!("Requires {req}.")
                } else {
                    format!("Requires {req}. {summary}")
                }
            }
            DispatchKind::Branching { bare_flags, subs, bare_ok, first_arg, .. } => {
                let mut lines = Vec::new();
                if *bare_ok {
                    lines.push("- Bare invocation allowed".to_string());
                }
                if !bare_flags.is_empty() {
                    lines.push(format!("- Allowed standalone flags: {}", bare_flags.join(", ")));
                }
                for sub in subs {
                    sub.doc_line("", &mut lines);
                }
                if !first_arg.is_empty() {
                    lines.push(format!("- Allowed arguments: {}", first_arg.join(", ")));
                }
                lines.sort();
                lines.join("\n")
            }
            DispatchKind::Wrapper { .. } => {
                "- Recursively validates the inner command.".to_string()
            }
            DispatchKind::FirstArg { patterns, .. } => {
                let args = patterns.join(", ");
                format!("Allowed first arguments: {args}")
            }
            DispatchKind::Custom { doc_body, subs, fallback, handler_policies, handler_data, matrices, .. } => {
                let mut sections: Vec<String> = Vec::new();
                if let Some(body) = doc_body
                    && !body.trim().is_empty()
                {
                    sections.push(body.clone());
                }
                let mut sub_lines = Vec::new();
                for sub in subs {
                    sub.doc_line("", &mut sub_lines);
                }
                if !sub_lines.is_empty() {
                    sub_lines.sort();
                    sections.push(sub_lines.join("\n"));
                }
                if let Some(s) = describe_matrices(matrices) {
                    sections.push(s);
                }
                if let Some(f) = fallback {
                    sections.push(describe_fallback(f));
                }
                if let Some(s) = describe_handler_policies(handler_policies) {
                    sections.push(s);
                }
                if let Some(s) = describe_handler_data(handler_data) {
                    sections.push(s);
                }
                sections.join("\n\n")
            }
            DispatchKind::WriteFlagged { policy, .. } => policy.describe(),
            DispatchKind::DelegateAfterSeparator { .. } | DispatchKind::DelegateSkip { .. } => String::new(),
        };
        let mut doc = crate::docs::CommandDoc::handler(
            Box::leak(self.name.clone().into_boxed_str()),
            Box::leak(self.url.clone().into_boxed_str()),
            description,
            &self.category,
        );
        doc.aliases = self.aliases.iter().map(|a| a.to_string()).collect();
        doc.examples = self.examples_safe.clone();
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
        if self.tolerance.unknown != crate::policy::UnknownTolerance::Strict {
            lines.push("- Hyphen-prefixed positional arguments accepted".to_string());
        }
        if self.tolerance.numeric_dash {
            lines.push("- Numeric shorthand accepted (e.g. -20 for -n 20)".to_string());
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
        if self.tolerance.unknown != crate::policy::UnknownTolerance::Strict {
            parts.push("Positional args accepted".to_string());
        }
        if self.tolerance.numeric_dash {
            parts.push("Numeric -N accepted".to_string());
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
            DispatchKind::Policy { policy, .. } => {
                let summary = policy.flag_summary();
                if summary.is_empty() {
                    out.push(format!("- **{label}**"));
                } else {
                    out.push(format!("- **{label}**: {summary}"));
                }
            }
            DispatchKind::RequireAny { require_any, policy, .. } => {
                let req = require_any.join(", ");
                let summary = policy.flag_summary();
                if summary.is_empty() {
                    out.push(format!("- **{label}** (requires {req})"));
                } else {
                    out.push(format!("- **{label}** (requires {req}): {summary}"));
                }
            }
            DispatchKind::Branching { subs, pre_standalone, pre_valued, .. } => {
                if !pre_standalone.is_empty() || !pre_valued.is_empty() {
                    let mut parts = Vec::new();
                    if !pre_standalone.is_empty() {
                        parts.push(format!("Flags: {}", pre_standalone.join(", ")));
                    }
                    if !pre_valued.is_empty() {
                        parts.push(format!("Valued: {}", pre_valued.join(", ")));
                    }
                    out.push(format!("- **{label}**: {}", parts.join(". ")));
                }
                for sub in subs {
                    sub.doc_line(&label, out);
                }
            }
            DispatchKind::FirstArg { patterns, .. } => {
                let args = patterns.join(", ");
                out.push(format!("- **{label}**: Allowed arguments: {args}"));
            }
            DispatchKind::WriteFlagged { policy, .. } => {
                let summary = policy.flag_summary();
                if summary.is_empty() {
                    out.push(format!("- **{label}**"));
                } else {
                    out.push(format!("- **{label}**: {summary}"));
                }
            }
            DispatchKind::DelegateAfterSeparator { .. } | DispatchKind::DelegateSkip { .. } => {
                out.push(format!("- **{label}**: delegates to inner command"));
            }
            DispatchKind::Custom { doc_body, .. } => {
                if let Some(body) = doc_body {
                    out.push(format!("- **{label}**: {body}"));
                } else {
                    out.push(format!("- **{label}**"));
                }
            }
            DispatchKind::Wrapper { .. } => {}
        }
    }
}
