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

/// Render a `[command.behavior]` command's docs from its FACETS — a plain-language summary of
/// what it does and its workspace scope, plus its behavior flag grammar (not the legacy fields).
/// Allowlist-only: states the allowed scope positively ("within your workspace"), never a denial.
fn describe_behavior(b: &crate::registry::types::BehaviorSpec) -> String {
    use crate::engine::facet::Operation;
    use crate::registry::types::{PositionalRole, TransferSource};

    let scope = " within your workspace";
    let summary = match (b.operation, b.positionals) {
        (Operation::Observe, PositionalRole::None) => "Prints its arguments to stdout.".to_string(),
        (Operation::Observe, PositionalRole::PatternThenRead) => {
            format!("Searches its file operands for a pattern{scope}.")
        }
        (Operation::Observe, _) => format!("Reads its file operands to stdout{scope}."),
        (Operation::Destroy, _) => format!("Deletes its file operands{scope}."),
        (Operation::Create, PositionalRole::Transfer) => match b.transfer.as_ref().map(|t| t.source) {
            Some(TransferSource::Relocate) => format!("Moves each source to the destination{scope}."),
            _ => format!("Reads each source and writes the destination{scope}."),
        },
        (Operation::Create, _) => format!("Creates or updates its file operands{scope}."),
        (Operation::Mutate, _) => format!("Edits its file operands in place{scope}."),
        (_, _) => format!("Operates on its file operands{scope}."),
    };
    let mut lines = vec![summary];

    // A hook parses its own flag grammar (grep/dd/tar/sed), so the behavior config carries no
    // flag lists — the examples below show real invocations. A declarative command lists every
    // flag it accepts, reconstructed from the behavior grammar (short bytes → `-x`, longs as-is).
    if b.hook.is_some() {
        lines.push("- Flag handling follows the command's own grammar (see examples).".to_string());
    } else {
        let mut standalone: Vec<String> = b.short.iter().map(|&c| format!("-{}", c as char)).collect();
        standalone.extend(b.long.iter().cloned());
        standalone.sort();
        let mut valued: Vec<String> = b.valued_short.iter().map(|&c| format!("-{}", c as char)).collect();
        valued.extend(b.valued_long.iter().cloned());
        valued.sort();
        if !standalone.is_empty() {
            lines.push(format!("- Allowed standalone flags: {}", standalone.join(", ")));
        }
        if !valued.is_empty() {
            lines.push(format!("- Allowed valued flags: {}", valued.join(", ")));
        }
    }

    // Flags that widen the effect to a whole directory tree (rm -r, cp -r/-R/-a).
    let mut recursive: Vec<String> = b.unbounded_flags.clone();
    if let Some(t) = &b.transfer {
        recursive.extend(t.recursive_flags.iter().cloned());
    }
    recursive.sort();
    recursive.dedup();
    if !recursive.is_empty() {
        lines.push(format!("- Recurses into directory trees with: {}", recursive.join(", ")));
    }
    if matches!(b.positionals, PositionalRole::Read | PositionalRole::None) && b.hook.is_none() {
        lines.push("- Bare invocation reads stdin".to_string());
    }
    lines.join("\n")
}

/// Render a handler command's (`DispatchKind::Custom`) docs: any doc body, its named subs (a
/// sub whose policy is shared 2+ times renders as a reference), then bare-flag forms, the
/// sub × action grid, and the shared flag-set definitions.
fn describe_custom(
    doc_body: &Option<String>,
    subs: &[SubSpec],
    fallback: &Option<FallbackSpec>,
    handler_policies: &std::collections::HashMap<String, OwnedPolicy>,
    matrices: &[MatrixSpec],
) -> String {
    let mut sections: Vec<String> = Vec::new();
    if let Some(body) = doc_body
        && !body.trim().is_empty()
    {
        sections.push(body.clone());
    }
    let shared_keys: std::collections::HashSet<&str> =
        matrix_policy_usage(matrices).iter().filter(|(_, count)| **count >= 2).map(|(k, _)| *k).collect();
    let mut sub_lines = Vec::new();
    for sub in subs {
        if let Some(ref_name) = sub.policy_ref.as_deref()
            && shared_keys.contains(ref_name)
        {
            sub_lines.push(format!("- **{}** — see `{ref_name}` below", sub.name));
        } else {
            sub.doc_line("", &mut sub_lines);
        }
    }
    if !sub_lines.is_empty() {
        sub_lines.sort();
        sections.push(sub_lines.join("\n"));
    }
    // Reader-friendly order: named subs → bare-flag forms → sub × action grid → shared sets.
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
        Some(crate::policy::PositionalShape::GoPackage) => {
            lines.push("- First positional must be a local package path (`.`, `./…`, `/…`, or a `*.go` file)".to_string());
        }
        None => {}
    }
    lines.join("\n")
}

impl CommandSpec {
    pub(super) fn to_command_doc(&self) -> crate::docs::CommandDoc {
        // A `[command.behavior]` command documents what it DOES, from its facets — the
        // authoritative classification — rather than its legacy fallback fields (which drift:
        // e.g. cat's legacy `standalone` lists -V/-h/-l, which the behavior grammar denies).
        let description = if let Some(b) = &self.behavior {
            describe_behavior(b)
        } else {
            self.describe_kind()
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

    /// The legacy-field description for a non-`behavior` command, keyed by dispatch kind.
    fn describe_kind(&self) -> String {
        match &self.kind {
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
                describe_custom(doc_body, subs, fallback, handler_policies, matrices)
            }
            DispatchKind::WriteFlagged { policy, .. } => policy.describe(),
            DispatchKind::VerbChain(spec) => {
                let mut verbs: Vec<&str> = spec.verbs.iter().map(String::as_str).collect();
                verbs.sort_unstable();
                format!(
                    "- Read-only verb chain (`verb [args] {} verb …`). Allowed verbs: {}",
                    spec.separator,
                    verbs.join(", "),
                )
            }
            DispatchKind::Executor { policy, kind, .. } => describe_executor(policy, *kind),
            DispatchKind::DelegateAfterSeparator { .. } | DispatchKind::DelegateSkip { .. } => String::new(),
        }
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
            DispatchKind::Executor { policy, kind, .. } => {
                let summary = policy.flag_summary();
                let body = describe_executor(policy, *kind);
                if summary.is_empty() {
                    out.push(format!("- **{label}**: {body}"));
                } else {
                    out.push(format!("- **{label}**: {body} {summary}"));
                }
            }
            DispatchKind::VerbChain(_) => out.push(format!("- **{label}**: verb chain")),
            DispatchKind::Wrapper { .. } => {}
        }
    }
}

/// One-line doc for an executor node: what it runs and that the executor is locus-gated.
fn describe_executor(_policy: &OwnedPolicy, kind: ExecutorKind) -> String {
    match kind {
        ExecutorKind::File => {
            "Runs a workspace-local script/package (the first positional).".to_string()
        }
        ExecutorKind::Project => "Runs this project's own code.".to_string(),
    }
}
