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
    }
}

fn allow_all_policy() -> OwnedPolicy {
    OwnedPolicy {
        standalone: Vec::new(),
        valued: Vec::new(),
        bare: true,
        max_positional: None,
        flag_style: FlagStyle::Positional,
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

pub(super) fn build_command(toml: TomlCommand) -> CommandSpec {
    if let Some(handler_name) = toml.handler {
        return CommandSpec {
            name: toml.name,
            aliases: toml.aliases,
            url: toml.url,
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
    );

    let level = toml.level.unwrap_or(TomlLevel::Inert).into();

    if !toml.first_arg.is_empty() {
        return CommandSpec {
            name: toml.name,
            aliases: toml.aliases,
            url: toml.url,
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
        kind: DispatchKind::Policy {
            policy,
            level,
        },
    }
}

pub fn load_toml(source: &str) -> Vec<CommandSpec> {
    let file: TomlFile = toml::from_str(source).expect("invalid TOML command definition");
    file.command.into_iter().map(build_command).collect()
}

pub fn build_registry(specs: Vec<CommandSpec>) -> HashMap<String, CommandSpec> {
    let mut map = HashMap::new();
    for spec in specs {
        for alias in &spec.aliases {
            map.insert(alias.clone(), CommandSpec {
                name: spec.name.clone(),
                aliases: vec![],
                url: spec.url.clone(),
                kind: match &spec.kind {
                    DispatchKind::Policy { policy, level } => DispatchKind::Policy {
                        policy: OwnedPolicy {
                            standalone: policy.standalone.clone(),
                            valued: policy.valued.clone(),
                            bare: policy.bare,
                            max_positional: policy.max_positional,
                            flag_style: policy.flag_style,
                        },
                        level: *level,
                    },
                    DispatchKind::FirstArg { patterns, level } => DispatchKind::FirstArg {
                        patterns: patterns.clone(),
                        level: *level,
                    },
                    DispatchKind::RequireAny { require_any, policy, level, accept_bare_help } => DispatchKind::RequireAny {
                        require_any: require_any.clone(),
                        policy: OwnedPolicy {
                            standalone: policy.standalone.clone(),
                            valued: policy.valued.clone(),
                            bare: policy.bare,
                            max_positional: policy.max_positional,
                            flag_style: policy.flag_style,
                        },
                        level: *level,
                        accept_bare_help: *accept_bare_help,
                    },
                    DispatchKind::Wrapper { standalone, valued, positional_skip, separator, bare_ok } => DispatchKind::Wrapper {
                        standalone: standalone.clone(),
                        valued: valued.clone(),
                        positional_skip: *positional_skip,
                        separator: separator.clone(),
                        bare_ok: *bare_ok,
                    },
                    _ => continue,
                },
            });
        }
        map.insert(spec.name.clone(), spec);
    }
    map
}
