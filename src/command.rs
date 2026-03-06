use crate::parse::{has_flag, Segment, Token};
use crate::policy::{self, FlagPolicy};
#[cfg(test)]
use crate::policy::FlagStyle;

pub type CheckFn = fn(&[Token], &dyn Fn(&Segment) -> bool) -> bool;

pub enum SubDef {
    Policy {
        name: &'static str,
        policy: &'static FlagPolicy,
    },
    Nested {
        name: &'static str,
        subs: &'static [SubDef],
    },
    Guarded {
        name: &'static str,
        guard_short: Option<&'static str>,
        guard_long: &'static str,
        policy: &'static FlagPolicy,
    },
    Custom {
        name: &'static str,
        check: CheckFn,
        doc: &'static str,
        test_suffix: Option<&'static str>,
    },
    Delegation {
        name: &'static str,
        skip: usize,
        doc: &'static str,
    },
}

pub struct CommandDef {
    pub name: &'static str,
    pub subs: &'static [SubDef],
    pub bare_flags: &'static [&'static str],
    pub help_eligible: bool,
}

impl SubDef {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Policy { name, .. }
            | Self::Nested { name, .. }
            | Self::Guarded { name, .. }
            | Self::Custom { name, .. }
            | Self::Delegation { name, .. } => name,
        }
    }

    pub fn check(&self, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
        match self {
            Self::Policy { policy, .. } => policy::check(tokens, policy),
            Self::Nested { subs, .. } => {
                if tokens.len() < 2 {
                    return false;
                }
                let sub = tokens[1].as_str();
                subs.iter()
                    .any(|s| s.name() == sub && s.check(&tokens[1..], is_safe))
            }
            Self::Guarded {
                guard_short,
                guard_long,
                policy,
                ..
            } => {
                has_flag(tokens, *guard_short, Some(guard_long))
                    && policy::check(tokens, policy)
            }
            Self::Custom { check: f, .. } => f(tokens, is_safe),
            Self::Delegation { skip, .. } => {
                if tokens.len() <= *skip {
                    return false;
                }
                let inner = Token::join(&tokens[*skip..]);
                is_safe(&inner)
            }
        }
    }
}

impl CommandDef {
    pub fn check(&self, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
        if tokens.len() < 2 {
            return false;
        }
        let arg = tokens[1].as_str();
        if tokens.len() == 2 && self.bare_flags.contains(&arg) {
            return true;
        }
        self.subs
            .iter()
            .find(|s| s.name() == arg)
            .is_some_and(|s| s.check(&tokens[1..], is_safe))
    }

    pub fn dispatch(
        &self,
        cmd: &str,
        tokens: &[Token],
        is_safe: &dyn Fn(&Segment) -> bool,
    ) -> Option<bool> {
        if cmd == self.name {
            Some(self.check(tokens, is_safe))
        } else {
            None
        }
    }

    pub fn to_doc(&self) -> crate::docs::CommandDoc {
        let mut parts = Vec::new();

        let mut policy_names: Vec<&str> = Vec::new();
        let mut nested_names: Vec<String> = Vec::new();
        let mut guarded_descs: Vec<String> = Vec::new();
        let mut extra_docs: Vec<&str> = Vec::new();

        for sub in self.subs {
            match sub {
                SubDef::Policy { name, .. } => {
                    policy_names.push(name);
                }
                SubDef::Nested { name, subs } => {
                    let visible: Vec<_> = subs.iter()
                        .filter(|s| !s.name().starts_with('-'))
                        .collect();
                    if visible.len() <= 5 {
                        for s in &visible {
                            nested_names.push(format!("{name} {}", s.name()));
                        }
                    } else {
                        nested_names.push((*name).to_string());
                    }
                }
                SubDef::Guarded { name, guard_long, .. } => {
                    guarded_descs.push(format!("{name} (requires {guard_long})"));
                }
                SubDef::Custom { name, doc, .. } => {
                    if doc.is_empty() {
                        policy_names.push(name);
                    } else if !doc.trim().is_empty() {
                        extra_docs.push(doc);
                    }
                }
                SubDef::Delegation { doc, .. } => {
                    if !doc.is_empty() {
                        extra_docs.push(doc);
                    }
                }
            }
        }

        if !policy_names.is_empty() {
            policy_names.sort();
            parts.push(format!("Subcommands: {}.", policy_names.join(", ")));
        }

        if !nested_names.is_empty() {
            nested_names.sort();
            parts.push(format!("Multi-level: {}.", nested_names.join(", ")));
        }

        if !self.bare_flags.is_empty() {
            parts.push(format!("Info flags: {}.", self.bare_flags.join(", ")));
        }

        if !guarded_descs.is_empty() {
            parts.push(format!("{}.", guarded_descs.join(", ")));
        }

        for doc in extra_docs {
            parts.push(doc.to_string());
        }

        crate::docs::CommandDoc::handler(self.name, parts.join(" "))
    }
}

#[cfg(test)]
impl CommandDef {
    pub fn auto_test_reject_unknown(&self) {
        let mut failures = Vec::new();

        assert!(
            !crate::is_safe_command(self.name),
            "{}: accepted bare invocation",
            self.name,
        );

        let test = format!("{} xyzzy-unknown-42", self.name);
        assert!(
            !crate::is_safe_command(&test),
            "{}: accepted unknown subcommand: {test}",
            self.name,
        );

        for sub in self.subs {
            auto_test_sub(self.name, sub, &mut failures);
        }
        assert!(
            failures.is_empty(),
            "{}: unknown flags/subcommands accepted:\n{}",
            self.name,
            failures.join("\n"),
        );
    }
}

#[cfg(test)]
fn auto_test_sub(prefix: &str, sub: &SubDef, failures: &mut Vec<String>) {
    const UNKNOWN: &str = "--xyzzy-unknown-42";

    match sub {
        SubDef::Policy { name, policy } => {
            if policy.flag_style == FlagStyle::Positional {
                return;
            }
            let test = format!("{prefix} {name} {UNKNOWN}");
            if crate::is_safe_command(&test) {
                failures.push(format!("{prefix} {name}: accepted unknown flag: {test}"));
            }
        }
        SubDef::Nested { name, subs } => {
            let path = format!("{prefix} {name}");
            let test = format!("{path} xyzzy-unknown-42");
            if crate::is_safe_command(&test) {
                failures.push(format!("{path}: accepted unknown subcommand: {test}"));
            }
            for s in *subs {
                auto_test_sub(&path, s, failures);
            }
        }
        SubDef::Guarded {
            name, guard_long, ..
        } => {
            let test = format!("{prefix} {name} {guard_long} {UNKNOWN}");
            if crate::is_safe_command(&test) {
                failures.push(format!("{prefix} {name}: accepted unknown flag: {test}"));
            }
        }
        SubDef::Custom {
            name, test_suffix, ..
        } => {
            if let Some(suffix) = test_suffix {
                let test = format!("{prefix} {name} {suffix} {UNKNOWN}");
                if crate::is_safe_command(&test) {
                    failures.push(format!(
                        "{prefix} {name}: accepted unknown flag: {test}"
                    ));
                }
            }
        }
        SubDef::Delegation { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::WordSet;
    use crate::policy::FlagStyle;

    fn toks(words: &[&str]) -> Vec<Token> {
        words.iter().map(|s| Token::from_test(s)).collect()
    }

    fn no_safe(_: &Segment) -> bool {
        false
    }

    static TEST_POLICY: FlagPolicy = FlagPolicy {
        standalone: WordSet::new(&["--verbose"]),
        standalone_short: b"v",
        valued: WordSet::new(&["--output"]),
        valued_short: b"o",
        bare: true,
        max_positional: None,
        flag_style: FlagStyle::Strict,
    };

    static SIMPLE_CMD: CommandDef = CommandDef {
        name: "mycmd",
        subs: &[SubDef::Policy {
            name: "build",
            policy: &TEST_POLICY,
        }],
        bare_flags: &["--info"],
        help_eligible: true,
    };

    #[test]
    fn bare_rejected() {
        assert!(!SIMPLE_CMD.check(&toks(&["mycmd"]), &no_safe));
    }

    #[test]
    fn bare_flag_accepted() {
        assert!(SIMPLE_CMD.check(&toks(&["mycmd", "--info"]), &no_safe));
    }

    #[test]
    fn bare_flag_with_extra_rejected() {
        assert!(!SIMPLE_CMD.check(&toks(&["mycmd", "--info", "extra"]), &no_safe));
    }

    #[test]
    fn policy_sub_bare() {
        assert!(SIMPLE_CMD.check(&toks(&["mycmd", "build"]), &no_safe));
    }

    #[test]
    fn policy_sub_with_flag() {
        assert!(SIMPLE_CMD.check(&toks(&["mycmd", "build", "--verbose"]), &no_safe));
    }

    #[test]
    fn policy_sub_unknown_flag() {
        assert!(!SIMPLE_CMD.check(&toks(&["mycmd", "build", "--bad"]), &no_safe));
    }

    #[test]
    fn unknown_sub_rejected() {
        assert!(!SIMPLE_CMD.check(&toks(&["mycmd", "deploy"]), &no_safe));
    }

    #[test]
    fn dispatch_matches() {
        assert_eq!(
            SIMPLE_CMD.dispatch("mycmd", &toks(&["mycmd", "build"]), &no_safe),
            Some(true)
        );
    }

    #[test]
    fn dispatch_no_match() {
        assert_eq!(
            SIMPLE_CMD.dispatch("other", &toks(&["other", "build"]), &no_safe),
            None
        );
    }

    static NESTED_CMD: CommandDef = CommandDef {
        name: "nested",
        subs: &[SubDef::Nested {
            name: "package",
            subs: &[SubDef::Policy {
                name: "describe",
                policy: &TEST_POLICY,
            }],
        }],
        bare_flags: &[],
        help_eligible: false,
    };

    #[test]
    fn nested_sub() {
        assert!(NESTED_CMD.check(&toks(&["nested", "package", "describe"]), &no_safe));
    }

    #[test]
    fn nested_sub_with_flag() {
        assert!(NESTED_CMD.check(
            &toks(&["nested", "package", "describe", "--verbose"]),
            &no_safe,
        ));
    }

    #[test]
    fn nested_bare_rejected() {
        assert!(!NESTED_CMD.check(&toks(&["nested", "package"]), &no_safe));
    }

    #[test]
    fn nested_unknown_sub_rejected() {
        assert!(!NESTED_CMD.check(&toks(&["nested", "package", "deploy"]), &no_safe));
    }

    static GUARDED_POLICY: FlagPolicy = FlagPolicy {
        standalone: WordSet::new(&["--all", "--check"]),
        standalone_short: b"",
        valued: WordSet::new(&[]),
        valued_short: b"",
        bare: false,
        max_positional: None,
        flag_style: FlagStyle::Strict,
    };

    static GUARDED_CMD: CommandDef = CommandDef {
        name: "guarded",
        subs: &[SubDef::Guarded {
            name: "fmt",
            guard_short: None,
            guard_long: "--check",
            policy: &GUARDED_POLICY,
        }],
        bare_flags: &[],
        help_eligible: false,
    };

    #[test]
    fn guarded_with_guard() {
        assert!(GUARDED_CMD.check(&toks(&["guarded", "fmt", "--check"]), &no_safe));
    }

    #[test]
    fn guarded_without_guard() {
        assert!(!GUARDED_CMD.check(&toks(&["guarded", "fmt"]), &no_safe));
    }

    #[test]
    fn guarded_with_guard_and_flag() {
        assert!(GUARDED_CMD.check(
            &toks(&["guarded", "fmt", "--check", "--all"]),
            &no_safe,
        ));
    }

    fn safe_echo(seg: &Segment) -> bool {
        seg.as_str() == "echo hello"
    }

    static DELEGATION_CMD: CommandDef = CommandDef {
        name: "runner",
        subs: &[SubDef::Delegation {
            name: "run",
            skip: 2,
            doc: "run delegates to inner command.",
        }],
        bare_flags: &[],
        help_eligible: false,
    };

    #[test]
    fn delegation_safe_inner() {
        assert!(DELEGATION_CMD.check(
            &toks(&["runner", "run", "stable", "echo", "hello"]),
            &safe_echo,
        ));
    }

    #[test]
    fn delegation_unsafe_inner() {
        assert!(!DELEGATION_CMD.check(
            &toks(&["runner", "run", "stable", "rm", "-rf"]),
            &no_safe,
        ));
    }

    #[test]
    fn delegation_no_inner() {
        assert!(!DELEGATION_CMD.check(
            &toks(&["runner", "run", "stable"]),
            &no_safe,
        ));
    }

    fn custom_check(tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> bool {
        tokens.len() >= 2 && tokens[1] == "safe"
    }

    static CUSTOM_CMD: CommandDef = CommandDef {
        name: "custom",
        subs: &[SubDef::Custom {
            name: "special",
            check: custom_check,
            doc: "special (safe only).",
            test_suffix: Some("safe"),
        }],
        bare_flags: &[],
        help_eligible: false,
    };

    #[test]
    fn custom_passes() {
        assert!(CUSTOM_CMD.check(&toks(&["custom", "special", "safe"]), &no_safe));
    }

    #[test]
    fn custom_fails() {
        assert!(!CUSTOM_CMD.check(&toks(&["custom", "special", "bad"]), &no_safe));
    }

    #[test]
    fn doc_simple() {
        let doc = SIMPLE_CMD.to_doc();
        assert_eq!(doc.name, "mycmd");
        assert_eq!(doc.description, "Subcommands: build. Info flags: --info.");
    }

    #[test]
    fn doc_nested() {
        let doc = NESTED_CMD.to_doc();
        assert_eq!(doc.description, "Multi-level: package describe.");
    }

    #[test]
    fn doc_guarded() {
        let doc = GUARDED_CMD.to_doc();
        assert_eq!(doc.description, "fmt (requires --check).");
    }

    #[test]
    fn doc_delegation() {
        let doc = DELEGATION_CMD.to_doc();
        assert_eq!(doc.description, "run delegates to inner command.");
    }

    #[test]
    fn doc_custom() {
        let doc = CUSTOM_CMD.to_doc();
        assert_eq!(doc.description, "special (safe only).");
    }
}
