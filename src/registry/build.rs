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

pub(super) fn build_sub(toml: TomlSub) -> SubSpec {
    if let Some(handler_name) = toml.handler {
        return SubSpec {
            name: toml.name,
            kind: DispatchKind::Custom { handler_name },
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
                subs: toml.sub.into_iter().map(build_sub).collect(),
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

#[allow(clippy::too_many_lines)]
pub(super) fn build_command(toml: TomlCommand, category: &str) -> CommandSpec {
    let cat = category.to_string();
    if let Some(handler_name) = toml.handler {
        return CommandSpec {
            name: toml.name,
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
            kind: DispatchKind::Custom { handler_name },
        };
    }

    if let Some(w) = toml.wrapper {
        if !toml.sub.is_empty() || !toml.bare_flags.is_empty() {
            let first_arg_level = toml.level.unwrap_or(TomlLevel::Inert).into();
            return CommandSpec {
                name: toml.name,
                aliases: toml.aliases,
                url: toml.url,
                category: cat,
                kind: DispatchKind::Branching {
                    bare_flags: toml.bare_flags,
                    subs: toml.sub.into_iter().map(build_sub).collect(),
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
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
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
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
            kind: DispatchKind::Branching {
                bare_flags: toml.bare_flags,
                subs: toml.sub.into_iter().map(build_sub).collect(),
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
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
            kind: DispatchKind::FirstArg {
                patterns: toml.first_arg,
                level,
            },
        };
    }

    if !toml.require_any.is_empty() {
        return CommandSpec {
            name: toml.name,
            aliases: toml.aliases,
            url: toml.url,
            category: cat,
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
        aliases: toml.aliases,
        url: toml.url,
        category: cat,
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
    file.command.into_iter().map(|cmd| build_command(cmd, category)).collect()
}

pub fn build_registry(specs: Vec<CommandSpec>) -> HashMap<String, CommandSpec> {
    let mut map = HashMap::new();
    for spec in specs {
        for alias in &spec.aliases {
            map.insert(alias.clone(), CommandSpec {
                name: spec.name.clone(),
                aliases: vec![],
                url: spec.url.clone(),
                category: spec.category.clone(),
                kind: spec.kind.clone(),
            });
        }
        map.insert(spec.name.clone(), spec);
    }
    map
}
