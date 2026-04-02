use serde::Deserialize;

use crate::policy::FlagStyle;
use crate::verdict::SafetyLevel;

#[derive(Debug, Deserialize)]
pub(super) struct TomlFile {
    pub command: Vec<TomlCommand>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TomlCommand {
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub level: Option<TomlLevel>,
    #[serde(default)]
    pub bare: Option<bool>,
    #[serde(default)]
    pub max_positional: Option<usize>,
    #[serde(default)]
    pub positional_style: Option<bool>,
    #[serde(default)]
    pub standalone: Vec<String>,
    #[serde(default)]
    pub valued: Vec<String>,
    #[serde(default)]
    pub bare_flags: Vec<String>,
    #[serde(default)]
    pub sub: Vec<TomlSub>,
    #[serde(default)]
    pub handler: Option<String>,
    #[serde(default)]
    pub require_any: Vec<String>,
    #[serde(default)]
    pub first_arg: Vec<String>,
    #[serde(default)]
    pub wrapper: Option<TomlWrapper>,
    #[allow(dead_code)]
    #[serde(default)]
    pub doc: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TomlWrapper {
    #[serde(default)]
    pub standalone: Vec<String>,
    #[serde(default)]
    pub valued: Vec<String>,
    #[serde(default)]
    pub positional_skip: Option<usize>,
    #[serde(default)]
    pub separator: Option<String>,
    #[serde(default)]
    pub bare_ok: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TomlSub {
    pub name: String,
    #[serde(default)]
    pub level: Option<TomlLevel>,
    #[serde(default)]
    pub bare: Option<bool>,
    #[serde(default)]
    pub max_positional: Option<usize>,
    #[serde(default)]
    pub positional_style: Option<bool>,
    #[serde(default)]
    pub standalone: Vec<String>,
    #[serde(default)]
    pub valued: Vec<String>,
    #[serde(default)]
    pub guard: Option<String>,
    #[serde(default)]
    pub guard_short: Option<String>,
    #[serde(default)]
    pub allow_all: Option<bool>,
    #[serde(default)]
    pub sub: Vec<TomlSub>,
    #[serde(default)]
    pub nested_bare: Option<bool>,
    #[serde(default)]
    pub require_any: Vec<String>,
    #[serde(default)]
    pub first_arg: Vec<String>,
    #[serde(default)]
    pub write_flags: Vec<String>,
    #[serde(default)]
    pub delegate_after: Option<String>,
    #[serde(default)]
    pub delegate_skip: Option<usize>,
    #[serde(default)]
    pub handler: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub(super) enum TomlLevel {
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
    pub(super) kind: DispatchKind,
}

#[derive(Debug)]
pub(super) struct SubSpec {
    pub name: String,
    pub kind: DispatchKind,
}

#[derive(Debug)]
pub(super) enum DispatchKind {
    Policy {
        policy: OwnedPolicy,
        level: SafetyLevel,
    },
    FirstArg {
        patterns: Vec<String>,
        level: SafetyLevel,
    },
    RequireAny {
        require_any: Vec<String>,
        policy: OwnedPolicy,
        level: SafetyLevel,
        accept_bare_help: bool,
    },
    Branching {
        subs: Vec<SubSpec>,
        bare_flags: Vec<String>,
        bare_ok: bool,
        pre_standalone: Vec<String>,
        pre_valued: Vec<String>,
        first_arg: Vec<String>,
        first_arg_level: SafetyLevel,
    },
    WriteFlagged {
        policy: OwnedPolicy,
        base_level: SafetyLevel,
        write_flags: Vec<String>,
    },
    DelegateAfterSeparator {
        separator: String,
    },
    DelegateSkip {
        skip: usize,
    },
    Wrapper {
        standalone: Vec<String>,
        valued: Vec<String>,
        positional_skip: usize,
        separator: Option<String>,
        bare_ok: bool,
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
