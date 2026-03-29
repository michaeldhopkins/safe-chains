use std::collections::HashMap;

use serde::Deserialize;

use crate::parse::Token;
use crate::policy::FlagStyle;
use crate::verdict::{SafetyLevel, Verdict};

#[derive(Debug, Deserialize)]
struct TomlFile {
    command: Vec<TomlCommand>,
}

#[derive(Debug, Deserialize)]
struct TomlCommand {
    name: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    url: String,
    #[serde(default)]
    level: Option<TomlLevel>,
    #[serde(default)]
    bare: Option<bool>,
    #[serde(default)]
    max_positional: Option<usize>,
    #[serde(default)]
    positional_style: Option<bool>,
    #[serde(default)]
    standalone: Vec<String>,
    #[serde(default)]
    valued: Vec<String>,
    #[serde(default)]
    bare_flags: Vec<String>,
    #[serde(default)]
    sub: Vec<TomlSub>,
    #[serde(default)]
    handler: Option<String>,
    #[serde(default)]
    require_any: Vec<String>,
    #[allow(dead_code)]
    #[serde(default)]
    doc: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TomlSub {
    name: String,
    #[serde(default)]
    level: Option<TomlLevel>,
    #[serde(default)]
    bare: Option<bool>,
    #[serde(default)]
    max_positional: Option<usize>,
    #[serde(default)]
    positional_style: Option<bool>,
    #[serde(default)]
    standalone: Vec<String>,
    #[serde(default)]
    valued: Vec<String>,
    #[serde(default)]
    guard: Option<String>,
    #[serde(default)]
    guard_short: Option<String>,
    #[serde(default)]
    allow_all: Option<bool>,
    #[serde(default)]
    sub: Vec<TomlSub>,
    #[serde(default)]
    nested_bare: Option<bool>,
    #[serde(default)]
    require_any: Vec<String>,
    #[serde(default)]
    first_arg: Vec<String>,
    #[serde(default)]
    write_flags: Vec<String>,
    #[serde(default)]
    delegate_after: Option<String>,
    #[serde(default)]
    delegate_skip: Option<usize>,
    #[serde(default)]
    handler: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    doc: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
enum TomlLevel {
    Inert,
    SafeRead,
    SafeWrite,
}

impl From<TomlLevel> for SafetyLevel {
    fn from(l: TomlLevel) -> Self {
        match l {
            TomlLevel::Inert => SafetyLevel::Inert,
            TomlLevel::SafeRead => SafetyLevel::SafeRead,
            TomlLevel::SafeWrite => SafetyLevel::SafeWrite,
        }
    }
}

#[derive(Debug)]
pub struct CommandSpec {
    pub name: String,
    pub aliases: Vec<String>,
    pub url: String,
    kind: CommandKind,
}

#[derive(Debug)]
enum CommandKind {
    Flat {
        policy: OwnedPolicy,
        level: SafetyLevel,
    },
    FlatRequireAny {
        require_any: Vec<String>,
        policy: OwnedPolicy,
        level: SafetyLevel,
    },
    Structured {
        bare_flags: Vec<String>,
        subs: Vec<SubSpec>,
    },
    Custom {
        #[allow(dead_code)]
        handler_name: String,
    },
}

#[derive(Debug)]
struct SubSpec {
    name: String,
    kind: SubKind,
}

#[derive(Debug)]
enum SubKind {
    Policy {
        policy: OwnedPolicy,
        level: SafetyLevel,
    },
    Guarded {
        guard_long: String,
        guard_short: Option<String>,
        policy: OwnedPolicy,
        level: SafetyLevel,
    },
    Nested {
        subs: Vec<SubSpec>,
        allow_bare: bool,
    },
    AllowAll {
        level: SafetyLevel,
    },
    WriteFlagged {
        policy: OwnedPolicy,
        base_level: SafetyLevel,
        write_flags: Vec<String>,
    },
    FirstArgFilter {
        patterns: Vec<String>,
        level: SafetyLevel,
    },
    RequireAny {
        require_any: Vec<String>,
        policy: OwnedPolicy,
        level: SafetyLevel,
    },
    DelegateAfterSeparator {
        separator: String,
    },
    DelegateSkip {
        skip: usize,
        #[allow(dead_code)]
        doc: String,
    },
    Custom {
        #[allow(dead_code)]
        handler_name: String,
    },
}

#[derive(Debug)]
pub struct OwnedPolicy {
    pub standalone: Vec<String>,
    pub valued: Vec<String>,
    pub bare: bool,
    pub max_positional: Option<usize>,
    pub flag_style: FlagStyle,
}

fn check_owned(tokens: &[Token], policy: &OwnedPolicy) -> bool {
    if tokens.len() == 1 {
        return policy.bare;
    }

    let mut i = 1;
    let mut positionals: usize = 0;
    while i < tokens.len() {
        let t = &tokens[i];

        if *t == "--" {
            positionals += tokens.len() - i - 1;
            break;
        }

        if !t.starts_with('-') {
            positionals += 1;
            i += 1;
            continue;
        }

        if policy.standalone.iter().any(|f| t == f.as_str()) {
            i += 1;
            continue;
        }

        if policy.valued.iter().any(|f| t == f.as_str()) {
            i += 2;
            continue;
        }

        if let Some(flag) = t.as_str().split_once('=').map(|(f, _)| f) {
            if policy.valued.iter().any(|f| f.as_str() == flag) {
                i += 1;
                continue;
            }
            if policy.flag_style == FlagStyle::Positional {
                positionals += 1;
                i += 1;
                continue;
            }
            return false;
        }

        if t.starts_with("--") {
            if policy.flag_style == FlagStyle::Positional {
                positionals += 1;
                i += 1;
                continue;
            }
            return false;
        }

        let bytes = t.as_bytes();
        let mut j = 1;
        while j < bytes.len() {
            let b = bytes[j];
            let is_last = j == bytes.len() - 1;
            if policy.standalone.iter().any(|f| f.len() == 2 && f.as_bytes()[1] == b) {
                j += 1;
                continue;
            }
            if policy.valued.iter().any(|f| f.len() == 2 && f.as_bytes()[1] == b) {
                if is_last {
                    i += 1;
                }
                break;
            }
            if policy.flag_style == FlagStyle::Positional {
                positionals += 1;
                break;
            }
            return false;
        }
        i += 1;
    }
    policy.max_positional.is_none_or(|max| positionals <= max)
}

fn build_policy(
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

fn build_sub(toml: TomlSub) -> SubSpec {
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

fn build_command(toml: TomlCommand) -> CommandSpec {
    if let Some(handler_name) = toml.handler {
        return CommandSpec {
            name: toml.name,
            aliases: toml.aliases,
            url: toml.url,
            kind: CommandKind::Custom { handler_name },
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
                    _ => continue,
                },
            });
        }
        map.insert(spec.name.clone(), spec);
    }
    map
}

fn has_flag_owned(tokens: &[Token], short: Option<&str>, long: &str) -> bool {
    tokens[1..].iter().any(|t| {
        t == long
            || short.is_some_and(|s| t == s)
            || t.as_str().starts_with(&format!("{long}="))
    })
}

fn dispatch_first_arg(tokens: &[Token], patterns: &[String], level: SafetyLevel) -> Verdict {
    if tokens.len() == 2 && (tokens[1] == "--help" || tokens[1] == "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let Some(arg) = tokens.get(1) else {
        return Verdict::Denied;
    };
    let arg_str = arg.as_str();
    let matches = patterns.iter().any(|p| {
        if let Some(prefix) = p.strip_suffix('*') {
            arg_str.starts_with(prefix)
        } else {
            arg_str == p
        }
    });
    if matches { Verdict::Allowed(level) } else { Verdict::Denied }
}

fn dispatch_require_any(
    tokens: &[Token],
    require_any: &[String],
    policy: &OwnedPolicy,
    level: SafetyLevel,
) -> Verdict {
    if tokens.len() == 2 && (tokens[1] == "--help" || tokens[1] == "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let has_required = tokens[1..].iter().any(|t| {
        require_any.iter().any(|r| {
            t == r.as_str() || t.as_str().starts_with(&format!("{r}="))
        })
    });
    if has_required && check_owned(tokens, policy) {
        Verdict::Allowed(level)
    } else {
        Verdict::Denied
    }
}

fn dispatch_sub(tokens: &[Token], sub: &SubSpec) -> Verdict {
    match &sub.kind {
        SubKind::Policy { policy, level } => {
            if check_owned(tokens, policy) {
                Verdict::Allowed(*level)
            } else {
                Verdict::Denied
            }
        }
        SubKind::Guarded {
            guard_long,
            guard_short,
            policy,
            level,
        } => {
            if has_flag_owned(tokens, guard_short.as_deref(), guard_long)
                && check_owned(tokens, policy)
            {
                Verdict::Allowed(*level)
            } else {
                Verdict::Denied
            }
        }
        SubKind::Nested { subs, allow_bare } => {
            if tokens.len() < 2 {
                if *allow_bare {
                    return Verdict::Allowed(SafetyLevel::Inert);
                }
                return Verdict::Denied;
            }
            let arg = tokens[1].as_str();
            if *allow_bare && (arg == "--help" || arg == "-h") {
                return Verdict::Allowed(SafetyLevel::Inert);
            }
            subs.iter()
                .find(|s| s.name == arg)
                .map(|s| dispatch_sub(&tokens[1..], s))
                .unwrap_or(Verdict::Denied)
        }
        SubKind::AllowAll { level } => Verdict::Allowed(*level),
        SubKind::WriteFlagged {
            policy,
            base_level,
            write_flags,
        } => {
            if !check_owned(tokens, policy) {
                return Verdict::Denied;
            }
            let has_write = tokens[1..].iter().any(|t| {
                write_flags.iter().any(|f| t == f.as_str() || t.as_str().starts_with(&format!("{f}=")))
            });
            if has_write {
                Verdict::Allowed(SafetyLevel::SafeWrite)
            } else {
                Verdict::Allowed(*base_level)
            }
        }
        SubKind::FirstArgFilter { patterns, level } => {
            dispatch_first_arg(tokens, patterns, *level)
        }
        SubKind::RequireAny {
            require_any,
            policy,
            level,
        } => dispatch_require_any(tokens, require_any, policy, *level),
        SubKind::DelegateAfterSeparator { separator } => {
            let sep_pos = tokens[1..].iter().position(|t| t == separator.as_str());
            let Some(pos) = sep_pos else {
                return Verdict::Denied;
            };
            let inner_start = pos + 2;
            if inner_start >= tokens.len() {
                return Verdict::Denied;
            }
            let inner = shell_words::join(tokens[inner_start..].iter().map(|t| t.as_str()));
            crate::command_verdict(&inner)
        }
        SubKind::DelegateSkip { skip, .. } => {
            if tokens.len() <= *skip {
                return Verdict::Denied;
            }
            let inner = shell_words::join(tokens[*skip..].iter().map(|t| t.as_str()));
            crate::command_verdict(&inner)
        }
        SubKind::Custom { handler_name } => {
            SUB_HANDLERS
                .get(handler_name.as_str())
                .map(|f| f(tokens))
                .unwrap_or(Verdict::Denied)
        }
    }
}

pub fn dispatch_spec(tokens: &[Token], spec: &CommandSpec) -> Verdict {
    match &spec.kind {
        CommandKind::Flat { policy, level } => {
            if check_owned(tokens, policy) {
                Verdict::Allowed(*level)
            } else {
                Verdict::Denied
            }
        }
        CommandKind::FlatRequireAny {
            require_any,
            policy,
            level,
        } => dispatch_require_any(tokens, require_any, policy, *level),
        CommandKind::Structured { bare_flags, subs } => {
            if tokens.len() < 2 {
                return Verdict::Denied;
            }
            let arg = tokens[1].as_str();
            if tokens.len() == 2 && bare_flags.iter().any(|f| f == arg) {
                return Verdict::Allowed(SafetyLevel::Inert);
            }
            subs.iter()
                .find(|s| s.name == arg)
                .map(|s| dispatch_sub(&tokens[1..], s))
                .unwrap_or(Verdict::Denied)
        }
        CommandKind::Custom { handler_name } => {
            CMD_HANDLERS
                .get(handler_name.as_str())
                .map(|f| f(tokens))
                .unwrap_or(Verdict::Denied)
        }
    }
}

use std::sync::LazyLock;

type HandlerFn = fn(&[Token]) -> Verdict;

static CMD_HANDLERS: LazyLock<HashMap<&'static str, HandlerFn>> =
    LazyLock::new(crate::handlers::custom_cmd_handlers);

static SUB_HANDLERS: LazyLock<HashMap<&'static str, HandlerFn>> =
    LazyLock::new(crate::handlers::custom_sub_handlers);

static TOML_REGISTRY: LazyLock<HashMap<String, CommandSpec>> = LazyLock::new(|| {
    let mut all = Vec::new();
    all.extend(load_toml(include_str!("../commands/ai.toml")));
    all.extend(load_toml(include_str!("../commands/android.toml")));
    all.extend(load_toml(include_str!("../commands/binary.toml")));
    all.extend(load_toml(include_str!("../commands/builtins.toml")));
    all.extend(load_toml(include_str!("../commands/containers.toml")));
    all.extend(load_toml(include_str!("../commands/data.toml")));
    all.extend(load_toml(include_str!("../commands/dotnet.toml")));
    all.extend(load_toml(include_str!("../commands/fs.toml")));
    all.extend(load_toml(include_str!("../commands/fuzzy.toml")));
    all.extend(load_toml(include_str!("../commands/go.toml")));
    all.extend(load_toml(include_str!("../commands/hash.toml")));
    all.extend(load_toml(include_str!("../commands/jvm.toml")));
    all.extend(load_toml(include_str!("../commands/magick.toml")));
    all.extend(load_toml(include_str!("../commands/net.toml")));
    all.extend(load_toml(include_str!("../commands/node.toml")));
    all.extend(load_toml(include_str!("../commands/php.toml")));
    all.extend(load_toml(include_str!("../commands/python.toml")));
    all.extend(load_toml(include_str!("../commands/ruby.toml")));
    all.extend(load_toml(include_str!("../commands/rust.toml")));
    all.extend(load_toml(include_str!("../commands/search.toml")));
    all.extend(load_toml(include_str!("../commands/swift.toml")));
    all.extend(load_toml(include_str!("../commands/sysinfo.toml")));
    all.extend(load_toml(include_str!("../commands/system.toml")));
    all.extend(load_toml(include_str!("../commands/text.toml")));
    all.extend(load_toml(include_str!("../commands/tools.toml")));
    all.extend(load_toml(include_str!("../commands/xcode.toml")));
    build_registry(all)
});

pub fn toml_dispatch(tokens: &[Token]) -> Option<Verdict> {
    let cmd = tokens[0].command_name();
    TOML_REGISTRY.get(cmd).map(|spec| dispatch_spec(tokens, spec))
}

pub fn toml_command_names() -> Vec<&'static str> {
    TOML_REGISTRY
        .keys()
        .map(|k| k.as_str())
        .collect()
}

pub fn toml_command_docs() -> Vec<crate::docs::CommandDoc> {
    TOML_REGISTRY
        .values()
        .map(|spec| spec.to_command_doc())
        .collect()
}

impl CommandSpec {
    fn to_command_doc(&self) -> crate::docs::CommandDoc {
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
    fn describe(&self) -> String {
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

    fn flag_summary(&self) -> String {
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
    fn doc_line(&self, prefix: &str, out: &mut Vec<String>) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::Token;

    fn toks(words: &[&str]) -> Vec<Token> {
        words.iter().map(|s| Token::from_test(s)).collect()
    }

    fn load_one(toml_str: &str) -> CommandSpec {
        let mut specs = load_toml(toml_str);
        assert_eq!(specs.len(), 1);
        specs.remove(0)
    }

    // ---------------------------------------------------------------
    // Flat commands
    // ---------------------------------------------------------------

    #[test]
    fn flat_bare_allowed() {
        let spec = load_one(r#"
            [[command]]
            name = "wc"
            bare = true
        "#);
        assert_eq!(dispatch_spec(&toks(&["wc"]), &spec), Verdict::Allowed(SafetyLevel::Inert));
    }

    #[test]
    fn flat_bare_denied_when_false() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
        "#);
        assert_eq!(dispatch_spec(&toks(&["grep"]), &spec), Verdict::Denied);
    }

    #[test]
    fn flat_standalone_flag() {
        let spec = load_one(r#"
            [[command]]
            name = "wc"
            bare = true
            standalone = ["-l", "--lines"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["wc", "-l", "file.txt"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn flat_unknown_flag_rejected() {
        let spec = load_one(r#"
            [[command]]
            name = "wc"
            standalone = ["-l"]
        "#);
        assert_eq!(dispatch_spec(&toks(&["wc", "--evil"]), &spec), Verdict::Denied);
    }

    #[test]
    fn flat_valued_flag_space() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
            valued = ["--max-count", "-m"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["grep", "--max-count", "5", "pattern"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn flat_valued_flag_eq() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
            valued = ["--max-count"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["grep", "--max-count=5", "pattern"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn flat_combined_short_flags() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
            standalone = ["-r", "-n", "-i"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["grep", "-rni", "pattern", "."]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn flat_combined_short_unknown_rejected() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
            standalone = ["-r", "-n"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["grep", "-rnz", "pattern"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn flat_combined_short_with_valued_last() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
            standalone = ["-r", "-n"]
            valued = ["-m"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["grep", "-rnm", "5", "pattern"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn flat_double_dash_stops_flag_checking() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
            standalone = ["-r"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["grep", "-r", "--", "--not-a-flag", "file"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn flat_max_positional_enforced() {
        let spec = load_one(r#"
            [[command]]
            name = "uniq"
            bare = true
            max_positional = 1
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["uniq", "a"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["uniq", "a", "b"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn flat_max_positional_after_double_dash() {
        let spec = load_one(r#"
            [[command]]
            name = "uniq"
            bare = true
            max_positional = 1
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["uniq", "--", "a", "b"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn flat_positional_style() {
        let spec = load_one(r#"
            [[command]]
            name = "echo"
            bare = true
            positional_style = true
            standalone = ["-n", "-e"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["echo", "--unknown", "hello"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn flat_level_safe_read() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"
            level = "SafeRead"
            bare = true
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo"]), &spec),
            Verdict::Allowed(SafetyLevel::SafeRead),
        );
    }

    #[test]
    fn flat_level_safe_write() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"
            level = "SafeWrite"
            bare = true
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo"]), &spec),
            Verdict::Allowed(SafetyLevel::SafeWrite),
        );
    }

    // ---------------------------------------------------------------
    // Structured commands with subcommands
    // ---------------------------------------------------------------

    #[test]
    fn structured_bare_rejected() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"
            bare_flags = ["--help"]

            [[command.sub]]
            name = "build"
            level = "SafeWrite"
        "#);
        assert_eq!(dispatch_spec(&toks(&["cargo"]), &spec), Verdict::Denied);
    }

    #[test]
    fn structured_bare_flag() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"
            bare_flags = ["--help", "-h"]

            [[command.sub]]
            name = "build"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "--help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn structured_bare_flag_with_extra_rejected() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"
            bare_flags = ["--help"]

            [[command.sub]]
            name = "build"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "--help", "extra"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn structured_unknown_sub_rejected() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"

            [[command.sub]]
            name = "build"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "deploy"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn structured_sub_policy() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"

            [[command.sub]]
            name = "test"
            level = "SafeRead"
            standalone = ["--release", "-h"]
            valued = ["--jobs", "-j"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "test", "--release", "-j", "4"]), &spec),
            Verdict::Allowed(SafetyLevel::SafeRead),
        );
    }

    #[test]
    fn structured_sub_unknown_flag_rejected() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"

            [[command.sub]]
            name = "test"
            standalone = ["--release"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "test", "--evil"]), &spec),
            Verdict::Denied,
        );
    }

    // ---------------------------------------------------------------
    // Guarded subcommands
    // ---------------------------------------------------------------

    #[test]
    fn guarded_with_guard() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"

            [[command.sub]]
            name = "fmt"
            guard = "--check"
            standalone = ["--all", "--check", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "fmt", "--check"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn guarded_without_guard_rejected() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"

            [[command.sub]]
            name = "fmt"
            guard = "--check"
            standalone = ["--all", "--check"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "fmt"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn guarded_with_short_form() {
        let spec = load_one(r#"
            [[command]]
            name = "cargo"

            [[command.sub]]
            name = "package"
            guard = "--list"
            guard_short = "-l"
            standalone = ["--list", "-l"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["cargo", "package", "-l"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn guarded_with_eq_syntax() {
        let spec = load_one(r#"
            [[command]]
            name = "tool"

            [[command.sub]]
            name = "sub"
            guard = "--mode"
            valued = ["--mode"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["tool", "sub", "--mode=check"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    // ---------------------------------------------------------------
    // Nested subcommands
    // ---------------------------------------------------------------

    #[test]
    fn nested_sub() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "config"

            [[command.sub.sub]]
            name = "get"
            standalone = ["--help", "-h"]

            [[command.sub.sub]]
            name = "list"
            standalone = ["--help", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "config", "get"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["mise", "config", "delete"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn nested_bare_rejected() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "config"

            [[command.sub.sub]]
            name = "get"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "config"]), &spec),
            Verdict::Denied,
        );
    }

    // ---------------------------------------------------------------
    // Nested with nested_bare = true
    // ---------------------------------------------------------------

    #[test]
    fn nested_bare_allowed_when_flag_set() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "settings"
            nested_bare = true

            [[command.sub.sub]]
            name = "get"
            standalone = ["--help", "-h", "-q", "-v"]

            [[command.sub.sub]]
            name = "list"
            standalone = ["--help", "-h", "-q", "-v"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn nested_bare_help_allowed() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "settings"
            nested_bare = true

            [[command.sub.sub]]
            name = "get"
            standalone = ["--help", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings", "--help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings", "-h"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn nested_bare_still_dispatches_to_subs() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "settings"
            nested_bare = true

            [[command.sub.sub]]
            name = "get"
            standalone = ["--help", "-h", "-q", "-v"]

            [[command.sub.sub]]
            name = "list"
            standalone = ["--help", "-h", "-q", "-v"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings", "get"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings", "list"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings", "get", "-q"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn nested_bare_rejects_unknown_sub() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "settings"
            nested_bare = true

            [[command.sub.sub]]
            name = "get"
            standalone = ["--help", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings", "set"]), &spec),
            Verdict::Denied,
        );
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings", "delete"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn nested_bare_rejects_unknown_flags() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "settings"
            nested_bare = true

            [[command.sub.sub]]
            name = "get"
            standalone = ["--help", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "settings", "--evil"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn nested_bare_false_is_default() {
        let spec = load_one(r#"
            [[command]]
            name = "npm"

            [[command.sub]]
            name = "config"

            [[command.sub.sub]]
            name = "get"
            standalone = ["--help", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["npm", "config"]), &spec),
            Verdict::Denied,
        );
    }

    // ---------------------------------------------------------------
    // AllowAll
    // ---------------------------------------------------------------

    #[test]
    fn allow_all_accepts_anything() {
        let spec = load_one(r#"
            [[command]]
            name = "git"

            [[command.sub]]
            name = "help"
            allow_all = true
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["git", "help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["git", "help", "commit", "--verbose"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    // ---------------------------------------------------------------
    // FirstArgFilter (first_arg field)
    // ---------------------------------------------------------------

    #[test]
    fn first_arg_exact_match() {
        let spec = load_one(r#"
            [[command]]
            name = "npm"
            bare_flags = ["--help", "--version", "-V", "-h"]

            [[command.sub]]
            name = "run"
            first_arg = ["test"]
            level = "SafeRead"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "test"]), &spec),
            Verdict::Allowed(SafetyLevel::SafeRead),
        );
    }

    #[test]
    fn first_arg_glob_match() {
        let spec = load_one(r#"
            [[command]]
            name = "npm"

            [[command.sub]]
            name = "run"
            first_arg = ["test", "test:*"]
            level = "SafeRead"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "test:unit"]), &spec),
            Verdict::Allowed(SafetyLevel::SafeRead),
        );
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "test:integration"]), &spec),
            Verdict::Allowed(SafetyLevel::SafeRead),
        );
    }

    #[test]
    fn first_arg_rejects_non_matching() {
        let spec = load_one(r#"
            [[command]]
            name = "npm"

            [[command.sub]]
            name = "run"
            first_arg = ["test", "test:*"]
            level = "SafeRead"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "build"]), &spec),
            Verdict::Denied,
        );
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "start"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn first_arg_rejects_bare() {
        let spec = load_one(r#"
            [[command]]
            name = "npm"

            [[command.sub]]
            name = "run"
            first_arg = ["test"]
            level = "SafeRead"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn first_arg_allows_help() {
        let spec = load_one(r#"
            [[command]]
            name = "npm"

            [[command.sub]]
            name = "run"
            first_arg = ["test"]
            level = "SafeRead"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "--help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "-h"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn first_arg_glob_does_not_match_partial() {
        let spec = load_one(r#"
            [[command]]
            name = "npm"

            [[command.sub]]
            name = "run"
            first_arg = ["test:*"]
            level = "SafeRead"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "test"]), &spec),
            Verdict::Denied,
        );
        assert_eq!(
            dispatch_spec(&toks(&["npm", "run", "testing"]), &spec),
            Verdict::Denied,
        );
    }

    // ---------------------------------------------------------------
    // RequireAny
    // ---------------------------------------------------------------

    #[test]
    fn require_any_with_required_flag() {
        let spec = load_one(r#"
            [[command]]
            name = "conda"
            bare_flags = ["--help", "--version", "-V", "-h"]

            [[command.sub]]
            name = "config"
            bare = false
            require_any = ["--show", "--show-sources"]
            standalone = ["--help", "--json", "--quiet", "--show", "--show-sources", "--verbose", "-h", "-q", "-v"]
            valued = ["--env", "--file", "--name", "--prefix", "-f", "-n", "-p"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config", "--show"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config", "--show-sources"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config", "--show", "--json"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn require_any_without_required_flag() {
        let spec = load_one(r#"
            [[command]]
            name = "conda"

            [[command.sub]]
            name = "config"
            bare = false
            require_any = ["--show", "--show-sources"]
            standalone = ["--help", "--json", "--show", "--show-sources", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config", "--json"]), &spec),
            Verdict::Denied,
        );
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn require_any_allows_help() {
        let spec = load_one(r#"
            [[command]]
            name = "conda"

            [[command.sub]]
            name = "config"
            bare = false
            require_any = ["--show"]
            standalone = ["--help", "--show", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config", "--help"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config", "-h"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn require_any_rejects_unknown_flags() {
        let spec = load_one(r#"
            [[command]]
            name = "conda"

            [[command.sub]]
            name = "config"
            bare = false
            require_any = ["--show"]
            standalone = ["--help", "--show", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config", "--show", "--evil"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn require_any_with_eq_syntax() {
        let spec = load_one(r#"
            [[command]]
            name = "tool"

            [[command.sub]]
            name = "sub"
            bare = false
            require_any = ["--mode"]
            standalone = ["--help"]
            valued = ["--mode"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["tool", "sub", "--mode=check"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn require_any_rejects_unlisted_extra_flags() {
        let spec = load_one(r#"
            [[command]]
            name = "conda"

            [[command.sub]]
            name = "config"
            bare = false
            require_any = ["--show", "--show-sources"]
            standalone = ["--help", "--show", "--show-sources", "-h"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["conda", "config", "--show", "--set"]), &spec),
            Verdict::Denied,
        );
    }

    // ---------------------------------------------------------------
    // WriteFlagged
    // ---------------------------------------------------------------

    #[test]
    fn write_flagged_base_level() {
        let spec = load_one(r#"
            [[command]]
            name = "sk"

            [[command.sub]]
            name = "run"
            write_flags = ["--history"]
            standalone = ["--help", "-h"]
            valued = ["--history", "--query", "-q"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["sk", "run", "-q", "test"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn write_flagged_with_write_flag() {
        let spec = load_one(r#"
            [[command]]
            name = "sk"

            [[command.sub]]
            name = "run"
            write_flags = ["--history"]
            standalone = ["--help"]
            valued = ["--history", "--query"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["sk", "run", "--history", "/tmp/h"]), &spec),
            Verdict::Allowed(SafetyLevel::SafeWrite),
        );
    }

    #[test]
    fn write_flagged_with_eq_syntax() {
        let spec = load_one(r#"
            [[command]]
            name = "sk"

            [[command.sub]]
            name = "run"
            write_flags = ["--history"]
            valued = ["--history"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["sk", "run", "--history=/tmp/h"]), &spec),
            Verdict::Allowed(SafetyLevel::SafeWrite),
        );
    }

    // ---------------------------------------------------------------
    // DelegateAfterSeparator
    // ---------------------------------------------------------------

    #[test]
    fn delegate_after_separator_safe() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "exec"
            delegate_after = "--"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "exec", "--", "echo", "hello"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn delegate_after_separator_unsafe() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "exec"
            delegate_after = "--"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "exec", "--", "rm", "-rf", "/"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn delegate_after_separator_no_separator() {
        let spec = load_one(r#"
            [[command]]
            name = "mise"

            [[command.sub]]
            name = "exec"
            delegate_after = "--"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["mise", "exec", "echo"]), &spec),
            Verdict::Denied,
        );
    }

    // ---------------------------------------------------------------
    // DelegateSkip
    // ---------------------------------------------------------------

    #[test]
    fn delegate_skip_safe() {
        let spec = load_one(r#"
            [[command]]
            name = "rustup"

            [[command.sub]]
            name = "run"
            delegate_skip = 2
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["rustup", "run", "stable", "echo", "hello"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn delegate_skip_unsafe() {
        let spec = load_one(r#"
            [[command]]
            name = "rustup"

            [[command.sub]]
            name = "run"
            delegate_skip = 2
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["rustup", "run", "stable", "rm", "-rf"]), &spec),
            Verdict::Denied,
        );
    }

    #[test]
    fn delegate_skip_no_inner() {
        let spec = load_one(r#"
            [[command]]
            name = "rustup"

            [[command.sub]]
            name = "run"
            delegate_skip = 2
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["rustup", "run", "stable"]), &spec),
            Verdict::Denied,
        );
    }

    // ---------------------------------------------------------------
    // Aliases
    // ---------------------------------------------------------------

    #[test]
    fn alias_dispatch() {
        let specs = load_toml(r#"
            [[command]]
            name = "grep"
            aliases = ["egrep"]
            bare = false
            standalone = ["-r"]
        "#);
        let registry = build_registry(specs);
        let spec = registry.get("egrep").expect("alias registered");
        assert_eq!(
            dispatch_spec(&toks(&["egrep", "-r", "pattern"]), spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    // ---------------------------------------------------------------
    // Custom handler reference
    // ---------------------------------------------------------------

    #[test]
    fn custom_handler_returns_denied_by_default() {
        let spec = load_one(r#"
            [[command]]
            name = "curl"
            handler = "curl"
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["curl", "http://example.com"]), &spec),
            Verdict::Denied,
        );
    }

    // ---------------------------------------------------------------
    // Multiple commands in one file
    // ---------------------------------------------------------------

    #[test]
    fn multiple_commands() {
        let specs = load_toml(r#"
            [[command]]
            name = "cat"
            bare = true
            standalone = ["-n"]

            [[command]]
            name = "head"
            bare = false
            valued = ["-n"]
        "#);
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].name, "cat");
        assert_eq!(specs[1].name, "head");
    }

    // ---------------------------------------------------------------
    // Edge cases
    // ---------------------------------------------------------------

    #[test]
    fn valued_flag_at_end_without_value() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
            valued = ["--max-count"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["grep", "--max-count"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn bare_dash_as_stdin() {
        let spec = load_one(r#"
            [[command]]
            name = "grep"
            bare = false
            standalone = ["-r"]
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["grep", "pattern", "-"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn positional_style_unknown_eq() {
        let spec = load_one(r#"
            [[command]]
            name = "echo"
            bare = true
            positional_style = true
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["echo", "--foo=bar"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn positional_style_with_max() {
        let spec = load_one(r#"
            [[command]]
            name = "echo"
            bare = true
            positional_style = true
            max_positional = 2
        "#);
        assert_eq!(
            dispatch_spec(&toks(&["echo", "--a", "--b"]), &spec),
            Verdict::Allowed(SafetyLevel::Inert),
        );
        assert_eq!(
            dispatch_spec(&toks(&["echo", "--a", "--b", "--c"]), &spec),
            Verdict::Denied,
        );
    }

    // ---------------------------------------------------------------
    // Integration: TOML registry rejects unknown flags
    // ---------------------------------------------------------------

    #[test]
    fn toml_registry_rejects_unknown_flags() {
        let mut failures = Vec::new();
        for (name, spec) in TOML_REGISTRY.iter() {
            match &spec.kind {
                CommandKind::Flat { policy, .. } | CommandKind::FlatRequireAny { policy, .. } => {
                    if policy.flag_style == FlagStyle::Positional {
                        continue;
                    }
                }
                _ => {}
            }
            let test = format!("{name} --xyzzy-unknown-42");
            if crate::is_safe_command(&test) {
                failures.push(format!("{name}: accepted unknown flag"));
            }
        }
        assert!(failures.is_empty(), "TOML commands accepted unknown flags:\n{}", failures.join("\n"));
    }

    #[test]
    fn toml_hash_commands_work() {
        assert!(crate::is_safe_command("md5sum file.txt"));
        assert!(crate::is_safe_command("sha256sum file.txt"));
        assert!(crate::is_safe_command("b2sum file.txt"));
        assert!(crate::is_safe_command("shasum -a 256 file.txt"));
        assert!(crate::is_safe_command("cksum file.txt"));
        assert!(crate::is_safe_command("md5 file.txt"));
        assert!(crate::is_safe_command("sum file.txt"));
        assert!(crate::is_safe_command("md5sum --check checksums.md5"));
    }

    #[test]
    fn toml_hash_commands_reject_unknown() {
        assert!(!crate::is_safe_command("md5sum --evil"));
        assert!(!crate::is_safe_command("sha256sum --evil"));
        assert!(!crate::is_safe_command("b2sum --evil"));
    }
}
