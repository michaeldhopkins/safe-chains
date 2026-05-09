use std::collections::HashMap;

use crate::policy::FlagStyle;
use crate::verdict::SafetyLevel;

use super::types::*;

pub(super) fn build_policy(
    standalone: Vec<String>,
    valued: Vec<String>,
    bare: Option<bool>,
    max_positional: Option<usize>,
    positional_style: Option<bool>,
    numeric_dash: Option<bool>,
) -> OwnedPolicy {
    OwnedPolicy {
        standalone,
        valued,
        bare: bare.unwrap_or(true),
        max_positional,
        flag_style: if positional_style.unwrap_or(false) {
            FlagStyle::Positional
        } else {
            FlagStyle::Strict
        },
        numeric_dash: numeric_dash.unwrap_or(false),
    }
}

fn allow_all_policy() -> OwnedPolicy {
    OwnedPolicy {
        standalone: Vec::new(),
        valued: Vec::new(),
        bare: true,
        max_positional: None,
        flag_style: FlagStyle::Positional,
        numeric_dash: false,
    }
}

fn filter_candidates(subs: Vec<TomlSub>) -> impl Iterator<Item = TomlSub> {
    subs.into_iter().filter(|s| !s.candidate.unwrap_or(false))
}

/// Builds one SubSpec per alias (canonical name first, then each alias).
/// All entries share the same kind via Clone — the dispatcher doesn't care
/// which name the user invoked.
pub(super) fn build_subs(toml: TomlSub) -> Vec<SubSpec> {
    let aliases = toml.aliases.clone();
    let canonical = build_sub(toml);
    let mut out = Vec::with_capacity(1 + aliases.len());
    for alias in aliases {
        out.push(SubSpec {
            name: alias,
            kind: canonical.kind.clone(),
        });
    }
    out.push(canonical);
    out
}

pub(super) fn build_sub(toml: TomlSub) -> SubSpec {
    if let Some(handler_name) = toml.handler {
        return SubSpec {
            name: toml.name,
            kind: DispatchKind::Custom { handler_name, doc_body: toml.doc_body },
        };
    }

    if toml.allow_all.unwrap_or(false) {
        return SubSpec {
            name: toml.name,
            kind: DispatchKind::Policy {
                policy: allow_all_policy(),
                level: toml.level.unwrap_or(TomlLevel::Inert).into(),
            },
        };
    }

    if let Some(sep) = toml.delegate_after {
        return SubSpec {
            name: toml.name,
            kind: DispatchKind::DelegateAfterSeparator { separator: sep },
        };
    }

    if let Some(skip) = toml.delegate_skip {
        return SubSpec {
            name: toml.name,
            kind: DispatchKind::DelegateSkip { skip },
        };
    }

    if !toml.sub.is_empty() {
        return SubSpec {
            name: toml.name,
            kind: DispatchKind::Branching {
                subs: filter_candidates(toml.sub).flat_map(build_subs).collect(),
                bare_flags: Vec::new(),
                bare_ok: toml.nested_bare.unwrap_or(false),
                pre_standalone: toml.standalone,
                pre_valued: toml.valued,
                first_arg: Vec::new(),
                first_arg_level: SafetyLevel::Inert,
            },
        };
    }

    let policy = build_policy(
        toml.standalone,
        toml.valued,
        toml.bare,
        toml.max_positional,
        toml.positional_style,
        toml.numeric_dash,
    );
    let level: SafetyLevel = toml.level.unwrap_or(TomlLevel::Inert).into();

    if !toml.write_flags.is_empty() {
        return SubSpec {
            name: toml.name,
            kind: DispatchKind::WriteFlagged {
                policy,
                base_level: level,
                write_flags: toml.write_flags,
            },
        };
    }

    if let Some(guard) = toml.guard {
        let mut require_any = vec![guard];
        if let Some(short) = toml.guard_short {
            require_any.push(short);
        }
        return SubSpec {
            name: toml.name,
            kind: DispatchKind::RequireAny {
                require_any,
                policy,
                level,
                accept_bare_help: true,
            },
        };
    }

    if !toml.first_arg.is_empty() {
        return SubSpec {
            name: toml.name,
            kind: DispatchKind::FirstArg {
                patterns: toml.first_arg,
                level,
            },
        };
    }

    if !toml.require_any.is_empty() {
        return SubSpec {
            name: toml.name,
            kind: DispatchKind::RequireAny {
                require_any: toml.require_any,
                policy,
                level,
                accept_bare_help: false,
            },
        };
    }

    SubSpec {
        name: toml.name,
        kind: DispatchKind::Policy { policy, level },
    }
}

/// Diagnostic for a configuration class that silently breaks flag dispatch:
/// a structured command (with `[[command.sub]]` blocks) cannot also use the
/// flat-style top-level fields. When subs are present, top-level standalone/
/// valued/max_positional/positional_style/numeric_dash are dropped — the
/// dispatch routes through the Branching path. The fix is to either remove
/// the subs (if the command is meant to be flat) or move global flags into
/// a `[command.wrapper]` block.
fn assert_flat_or_structured(toml: &TomlCommand) {
    if toml.sub.is_empty() {
        return;
    }
    let mut conflicts = Vec::new();
    if !toml.standalone.is_empty() {
        conflicts.push("standalone");
    }
    if !toml.valued.is_empty() {
        conflicts.push("valued");
    }
    if toml.max_positional.is_some() {
        conflicts.push("max_positional");
    }
    if toml.positional_style.is_some() {
        conflicts.push("positional_style");
    }
    if toml.numeric_dash.is_some() {
        conflicts.push("numeric_dash");
    }
    if !conflicts.is_empty() {
        panic!(
            "command '{}' mixes flat-style top-level fields ({}) with [[command.sub]] blocks. \
             When subs are present these fields are silently dropped. \
             Either drop the subs (if the command is flat) or move global \
             flags into a [command.wrapper] block.",
            toml.name,
            conflicts.join(", "),
        );
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn build_command(toml: TomlCommand, category: &str) -> CommandSpec {
    assert_flat_or_structured(&toml);
    let cat = category.to_string();
    let desc = toml.description.unwrap_or_default();
    let researched_version = toml.researched_version;
    let examples_safe = toml.examples_safe;
    let examples_denied = toml.examples_denied;
    if toml.deny.unwrap_or(false) {
        return CommandSpec {
            name: toml.name,
            description: desc,
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
            researched_version,
            examples_safe,
            examples_denied,
            kind: DispatchKind::Policy {
                policy: OwnedPolicy {
                    standalone: Vec::new(),
                    valued: Vec::new(),
                    bare: false,
                    max_positional: Some(0),
                    flag_style: crate::policy::FlagStyle::Strict,
                    numeric_dash: false,
                },
                level: SafetyLevel::Inert,
            },
        };
    }
    if let Some(handler_name) = toml.handler {
        return CommandSpec {
            name: toml.name,
            description: desc,
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
            researched_version,
            examples_safe,
            examples_denied,
            kind: DispatchKind::Custom { handler_name, doc_body: toml.doc_body },
        };
    }

    if let Some(w) = toml.wrapper {
        if !toml.sub.is_empty() || !toml.bare_flags.is_empty() {
            let first_arg_level = toml.level.unwrap_or(TomlLevel::Inert).into();
            return CommandSpec {
                name: toml.name,
                description: desc,
                aliases: toml.aliases,
                url: toml.url,
                category: cat,
                researched_version,
                examples_safe,
                examples_denied,
                kind: DispatchKind::Branching {
                    bare_flags: toml.bare_flags,
                    subs: filter_candidates(toml.sub).flat_map(build_subs).collect(),
                    pre_standalone: w.standalone,
                    pre_valued: w.valued,
                    bare_ok: toml.bare.unwrap_or(false),
                    first_arg: toml.first_arg,
                    first_arg_level,
                },
            };
        }
        return CommandSpec {
            name: toml.name,
            description: desc,
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
            researched_version,
            examples_safe,
            examples_denied,
            kind: DispatchKind::Wrapper {
                standalone: w.standalone,
                valued: w.valued,
                positional_skip: w.positional_skip.unwrap_or(0),
                separator: w.separator,
                bare_ok: w.bare_ok.unwrap_or(false),
            },
        };
    }

    if !toml.sub.is_empty() || !toml.bare_flags.is_empty() {
        let first_arg_level = toml.level.unwrap_or(TomlLevel::Inert).into();
        return CommandSpec {
            name: toml.name,
            description: desc,
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
            researched_version,
            examples_safe,
            examples_denied,
            kind: DispatchKind::Branching {
                bare_flags: toml.bare_flags,
                subs: filter_candidates(toml.sub).flat_map(build_subs).collect(),
                pre_standalone: Vec::new(),
                pre_valued: Vec::new(),
                bare_ok: toml.bare.unwrap_or(false),
                first_arg: toml.first_arg,
                first_arg_level,
            },
        };
    }

    let policy = build_policy(
        toml.standalone,
        toml.valued,
        toml.bare,
        toml.max_positional,
        toml.positional_style,
        toml.numeric_dash,
    );

    let level = toml.level.unwrap_or(TomlLevel::Inert).into();

    if !toml.first_arg.is_empty() {
        return CommandSpec {
            name: toml.name,
            description: desc,
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
            researched_version,
            examples_safe,
            examples_denied,
            kind: DispatchKind::FirstArg {
                patterns: toml.first_arg,
                level,
            },
        };
    }

    if !toml.write_flags.is_empty() {
        return CommandSpec {
            name: toml.name,
            description: desc,
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
            researched_version,
            examples_safe,
            examples_denied,
            kind: DispatchKind::WriteFlagged {
                policy,
                base_level: level,
                write_flags: toml.write_flags,
            },
        };
    }

    if !toml.require_any.is_empty() {
        return CommandSpec {
            name: toml.name,
            description: desc,
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
            researched_version,
            examples_safe,
            examples_denied,
            kind: DispatchKind::RequireAny {
                require_any: toml.require_any,
                policy,
                level,
                accept_bare_help: false,
            },
        };
    }

    CommandSpec {
        name: toml.name,
        description: desc,
        aliases: toml.aliases,
        url: toml.url,
        category: cat,
        researched_version,
        examples_safe,
        examples_denied,
        kind: DispatchKind::Policy {
            policy,
            level,
        },
    }
}

pub fn load_toml(source: &str, category: &str) -> Vec<CommandSpec> {
    let file: TomlFile = match toml::from_str(source) {
        Ok(f) => f,
        Err(e) => {
            let preview: String = source.chars().take(80).collect();
            panic!("invalid TOML command definition: {e}\n  source begins: {preview}");
        }
    };
    file.command.into_iter()
        .filter(|cmd| !cmd.candidate.unwrap_or(false))
        .map(|cmd| build_command(cmd, category))
        .collect()
}

pub fn build_registry(specs: Vec<CommandSpec>) -> HashMap<String, CommandSpec> {
    let mut map = HashMap::new();
    for spec in specs {
        insert_spec(&mut map, spec);
    }
    map
}

/// Insert a CommandSpec into the registry, registering both its canonical
/// name and each alias. Existing entries for the same command name are
/// removed first, so a custom-TOML override of `gh` replaces every
/// built-in alias of `gh` rather than leaving stale aliases pointing at
/// the old spec.
pub fn insert_spec(map: &mut HashMap<String, CommandSpec>, spec: CommandSpec) {
    map.retain(|_, s| s.name != spec.name);
    for alias in &spec.aliases {
        map.insert(alias.clone(), CommandSpec {
            name: spec.name.clone(),
            description: spec.description.clone(),
            aliases: vec![],
            url: spec.url.clone(),
            category: spec.category.clone(),
            researched_version: spec.researched_version.clone(),
            examples_safe: vec![],
            examples_denied: vec![],
            kind: spec.kind.clone(),
        });
    }
    map.insert(spec.name.clone(), spec);
}
