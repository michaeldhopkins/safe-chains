use crate::parse::{has_flag, Token};
use crate::policy::{self, FlagPolicy};
use crate::verdict::{SafetyLevel, Verdict};
#[cfg(test)]
use crate::policy::FlagStyle;

pub type CheckFn = fn(&[Token]) -> Verdict;

pub enum SubDef {
    Policy {
        name: &'static str,
        policy: &'static FlagPolicy,
        level: SafetyLevel,
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
        level: SafetyLevel,
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
    pub url: &'static str,
    pub aliases: &'static [&'static str],
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

    pub fn check(&self, tokens: &[Token]) -> Verdict {
        match self {
            Self::Policy { policy, level, .. } => {
                if policy::check(tokens, policy) {
                    Verdict::Allowed(*level)
                } else {
                    Verdict::Denied
                }
            }
            Self::Nested { subs, .. } => {
                if tokens.len() < 2 {
                    return Verdict::Denied;
                }
                let sub = tokens[1].as_str();
                subs.iter()
                    .find(|s| s.name() == sub)
                    .map(|s| s.check(&tokens[1..]))
                    .unwrap_or(Verdict::Denied)
            }
            Self::Guarded {
                guard_short,
                guard_long,
                policy,
                level,
                ..
            } => {
                if has_flag(tokens, *guard_short, Some(guard_long))
                    && policy::check(tokens, policy)
                {
                    Verdict::Allowed(*level)
                } else {
                    Verdict::Denied
                }
            }
            Self::Custom { check: f, .. } => f(tokens),
            Self::Delegation { skip, .. } => {
                if tokens.len() <= *skip {
                    return Verdict::Denied;
                }
                let inner = shell_words::join(tokens[*skip..].iter().map(|t| t.as_str()));
                crate::command_verdict(&inner)
            }
        }
    }
}

impl CommandDef {
    pub fn opencode_patterns(&self) -> Vec<String> {
        let mut patterns = Vec::new();
        let names: Vec<&str> = std::iter::once(self.name)
            .chain(self.aliases.iter().copied())
            .collect();
        for name in &names {
            for sub in self.subs {
                sub_opencode_patterns(name, sub, &mut patterns);
            }
        }
        patterns
    }

    pub fn check(&self, tokens: &[Token]) -> Verdict {
        if tokens.len() < 2 {
            return Verdict::Denied;
        }
        let arg = tokens[1].as_str();
        if tokens.len() == 2 && self.bare_flags.contains(&arg) {
            return Verdict::Allowed(SafetyLevel::Inert);
        }
        self.subs
            .iter()
            .find(|s| s.name() == arg)
            .map(|s| s.check(&tokens[1..]))
            .unwrap_or(Verdict::Denied)
    }

    pub fn dispatch(
        &self,
        cmd: &str,
        tokens: &[Token],
    ) -> Option<Verdict> {
        if cmd == self.name || self.aliases.contains(&cmd) {
            Some(self.check(tokens))
        } else {
            None
        }
    }

    pub fn to_doc(&self) -> crate::docs::CommandDoc {
        let mut lines = Vec::new();

        if !self.bare_flags.is_empty() {
            lines.push(format!("- Allowed standalone flags: {}", self.bare_flags.join(", ")));
        }

        let mut sub_lines: Vec<String> = Vec::new();
        for sub in self.subs {
            sub_doc_line(sub, "", &mut sub_lines);
        }
        sub_lines.sort();
        lines.extend(sub_lines);

        let mut doc = crate::docs::CommandDoc::handler(self.name, self.url, lines.join("\n"));
        doc.aliases = self.aliases.iter().map(|a| a.to_string()).collect();
        doc
    }
}

pub struct FlatDef {
    pub name: &'static str,
    pub policy: &'static FlagPolicy,
    pub level: SafetyLevel,
    pub url: &'static str,
    pub aliases: &'static [&'static str],
}

impl FlatDef {
    pub fn opencode_patterns(&self) -> Vec<String> {
        let mut patterns = Vec::new();
        let names: Vec<&str> = std::iter::once(self.name)
            .chain(self.aliases.iter().copied())
            .collect();
        for name in names {
            patterns.push(name.to_string());
            patterns.push(format!("{name} *"));
        }
        patterns
    }

    pub fn dispatch(&self, cmd: &str, tokens: &[Token]) -> Option<Verdict> {
        if cmd == self.name || self.aliases.contains(&cmd) {
            if policy::check(tokens, self.policy) {
                Some(Verdict::Allowed(self.level))
            } else {
                Some(Verdict::Denied)
            }
        } else {
            None
        }
    }

    pub fn to_doc(&self) -> crate::docs::CommandDoc {
        let mut doc = crate::docs::CommandDoc::handler(self.name, self.url, self.policy.describe());
        doc.aliases = self.aliases.iter().map(|a| a.to_string()).collect();
        doc
    }
}

#[cfg(test)]
impl FlatDef {
    pub fn auto_test_reject_unknown(&self) {
        if self.policy.flag_style == FlagStyle::Positional {
            return;
        }
        let test = format!("{} --xyzzy-unknown-42", self.name);
        assert!(
            !crate::is_safe_command(&test),
            "{}: accepted unknown flag: {test}",
            self.name,
        );
        for alias in self.aliases {
            let test = format!("{alias} --xyzzy-unknown-42");
            assert!(
                !crate::is_safe_command(&test),
                "{alias}: alias accepted unknown flag: {test}",
            );
        }
    }
}

fn sub_opencode_patterns(prefix: &str, sub: &SubDef, out: &mut Vec<String>) {
    match sub {
        SubDef::Policy { name, .. } => {
            out.push(format!("{prefix} {name}"));
            out.push(format!("{prefix} {name} *"));
        }
        SubDef::Nested { name, subs } => {
            let path = format!("{prefix} {name}");
            for s in *subs {
                sub_opencode_patterns(&path, s, out);
            }
        }
        SubDef::Guarded {
            name, guard_long, ..
        } => {
            out.push(format!("{prefix} {name} {guard_long}"));
            out.push(format!("{prefix} {name} {guard_long} *"));
        }
        SubDef::Custom { name, .. } => {
            out.push(format!("{prefix} {name}"));
            out.push(format!("{prefix} {name} *"));
        }
        SubDef::Delegation { .. } => {}
    }
}

fn sub_doc_line(sub: &SubDef, prefix: &str, out: &mut Vec<String>) {
    match sub {
        SubDef::Policy { name, policy, .. } => {
            let summary = policy.flag_summary();
            let label = if prefix.is_empty() {
                (*name).to_string()
            } else {
                format!("{prefix} {name}")
            };
            if summary.is_empty() {
                out.push(format!("- **{label}**"));
            } else {
                out.push(format!("- **{label}**: {summary}"));
            }
        }
        SubDef::Nested { name, subs } => {
            let path = if prefix.is_empty() {
                (*name).to_string()
            } else {
                format!("{prefix} {name}")
            };
            for s in *subs {
                sub_doc_line(s, &path, out);
            }
        }
        SubDef::Guarded {
            name,
            guard_long,
            policy,
            ..
        } => {
            let summary = policy.flag_summary();
            let label = if prefix.is_empty() {
                (*name).to_string()
            } else {
                format!("{prefix} {name}")
            };
            if summary.is_empty() {
                out.push(format!("- **{label}** (requires {guard_long})"));
            } else {
                out.push(format!("- **{label}** (requires {guard_long}): {summary}"));
            }
        }
        SubDef::Custom { name, doc, .. } => {
            if !doc.is_empty() && doc.trim().is_empty() {
                return;
            }
            let label = if prefix.is_empty() {
                (*name).to_string()
            } else {
                format!("{prefix} {name}")
            };
            if doc.is_empty() {
                out.push(format!("- **{label}**"));
            } else {
                out.push(format!("- **{label}**: {doc}"));
            }
        }
        SubDef::Delegation { name, doc, .. } => {
            if doc.is_empty() {
                return;
            }
            let label = if prefix.is_empty() {
                (*name).to_string()
            } else {
                format!("{prefix} {name}")
            };
            out.push(format!("- **{label}**: {doc}"));
        }
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
        SubDef::Policy { name, policy, .. } => {
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


    static TEST_POLICY: FlagPolicy = FlagPolicy {
        standalone: WordSet::new(&["--help", "--verbose", "-h", "-v"]),
        valued: WordSet::new(&["--output", "-o"]),
        bare: true,
        max_positional: None,
        flag_style: FlagStyle::Strict,
    };

    static SIMPLE_CMD: CommandDef = CommandDef {
        name: "mycmd",
        subs: &[SubDef::Policy {
            name: "build",
            policy: &TEST_POLICY,
            level: SafetyLevel::SafeWrite,
        }],
        bare_flags: &["--help", "--info", "--version", "-V", "-h"],
        url: "",
        aliases: &[],
    };

    #[test]
    fn bare_rejected() {
        assert_eq!(SIMPLE_CMD.check(&toks(&["mycmd"])), Verdict::Denied);
    }

    #[test]
    fn bare_flag_accepted() {
        assert_eq!(
            SIMPLE_CMD.check(&toks(&["mycmd", "--info"])),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn bare_flag_with_extra_rejected() {
        assert_eq!(
            SIMPLE_CMD.check(&toks(&["mycmd", "--info", "extra"])),
            Verdict::Denied,
        );
    }

    #[test]
    fn policy_sub_bare() {
        assert_eq!(
            SIMPLE_CMD.check(&toks(&["mycmd", "build"])),
            Verdict::Allowed(SafetyLevel::SafeWrite),
        );
    }

    #[test]
    fn policy_sub_with_flag() {
        assert_eq!(
            SIMPLE_CMD.check(&toks(&["mycmd", "build", "--verbose"])),
            Verdict::Allowed(SafetyLevel::SafeWrite),
        );
    }

    #[test]
    fn policy_sub_unknown_flag() {
        assert_eq!(
            SIMPLE_CMD.check(&toks(&["mycmd", "build", "--bad"])),
            Verdict::Denied,
        );
    }

    #[test]
    fn unknown_sub_rejected() {
        assert_eq!(
            SIMPLE_CMD.check(&toks(&["mycmd", "deploy"])),
            Verdict::Denied,
        );
    }

    #[test]
    fn dispatch_matches() {
        assert_eq!(
            SIMPLE_CMD.dispatch("mycmd", &toks(&["mycmd", "build"])),
            Some(Verdict::Allowed(SafetyLevel::SafeWrite)),
        );
    }

    #[test]
    fn dispatch_no_match() {
        assert_eq!(
            SIMPLE_CMD.dispatch("other", &toks(&["other", "build"])),
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
                level: SafetyLevel::Inert,
            }],
        }],
        bare_flags: &[],
        url: "",
        aliases: &[],
    };

    #[test]
    fn nested_sub() {
        assert!(NESTED_CMD.check(&toks(&["nested", "package", "describe"])).is_allowed());
    }

    #[test]
    fn nested_sub_with_flag() {
        assert!(NESTED_CMD.check(
            &toks(&["nested", "package", "describe", "--verbose"]),
        ).is_allowed());
    }

    #[test]
    fn nested_bare_rejected() {
        assert_eq!(
            NESTED_CMD.check(&toks(&["nested", "package"])),
            Verdict::Denied,
        );
    }

    #[test]
    fn nested_unknown_sub_rejected() {
        assert_eq!(
            NESTED_CMD.check(&toks(&["nested", "package", "deploy"])),
            Verdict::Denied,
        );
    }

    static GUARDED_POLICY: FlagPolicy = FlagPolicy {
        standalone: WordSet::new(&["--all", "--check", "--help", "-h"]),
        valued: WordSet::new(&[]),
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
            level: SafetyLevel::Inert,
        }],
        bare_flags: &[],
        url: "",
        aliases: &[],
    };

    #[test]
    fn guarded_with_guard() {
        assert!(GUARDED_CMD.check(&toks(&["guarded", "fmt", "--check"])).is_allowed());
    }

    #[test]
    fn guarded_without_guard() {
        assert_eq!(
            GUARDED_CMD.check(&toks(&["guarded", "fmt"])),
            Verdict::Denied,
        );
    }

    #[test]
    fn guarded_with_guard_and_flag() {
        assert!(GUARDED_CMD.check(
            &toks(&["guarded", "fmt", "--check", "--all"]),
        ).is_allowed());
    }

    static DELEGATION_CMD: CommandDef = CommandDef {
        name: "runner",
        subs: &[SubDef::Delegation {
            name: "run",
            skip: 2,
            doc: "run delegates to inner command.",
        }],
        bare_flags: &[],
        url: "",
        aliases: &[],
    };

    #[test]
    fn delegation_safe_inner() {
        assert!(DELEGATION_CMD.check(
            &toks(&["runner", "run", "stable", "echo", "hello"]),
        ).is_allowed());
    }

    #[test]
    fn delegation_unsafe_inner() {
        assert_eq!(
            DELEGATION_CMD.check(&toks(&["runner", "run", "stable", "rm", "-rf"])),
            Verdict::Denied,
        );
    }

    #[test]
    fn delegation_no_inner() {
        assert_eq!(
            DELEGATION_CMD.check(&toks(&["runner", "run", "stable"])),
            Verdict::Denied,
        );
    }

    fn custom_check(tokens: &[Token]) -> Verdict {
        if tokens.len() >= 2 && tokens[1] == "safe" {
            Verdict::Allowed(SafetyLevel::Inert)
        } else {
            Verdict::Denied
        }
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
        url: "",
        aliases: &[],
    };

    #[test]
    fn custom_passes() {
        assert!(CUSTOM_CMD.check(&toks(&["custom", "special", "safe"])).is_allowed());
    }

    #[test]
    fn custom_fails() {
        assert_eq!(
            CUSTOM_CMD.check(&toks(&["custom", "special", "bad"])),
            Verdict::Denied,
        );
    }

    #[test]
    fn help_on_sub_uses_sub_level() {
        assert_eq!(
            SIMPLE_CMD.check(&toks(&["mycmd", "build", "--help"])),
            Verdict::Allowed(SafetyLevel::SafeWrite),
        );
    }

    #[test]
    fn help_on_command_uses_bare_flags() {
        assert_eq!(
            SIMPLE_CMD.check(&toks(&["mycmd", "--help"])),
            Verdict::Allowed(SafetyLevel::Inert),
        );
    }

    #[test]
    fn doc_simple() {
        let doc = SIMPLE_CMD.to_doc();
        assert_eq!(doc.name, "mycmd");
        assert_eq!(
            doc.description,
            "- Allowed standalone flags: --help, --info, --version, -V, -h\n- **build**: Flags: --help, --verbose, -h, -v. Valued: --output, -o"
        );
    }

    #[test]
    fn doc_nested() {
        let doc = NESTED_CMD.to_doc();
        assert_eq!(
            doc.description,
            "- **package describe**: Flags: --help, --verbose, -h, -v. Valued: --output, -o"
        );
    }

    #[test]
    fn doc_guarded() {
        let doc = GUARDED_CMD.to_doc();
        assert_eq!(
            doc.description,
            "- **fmt** (requires --check): Flags: --all, --check, --help, -h"
        );
    }

    #[test]
    fn doc_delegation() {
        let doc = DELEGATION_CMD.to_doc();
        assert_eq!(doc.description, "- **run**: run delegates to inner command.");
    }

    #[test]
    fn doc_custom() {
        let doc = CUSTOM_CMD.to_doc();
        assert_eq!(doc.description, "- **special**: special (safe only).");
    }

    #[test]
    fn opencode_patterns_simple() {
        let patterns = SIMPLE_CMD.opencode_patterns();
        assert!(patterns.contains(&"mycmd build".to_string()));
        assert!(patterns.contains(&"mycmd build *".to_string()));
    }

    #[test]
    fn opencode_patterns_nested() {
        let patterns = NESTED_CMD.opencode_patterns();
        assert!(patterns.contains(&"nested package describe".to_string()));
        assert!(patterns.contains(&"nested package describe *".to_string()));
        assert!(!patterns.iter().any(|p| p == "nested package"));
    }

    #[test]
    fn opencode_patterns_guarded() {
        let patterns = GUARDED_CMD.opencode_patterns();
        assert!(patterns.contains(&"guarded fmt --check".to_string()));
        assert!(patterns.contains(&"guarded fmt --check *".to_string()));
        assert!(!patterns.iter().any(|p| p == "guarded fmt"));
    }

    #[test]
    fn opencode_patterns_delegation_skipped() {
        let patterns = DELEGATION_CMD.opencode_patterns();
        assert!(patterns.is_empty());
    }

    #[test]
    fn opencode_patterns_custom() {
        let patterns = CUSTOM_CMD.opencode_patterns();
        assert!(patterns.contains(&"custom special".to_string()));
        assert!(patterns.contains(&"custom special *".to_string()));
    }

    #[test]
    fn opencode_patterns_aliases() {
        static ALIASED: CommandDef = CommandDef {
            name: "primary",
            subs: &[SubDef::Policy {
                name: "list",
                policy: &TEST_POLICY,
                level: SafetyLevel::Inert,
            }],
            bare_flags: &[],
            url: "",
            aliases: &["alt"],
        };
        let patterns = ALIASED.opencode_patterns();
        assert!(patterns.contains(&"primary list".to_string()));
        assert!(patterns.contains(&"alt list".to_string()));
        assert!(patterns.contains(&"alt list *".to_string()));
    }

    #[test]
    fn flat_def_opencode_patterns() {
        static FLAT: FlatDef = FlatDef {
            name: "grep",
            policy: &TEST_POLICY,
            level: SafetyLevel::Inert,
            url: "",
            aliases: &["rg"],
        };
        let patterns = FLAT.opencode_patterns();
        assert_eq!(patterns, vec!["grep", "grep *", "rg", "rg *"]);
    }
}
