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

pub(super) fn build_sub(toml: TomlSub) -> SubSpec {
    if let Some(handler_name) = toml.handler {
        return SubSpec {
            name: toml.name,
            kind: SubKind::Custom { handler_name },
        };
    }

    if toml.allow_all.unwrap_or(false) {
        return SubSpec {
            name: toml.name,
            kind: SubKind::AllowAll {
                level: toml.level.unwrap_or(TomlLevel::Inert).into(),
            },
        };
    }

    if let Some(sep) = toml.delegate_after {
        return SubSpec {
            name: toml.name,
            kind: SubKind::DelegateAfterSeparator { separator: sep },
        };
    }

    if let Some(skip) = toml.delegate_skip {
        return SubSpec {
            name: toml.name,
            kind: SubKind::DelegateSkip {
                skip,
                doc: toml.doc.unwrap_or_default(),
            },
        };
    }

    if !toml.sub.is_empty() {
        return SubSpec {
            name: toml.name,
            kind: SubKind::Nested {
                subs: toml.sub.into_iter().map(build_sub).collect(),
                allow_bare: toml.nested_bare.unwrap_or(false),
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
            kind: SubKind::WriteFlagged {
                policy,
                base_level: level,
                write_flags: toml.write_flags,
            },
        };
    }

    if let Some(guard) = toml.guard {
        return SubSpec {
            name: toml.name,
            kind: SubKind::Guarded {
                guard_long: guard,
                guard_short: toml.guard_short,
                policy,
                level,
            },
        };
    }

    if !toml.first_arg.is_empty() {
        return SubSpec {
            name: toml.name,
            kind: SubKind::FirstArgFilter {
                patterns: toml.first_arg,
                level,
            },
        };
    }

    if !toml.require_any.is_empty() {
        return SubSpec {
            name: toml.name,
            kind: SubKind::RequireAny {
                require_any: toml.require_any,
                policy,
                level,
            },
        };
    }

    SubSpec {
        name: toml.name,
        kind: SubKind::Policy { policy, level },
    }
}

pub(super) fn build_command(toml: TomlCommand) -> CommandSpec {
    if let Some(handler_name) = toml.handler {
        return CommandSpec {
            name: toml.name,
            aliases: toml.aliases,
            url: toml.url,
            kind: CommandKind::Custom { handler_name },
        };
    }

    if let Some(w) = toml.wrapper {
        if !toml.sub.is_empty() || !toml.bare_flags.is_empty() {
            return CommandSpec {
                name: toml.name,
                aliases: toml.aliases,
                url: toml.url,
                kind: CommandKind::Structured {
                    bare_flags: toml.bare_flags,
                    subs: toml.sub.into_iter().map(build_sub).collect(),
                    pre_standalone: w.standalone,
                    pre_valued: w.valued,
                },
            };
        }
        return CommandSpec {
            name: toml.name,
            aliases: toml.aliases,
            url: toml.url,
            kind: CommandKind::Wrapper {
                standalone: w.standalone,
                valued: w.valued,
                positional_skip: w.positional_skip.unwrap_or(0),
                separator: w.separator,
                bare_ok: w.bare_ok.unwrap_or(false),
            },
        };
    }

    if !toml.sub.is_empty() || !toml.bare_flags.is_empty() {
        return CommandSpec {
            name: toml.name,
            aliases: toml.aliases,
            url: toml.url,
            kind: CommandKind::Structured {
                bare_flags: toml.bare_flags,
                subs: toml.sub.into_iter().map(build_sub).collect(),
                pre_standalone: Vec::new(),
                pre_valued: Vec::new(),
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
            kind: CommandKind::FlatFirstArg {
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
            kind: CommandKind::FlatRequireAny {
                require_any: toml.require_any,
                policy,
                level,
            },
        };
    }

    CommandSpec {
        name: toml.name,
        aliases: toml.aliases,
        url: toml.url,
        kind: CommandKind::Flat {
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
                    CommandKind::Flat { policy, level } => CommandKind::Flat {
                        policy: OwnedPolicy {
                            standalone: policy.standalone.clone(),
                            valued: policy.valued.clone(),
                            bare: policy.bare,
                            max_positional: policy.max_positional,
                            flag_style: policy.flag_style,
                        },
                        level: *level,
                    },
                    CommandKind::FlatFirstArg { patterns, level } => CommandKind::FlatFirstArg {
                        patterns: patterns.clone(),
                        level: *level,
                    },
                    CommandKind::FlatRequireAny { require_any, policy, level } => CommandKind::FlatRequireAny {
                        require_any: require_any.clone(),
                        policy: OwnedPolicy {
                            standalone: policy.standalone.clone(),
                            valued: policy.valued.clone(),
                            bare: policy.bare,
                            max_positional: policy.max_positional,
                            flag_style: policy.flag_style,
                        },
                        level: *level,
                    },
                    CommandKind::Wrapper { standalone, valued, positional_skip, separator, bare_ok } => CommandKind::Wrapper {
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
