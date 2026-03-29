pub mod ai;
pub mod android;
pub mod containers;
pub mod coreutils;
pub mod dotnet;
pub mod forges;
pub mod fuzzy;
pub mod go;
pub mod jvm;
pub mod magick;
pub mod network;
pub mod node;
pub mod perl;
pub mod php;
pub mod python;
pub mod r;
pub mod ruby;
pub mod rust;
pub mod shell;
pub mod swift;
pub mod system;
pub mod vcs;
pub mod wrappers;
pub mod xcode;

use crate::parse::Token;
use crate::verdict::Verdict;

pub fn dispatch(tokens: &[Token]) -> Verdict {
    let cmd = tokens[0].command_name();
    None
        .or_else(|| shell::dispatch(cmd, tokens))
        .or_else(|| wrappers::dispatch(cmd, tokens))
        .or_else(|| vcs::dispatch(cmd, tokens))
        .or_else(|| forges::dispatch(cmd, tokens))
        .or_else(|| node::dispatch(cmd, tokens))
        .or_else(|| ruby::dispatch(cmd, tokens))
        .or_else(|| python::dispatch(cmd, tokens))
        .or_else(|| rust::dispatch(cmd, tokens))
        .or_else(|| go::dispatch(cmd, tokens))
        .or_else(|| jvm::dispatch(cmd, tokens))
        .or_else(|| android::dispatch(cmd, tokens))
        .or_else(|| php::dispatch(cmd, tokens))
        .or_else(|| swift::dispatch(cmd, tokens))
        .or_else(|| dotnet::dispatch(cmd, tokens))
        .or_else(|| containers::dispatch(cmd, tokens))
        .or_else(|| network::dispatch(cmd, tokens))
        .or_else(|| ai::dispatch(cmd, tokens))
        .or_else(|| system::dispatch(cmd, tokens))
        .or_else(|| xcode::dispatch(cmd, tokens))
        .or_else(|| perl::dispatch(cmd, tokens))
        .or_else(|| r::dispatch(cmd, tokens))
        .or_else(|| coreutils::dispatch(cmd, tokens))
        .or_else(|| fuzzy::dispatch(cmd, tokens))
        .or_else(|| magick::dispatch(cmd, tokens))
        .or_else(|| crate::registry::toml_dispatch(tokens))
        .unwrap_or(Verdict::Denied)
}

#[cfg(test)]
const HANDLED_CMDS: &[&str] = &[
    "sh", "bash", "xargs", "timeout", "time", "env", "nice", "ionice", "hyperfine", "dotenv",
    "git", "jj", "gh", "glab", "jjpr", "tea",
    "npm", "yarn", "pnpm", "bun", "deno", "npx", "bunx", "nvm", "fnm", "volta",
    "ruby", "ri", "bundle", "gem", "importmap", "rbenv",
    "pip", "uv", "poetry", "pyenv", "conda",
    "cargo", "rustup",
    "go",
    "gradle", "mvn", "mvnw", "ktlint", "detekt",
    "javap", "jar", "keytool", "jarsigner",
    "adb", "apkanalyzer", "apksigner", "bundletool", "aapt2",
    "emulator", "avdmanager", "sdkmanager", "zipalign", "lint",
    "fastlane", "firebase",
    "composer", "craft",
    "swift",
    "dotnet",
    "curl",
    "docker", "podman", "kubectl", "orbctl", "qemu-img",
    "ollama", "llm", "hf", "claude", "aider", "codex", "opencode", "vibe",
    "ddev", "dcli",
    "brew", "mise", "asdf", "crontab", "defaults", "pmset", "sysctl", "cmake", "psql", "pg_isready",
    "terraform", "heroku", "vercel", "flyctl",
    "overmind", "tailscale", "tmux", "wg",
    "networksetup", "launchctl", "diskutil", "security", "csrutil", "log",
    "xcodebuild", "plutil", "xcode-select", "xcrun", "pkgutil", "lipo", "codesign", "spctl",
    "xcodegen", "tuist", "pod", "swiftlint", "swiftformat", "periphery", "xcbeautify", "agvtool", "simctl",
    "perl",
    "R", "Rscript",
    "grep", "egrep", "fgrep", "rg", "ag", "ack", "zgrep", "zegrep", "zfgrep", "locate", "mlocate", "plocate",
    "cat", "gzcat", "head", "tail", "wc", "cut", "tr", "uniq", "less", "more", "zcat",
    "diff", "comm", "paste", "tac", "rev", "nl",
    "expand", "unexpand", "fold", "fmt", "col", "column", "iconv", "nroff",
    "echo", "printf", "seq", "test", "[", "expr", "bc", "factor", "bat",
    "arch", "command", "hostname",
    "find", "sed", "shuf", "sort", "yq", "xmllint", "awk", "gawk", "mawk", "nawk",
    "magick",
    "fd", "eza", "exa", "ls", "delta", "colordiff",
    "dirname", "basename", "realpath", "readlink",
    "file", "stat", "du", "df", "tree", "cmp", "zipinfo", "tar", "unzip", "gzip",
    "true", "false",
    "alias", "export", "printenv", "read", "type", "wait", "whereis", "which", "whoami", "date", "pwd", "cd", "unset",
    "uname", "nproc", "uptime", "id", "groups", "tty", "locale", "cal", "sleep",
    "who", "w", "last", "lastlog",
    "ps", "top", "htop", "iotop", "procs", "dust", "lsof", "pgrep", "lsblk", "free",
    "jq", "jaq", "gojq", "fx", "jless", "htmlq", "xq", "tomlq", "mlr", "dasel",
    "base64", "xxd", "getconf", "uuidgen",
    "md5sum", "md5", "sha256sum", "shasum", "sha1sum", "sha512sum",
    "cksum", "b2sum", "sum", "strings", "hexdump", "od", "size", "sips",
    "sw_vers", "mdls", "otool", "nm", "system_profiler", "ioreg", "vm_stat", "mdfind", "man",
    "dig", "nslookup", "host", "whois", "netstat", "ss", "ifconfig", "route", "ping",
    "xv",
    "fzf", "fzy", "peco", "pick", "selecta", "sk", "zf",
    "identify", "shellcheck", "cloc", "tokei", "cucumber", "branchdiff", "workon", "safe-chains",
];

pub fn handler_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
    docs.extend(vcs::command_docs());
    docs.extend(forges::command_docs());
    docs.extend(node::command_docs());
    docs.extend(ruby::command_docs());
    docs.extend(python::command_docs());
    docs.extend(rust::command_docs());
    docs.extend(go::command_docs());
    docs.extend(jvm::command_docs());
    docs.extend(android::command_docs());
    docs.extend(php::command_docs());
    docs.extend(swift::command_docs());
    docs.extend(dotnet::command_docs());
    docs.extend(containers::command_docs());
    docs.extend(ai::command_docs());
    docs.extend(network::command_docs());
    docs.extend(system::command_docs());
    docs.extend(xcode::command_docs());
    docs.extend(perl::command_docs());
    docs.extend(r::command_docs());
    docs.extend(coreutils::command_docs());
    docs.extend(fuzzy::command_docs());
    docs.extend(shell::command_docs());
    docs.extend(wrappers::command_docs());
    docs.extend(magick::command_docs());
    docs.extend(crate::registry::toml_command_docs());
    docs
}

#[cfg(test)]
#[derive(Debug)]
pub(crate) enum CommandEntry {
    Positional { cmd: &'static str },
    Custom { cmd: &'static str, valid_prefix: Option<&'static str> },
    Subcommand { cmd: &'static str, subs: &'static [SubEntry], bare_ok: bool },
    Delegation { cmd: &'static str },
}

#[cfg(test)]
#[derive(Debug)]
pub(crate) enum SubEntry {
    Policy { name: &'static str },
    Nested { name: &'static str, subs: &'static [SubEntry] },
    Custom { name: &'static str, valid_suffix: Option<&'static str> },
    Positional,
    Guarded { name: &'static str, valid_suffix: &'static str },
}

use crate::command::CommandDef;

const COMMAND_DEFS: &[&CommandDef] = &[
    &ai::CODEX, &ai::OLLAMA, &ai::OPENCODE, &ai::LLM, &ai::HF,
    &containers::DOCKER, &containers::PODMAN, &containers::KUBECTL, &containers::ORBCTL, &containers::QEMU_IMG,
    &dotnet::DOTNET,
    &go::GO,
    &android::APKANALYZER, &android::APKSIGNER, &android::BUNDLETOOL, &android::AAPT2,
    &android::AVDMANAGER,
    &jvm::GRADLE, &jvm::KEYTOOL,
    &magick::MAGICK,
    &node::NPM, &node::PNPM, &node::BUN, &node::DENO,
    &node::NVM, &node::FNM, &node::VOLTA,
    &php::COMPOSER, &php::CRAFT,
    &python::PIP, &python::UV, &python::POETRY,
    &python::PYENV, &python::CONDA,
    &ruby::BUNDLE, &ruby::GEM, &ruby::IMPORTMAP, &ruby::RBENV,
    &rust::CARGO, &rust::RUSTUP,
    &vcs::GIT,
    &swift::SWIFT,
    &system::BREW, &system::MISE, &system::ASDF, &system::DDEV, &system::DCLI, &system::CMAKE,
    &system::DEFAULTS, &system::TERRAFORM, &system::HEROKU, &system::VERCEL,
    &system::FLYCTL, &system::FASTLANE, &system::FIREBASE,
    &system::OVERMIND, &system::TAILSCALE, &system::WG,
    &system::SECURITY, &system::CSRUTIL, &system::DISKUTIL,
    &system::LAUNCHCTL, &system::LOG,
    &xcode::XCODEBUILD, &xcode::PLUTIL, &xcode::XCODE_SELECT,
    &xcode::XCODEGEN, &xcode::TUIST, &xcode::POD, &xcode::SWIFTLINT,
    &xcode::PERIPHERY, &xcode::AGVTOOL, &xcode::SIMCTL,
];

pub fn all_opencode_patterns() -> Vec<String> {
    let mut patterns = Vec::new();
    for def in COMMAND_DEFS {
        patterns.extend(def.opencode_patterns());
    }
    for def in coreutils::all_flat_defs() {
        patterns.extend(def.opencode_patterns());
    }
    for def in jvm::jvm_flat_defs() {
        patterns.extend(def.opencode_patterns());
    }
    for def in android::android_flat_defs() {
        patterns.extend(def.opencode_patterns());
    }
    for def in ai::ai_flat_defs() {
        patterns.extend(def.opencode_patterns());
    }
    for def in ruby::ruby_flat_defs() {
        patterns.extend(def.opencode_patterns());
    }
    for def in system::system_flat_defs() {
        patterns.extend(def.opencode_patterns());
    }
    for def in xcode::xcbeautify_flat_defs() {
        patterns.extend(def.opencode_patterns());
    }
    for def in fuzzy::fuzzy_flat_defs() {
        patterns.extend(def.opencode_patterns());
    }
    patterns.sort();
    patterns.dedup();
    patterns
}

#[cfg(test)]
fn full_registry() -> Vec<&'static CommandEntry> {
    let mut entries = Vec::new();
    entries.extend(shell::REGISTRY);
    entries.extend(wrappers::REGISTRY);
    entries.extend(vcs::full_registry());
    entries.extend(forges::full_registry());
    entries.extend(node::full_registry());
    entries.extend(jvm::full_registry());
    entries.extend(android::full_registry());
    entries.extend(network::REGISTRY);
    entries.extend(system::full_registry());
    entries.extend(xcode::full_registry());
    entries.extend(perl::REGISTRY);
    entries.extend(r::REGISTRY);
    entries.extend(coreutils::full_registry());
    entries.extend(fuzzy::full_registry());
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const UNKNOWN_FLAG: &str = "--xyzzy-unknown-42";
    const UNKNOWN_SUB: &str = "xyzzy-unknown-42";

    fn check_entry(entry: &CommandEntry, failures: &mut Vec<String>) {
        match entry {
            CommandEntry::Positional { .. } | CommandEntry::Delegation { .. } => {}
            CommandEntry::Custom { cmd, valid_prefix } => {
                let base = valid_prefix.unwrap_or(cmd);
                let test = format!("{base} {UNKNOWN_FLAG}");
                if crate::is_safe_command(&test) {
                    failures.push(format!("{cmd}: accepted unknown flag: {test}"));
                }
            }
            CommandEntry::Subcommand { cmd, subs, bare_ok } => {
                if !bare_ok && crate::is_safe_command(cmd) {
                    failures.push(format!("{cmd}: accepted bare invocation"));
                }
                let test = format!("{cmd} {UNKNOWN_SUB}");
                if crate::is_safe_command(&test) {
                    failures.push(format!("{cmd}: accepted unknown subcommand: {test}"));
                }
                for sub in *subs {
                    check_sub(cmd, sub, failures);
                }
            }
        }
    }

    fn check_sub(prefix: &str, entry: &SubEntry, failures: &mut Vec<String>) {
        match entry {
            SubEntry::Policy { name } => {
                let test = format!("{prefix} {name} {UNKNOWN_FLAG}");
                if crate::is_safe_command(&test) {
                    failures.push(format!("{prefix} {name}: accepted unknown flag: {test}"));
                }
            }
            SubEntry::Nested { name, subs } => {
                let path = format!("{prefix} {name}");
                let test = format!("{path} {UNKNOWN_SUB}");
                if crate::is_safe_command(&test) {
                    failures.push(format!("{path}: accepted unknown subcommand: {test}"));
                }
                for sub in *subs {
                    check_sub(&path, sub, failures);
                }
            }
            SubEntry::Custom { name, valid_suffix } => {
                let base = match valid_suffix {
                    Some(s) => format!("{prefix} {name} {s}"),
                    None => format!("{prefix} {name}"),
                };
                let test = format!("{base} {UNKNOWN_FLAG}");
                if crate::is_safe_command(&test) {
                    failures.push(format!("{prefix} {name}: accepted unknown flag: {test}"));
                }
            }
            SubEntry::Positional => {}
            SubEntry::Guarded { name, valid_suffix } => {
                let test = format!("{prefix} {name} {valid_suffix} {UNKNOWN_FLAG}");
                if crate::is_safe_command(&test) {
                    failures.push(format!("{prefix} {name}: accepted unknown flag: {test}"));
                }
            }
        }
    }

    #[test]
    fn all_commands_reject_unknown() {
        let registry = full_registry();
        let mut failures = Vec::new();
        for entry in &registry {
            check_entry(entry, &mut failures);
        }
        assert!(
            failures.is_empty(),
            "unknown flags/subcommands accepted:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn command_defs_reject_unknown() {
        for def in COMMAND_DEFS {
            def.auto_test_reject_unknown();
        }
    }

    #[test]
    fn flat_defs_reject_unknown() {
        for def in coreutils::all_flat_defs() {
            def.auto_test_reject_unknown();
        }
        for def in xcode::xcbeautify_flat_defs() {
            def.auto_test_reject_unknown();
        }
        for def in jvm::jvm_flat_defs().into_iter().chain(android::android_flat_defs()).chain(ai::ai_flat_defs()).chain(ruby::ruby_flat_defs()).chain(system::system_flat_defs()).chain(fuzzy::fuzzy_flat_defs()) {
            def.auto_test_reject_unknown();
        }
    }


    #[test]
    fn bare_false_rejects_bare_invocation() {
        let check_def = |def: &crate::command::FlatDef| {
            if !def.policy.bare {
                assert!(
                    !crate::is_safe_command(def.name),
                    "{}: bare=false but bare invocation accepted",
                    def.name,
                );
            }
        };
        for def in coreutils::all_flat_defs()
            .into_iter()
            .chain(xcode::xcbeautify_flat_defs())
        {
            check_def(def);
        }
        for def in jvm::jvm_flat_defs().into_iter().chain(android::android_flat_defs()).chain(ai::ai_flat_defs()).chain(ruby::ruby_flat_defs()).chain(system::system_flat_defs()).chain(fuzzy::fuzzy_flat_defs()) {
            check_def(def);
        }
    }

    fn visit_subs(prefix: &str, subs: &[crate::command::SubDef], visitor: &mut dyn FnMut(&str, &crate::command::SubDef)) {
        for sub in subs {
            visitor(prefix, sub);
            if let crate::command::SubDef::Nested { name, subs: inner } = sub {
                visit_subs(&format!("{prefix} {name}"), inner, visitor);
            }
        }
    }

    #[test]
    fn guarded_subs_require_guard() {
        let mut failures = Vec::new();
        for def in COMMAND_DEFS {
            visit_subs(def.name, def.subs, &mut |prefix, sub| {
                if let crate::command::SubDef::Guarded { name, guard_long, .. } = sub {
                    let without = format!("{prefix} {name}");
                    if crate::is_safe_command(&without) {
                        failures.push(format!("{without}: accepted without guard {guard_long}"));
                    }
                    let with = format!("{prefix} {name} {guard_long}");
                    if !crate::is_safe_command(&with) {
                        failures.push(format!("{with}: rejected with guard {guard_long}"));
                    }
                }
            });
        }
        assert!(failures.is_empty(), "guarded sub issues:\n{}", failures.join("\n"));
    }

    #[test]
    fn guarded_subs_accept_guard_short() {
        let mut failures = Vec::new();
        for def in COMMAND_DEFS {
            visit_subs(def.name, def.subs, &mut |prefix, sub| {
                if let crate::command::SubDef::Guarded { name, guard_short: Some(short), .. } = sub {
                    let with_short = format!("{prefix} {name} {short}");
                    if !crate::is_safe_command(&with_short) {
                        failures.push(format!("{with_short}: rejected with guard_short"));
                    }
                }
            });
        }
        assert!(failures.is_empty(), "guard_short issues:\n{}", failures.join("\n"));
    }

    #[test]
    fn nested_subs_reject_bare() {
        let mut failures = Vec::new();
        for def in COMMAND_DEFS {
            visit_subs(def.name, def.subs, &mut |prefix, sub| {
                if let crate::command::SubDef::Nested { name, .. } = sub {
                    let bare = format!("{prefix} {name}");
                    if crate::is_safe_command(&bare) {
                        failures.push(format!("{bare}: nested sub accepted bare invocation"));
                    }
                }
            });
        }
        assert!(failures.is_empty(), "nested bare issues:\n{}", failures.join("\n"));
    }

    #[test]
    fn process_substitution_blocked() {
        let cmds = ["echo <(cat /etc/passwd)", "echo >(rm -rf /)", "grep pattern <(ls)"];
        for cmd in &cmds {
            assert!(
                !crate::is_safe_command(cmd),
                "process substitution not blocked: {cmd}",
            );
        }
    }

    #[test]
    fn positional_style_accepts_unknown_args() {
        use crate::policy::FlagStyle;
        for def in coreutils::all_flat_defs() {
            if def.policy.flag_style == FlagStyle::Positional {
                let test = format!("{} --unknown-xyz", def.name);
                assert!(
                    crate::is_safe_command(&test),
                    "{}: FlagStyle::Positional but rejected unknown arg",
                    def.name,
                );
            }
        }
    }

    fn visit_policies(prefix: &str, subs: &[crate::command::SubDef], visitor: &mut dyn FnMut(&str, &crate::policy::FlagPolicy)) {
        for sub in subs {
            match sub {
                crate::command::SubDef::Policy { name, policy, .. } => {
                    visitor(&format!("{prefix} {name}"), policy);
                }
                crate::command::SubDef::Guarded { name, guard_long, policy, .. } => {
                    visitor(&format!("{prefix} {name} {guard_long}"), policy);
                }
                crate::command::SubDef::Nested { name, subs: inner } => {
                    visit_policies(&format!("{prefix} {name}"), inner, visitor);
                }
                _ => {}
            }
        }
    }

    #[test]
    fn valued_flags_accept_eq_syntax() {
        let mut failures = Vec::new();

        let check_flat = |def: &crate::command::FlatDef, failures: &mut Vec<String>| {
            for flag in def.policy.valued.iter() {
                let cmd = format!("{} {flag}=test_value", def.name);
                if !crate::is_safe_command(&cmd) {
                    failures.push(format!("{cmd}: valued flag rejected with = syntax"));
                }
            }
        };
        for def in coreutils::all_flat_defs()
            .into_iter()
            .chain(xcode::xcbeautify_flat_defs())
        {
            check_flat(def, &mut failures);
        }
        for def in jvm::jvm_flat_defs().into_iter().chain(android::android_flat_defs()).chain(ai::ai_flat_defs()).chain(ruby::ruby_flat_defs()).chain(system::system_flat_defs()).chain(fuzzy::fuzzy_flat_defs()) {
            check_flat(def, &mut failures);
        }

        for def in COMMAND_DEFS {
            visit_policies(def.name, def.subs, &mut |prefix, policy| {
                for flag in policy.valued.iter() {
                    let cmd = format!("{prefix} {flag}=test_value");
                    if !crate::is_safe_command(&cmd) {
                        failures.push(format!("{cmd}: valued flag rejected with = syntax"));
                    }
                }
            });
        }

        assert!(failures.is_empty(), "valued = syntax issues:\n{}", failures.join("\n"));
    }

    #[test]
    fn max_positional_enforced() {
        let mut failures = Vec::new();

        let check_flat = |def: &crate::command::FlatDef, failures: &mut Vec<String>| {
            if let Some(max) = def.policy.max_positional {
                let args: Vec<&str> = (0..=max).map(|_| "testarg").collect();
                let cmd = format!("{} {}", def.name, args.join(" "));
                if crate::is_safe_command(&cmd) {
                    failures.push(format!(
                        "{}: max_positional={max} but accepted {} positional args",
                        def.name,
                        max + 1,
                    ));
                }
            }
        };
        for def in coreutils::all_flat_defs()
            .into_iter()
            .chain(xcode::xcbeautify_flat_defs())
        {
            check_flat(def, &mut failures);
        }
        for def in jvm::jvm_flat_defs().into_iter().chain(android::android_flat_defs()).chain(ai::ai_flat_defs()).chain(ruby::ruby_flat_defs()).chain(system::system_flat_defs()).chain(fuzzy::fuzzy_flat_defs()) {
            check_flat(def, &mut failures);
        }

        for def in COMMAND_DEFS {
            visit_policies(def.name, def.subs, &mut |prefix, policy| {
                if let Some(max) = policy.max_positional {
                    let args: Vec<&str> = (0..=max).map(|_| "testarg").collect();
                    let cmd = format!("{prefix} {}", args.join(" "));
                    if crate::is_safe_command(&cmd) {
                        failures.push(format!(
                            "{prefix}: max_positional={max} but accepted {} positional args",
                            max + 1,
                        ));
                    }
                }
            });
        }

        assert!(failures.is_empty(), "max_positional issues:\n{}", failures.join("\n"));
    }

    #[test]
    fn doc_generation_non_empty() {
        let mut failures = Vec::new();

        for def in COMMAND_DEFS {
            let doc = def.to_doc();
            if doc.description.trim().is_empty() {
                failures.push(format!("{}: CommandDef produced empty doc", def.name));
            }
            if doc.url.is_empty() {
                failures.push(format!("{}: CommandDef has empty URL", def.name));
            }
        }

        let check_flat = |def: &crate::command::FlatDef, failures: &mut Vec<String>| {
            let doc = def.to_doc();
            if doc.description.trim().is_empty() && !def.policy.bare {
                failures.push(format!("{}: FlatDef produced empty doc", def.name));
            }
            if doc.url.is_empty() {
                failures.push(format!("{}: FlatDef has empty URL", def.name));
            }
        };
        for def in coreutils::all_flat_defs()
            .into_iter()
            .chain(xcode::xcbeautify_flat_defs())
        {
            check_flat(def, &mut failures);
        }
        for def in jvm::jvm_flat_defs().into_iter().chain(android::android_flat_defs()).chain(ai::ai_flat_defs()).chain(ruby::ruby_flat_defs()).chain(system::system_flat_defs()).chain(fuzzy::fuzzy_flat_defs()) {
            check_flat(def, &mut failures);
        }

        assert!(failures.is_empty(), "doc generation issues:\n{}", failures.join("\n"));
    }

    #[test]
    fn registry_covers_handled_commands() {
        let registry = full_registry();
        let mut all_cmds: HashSet<&str> = registry
            .iter()
            .map(|e| match e {
                CommandEntry::Positional { cmd }
                | CommandEntry::Custom { cmd, .. }
                | CommandEntry::Subcommand { cmd, .. }
                | CommandEntry::Delegation { cmd } => *cmd,
            })
            .collect();
        for def in COMMAND_DEFS {
            all_cmds.insert(def.name);
        }
        for def in coreutils::all_flat_defs() {
            all_cmds.insert(def.name);
        }
        for def in xcode::xcbeautify_flat_defs() {
            all_cmds.insert(def.name);
        }
        for def in jvm::jvm_flat_defs().into_iter().chain(android::android_flat_defs()).chain(ai::ai_flat_defs()).chain(ruby::ruby_flat_defs()).chain(system::system_flat_defs()).chain(fuzzy::fuzzy_flat_defs()) {
            all_cmds.insert(def.name);
        }
        for name in crate::registry::toml_command_names() {
            all_cmds.insert(name);
        }
        let handled: HashSet<&str> = HANDLED_CMDS.iter().copied().collect();

        let missing: Vec<_> = handled.difference(&all_cmds).collect();
        assert!(missing.is_empty(), "not in registry or COMMAND_DEFS: {missing:?}");

        let extra: Vec<_> = all_cmds.difference(&handled).collect();
        assert!(extra.is_empty(), "not in HANDLED_CMDS: {extra:?}");
    }

}
