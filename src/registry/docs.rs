use super::types::*;

fn matrix_policy_usage(
    matrices: &[MatrixSpec],
) -> std::collections::HashMap<&str, usize> {
    let mut usage: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for matrix in matrices {
        for action in matrix.actions.values() {
            *usage.entry(action.policy_key.as_str()).or_insert(0) += 1;
        }
    }
    usage
}

fn describe_shared_policies(
    matrices: &[MatrixSpec],
    handler_policies: &std::collections::HashMap<String, OwnedPolicy>,
) -> Option<String> {
    let usage = matrix_policy_usage(matrices);
    let mut shared: Vec<&String> = handler_policies
        .keys()
        .filter(|k| usage.get(k.as_str()).copied().unwrap_or(0) >= 2)
        .collect();
    shared.sort();
    if shared.is_empty() {
        return None;
    }
    let mut lines = vec!["**Shared flag sets:**".to_string()];
    for key in shared {
        let summary = handler_policies[key].flag_summary();
        if summary.is_empty() {
            lines.push(format!("- **{key}**"));
        } else {
            lines.push(format!("- **{key}**: {summary}"));
        }
    }
    Some(lines.join("\n"))
}

fn describe_matrices(
    matrices: &[MatrixSpec],
    handler_policies: &std::collections::HashMap<String, OwnedPolicy>,
) -> Option<String> {
    if matrices.is_empty() {
        return None;
    }
    let usage = matrix_policy_usage(matrices);
    let mut lines = vec!["**Subcommands by action verb:**".to_string()];
    for matrix in matrices {
        let parents = matrix.parents.join(", ");
        let level = format!("{:?}", matrix.level);
        lines.push(format!("- **{parents}** ({level})"));
        let mut actions: Vec<&String> = matrix.actions.keys().collect();
        actions.sort();
        for name in actions {
            let action = &matrix.actions[name];
            let guard = match (&action.guard, &action.guard_short) {
                (Some(long), Some(short)) => format!(" (requires {short}/{long})"),
                (Some(long), None) => format!(" (requires {long})"),
                _ => String::new(),
            };
            let shared_count = usage.get(action.policy_key.as_str()).copied().unwrap_or(0);
            if shared_count >= 2 {
                // Reference by name — the flag list shows up in
                // **Shared flag sets** below.
                lines.push(format!(
                    "  - **{name}**{guard} — see `{}` below",
                    action.policy_key,
                ));
            } else if let Some(policy) = handler_policies.get(&action.policy_key) {
                let summary = policy.flag_summary();
                if summary.is_empty() {
                    lines.push(format!("  - **{name}**{guard}"));
                } else {
                    lines.push(format!("  - **{name}**{guard}: {summary}"));
                }
            } else {
                lines.push(format!("  - **{name}**{guard}"));
            }
        }
    }
    Some(lines.join("\n"))
}

fn describe_fallback(f: &FallbackSpec) -> String {
    let mut lines = vec!["**Without a subcommand:**".to_string()];
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
            DispatchKind::Custom { doc_body, subs, fallback, handler_policies, matrices, .. } => {
                let mut sections: Vec<String> = Vec::new();
                if let Some(body) = doc_body
                    && !body.trim().is_empty()
                {
                    sections.push(body.clone());
                }
                // Policies referenced 2+ times across the matrix render
                // as a separate "Shared flag sets" section. Subs whose
                // policy_ref points at one of these render as a
                // reference too, so the flag list appears exactly once
                // (in the shared section).
                let shared_keys: std::collections::HashSet<&str> =
                    matrix_policy_usage(matrices)
                        .iter()
                        .filter(|(_, count)| **count >= 2)
                        .map(|(k, _)| *k)
                        .collect();
                let mut sub_lines = Vec::new();
                for sub in subs {
                    if let Some(ref_name) = sub.policy_ref.as_deref()
                        && shared_keys.contains(ref_name)
                    {
                        sub_lines.push(format!(
                            "- **{}** — see `{ref_name}` below",
                            sub.name,
                        ));
                    } else {
                        sub.doc_line("", &mut sub_lines);
                    }
                }
                if !sub_lines.is_empty() {
                    sub_lines.sort();
                    sections.push(sub_lines.join("\n"));
                }
                // Reader-friendly order: named subs → bare-flag forms →
                // sub × action grid → shared flag-set definitions.
                if let Some(f) = fallback {
                    sections.push(describe_fallback(f));
                }
                if let Some(s) = describe_matrices(matrices, handler_policies) {
                    sections.push(s);
                }
                if let Some(s) = describe_shared_policies(matrices, handler_policies) {
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
