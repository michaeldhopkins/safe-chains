use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

use super::policy::check_owned;
use super::types::*;
use super::{CMD_HANDLERS, SUB_HANDLERS};

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
        CommandKind::Wrapper {
            standalone,
            valued,
            positional_skip,
            separator,
            bare_ok,
        } => {
            let mut i = 1;
            while i < tokens.len() {
                let t = &tokens[i];
                if let Some(sep) = separator
                    && t == sep.as_str()
                {
                    i += 1;
                    break;
                }
                if !t.starts_with('-') {
                    break;
                }
                if valued.iter().any(|f| t == f.as_str()) {
                    i += 2;
                    continue;
                }
                if standalone.iter().any(|f| t == f.as_str()) {
                    i += 1;
                    continue;
                }
                i += 1;
            }
            for _ in 0..*positional_skip {
                if i >= tokens.len() {
                    return if *bare_ok {
                        Verdict::Allowed(SafetyLevel::Inert)
                    } else {
                        Verdict::Denied
                    };
                }
                i += 1;
            }
            if i >= tokens.len() {
                return if *bare_ok {
                    Verdict::Allowed(SafetyLevel::Inert)
                } else {
                    Verdict::Denied
                };
            }
            let inner = shell_words::join(tokens[i..].iter().map(|t| t.as_str()));
            crate::command_verdict(&inner)
        }
        CommandKind::Custom { handler_name } => {
            CMD_HANDLERS
                .get(handler_name.as_str())
                .map(|f| f(tokens))
                .unwrap_or(Verdict::Denied)
        }
    }
}
