pub mod ai;
pub mod containers;
pub mod coreutils;
pub mod dotnet;
pub mod forges;
pub mod go;
pub mod jvm;
pub mod magick;
pub mod network;
pub mod node;
pub mod perl;
pub mod php;
pub mod python;
pub mod ruby;
pub mod rust;
pub mod shell;
pub mod swift;
pub mod system;
pub mod vcs;
pub mod wrappers;
pub mod xcode;

use std::collections::HashSet;
use std::sync::LazyLock;

use crate::parse::{Segment, Token};

fn is_bare_info_request(tokens: &[Token]) -> bool {
    tokens.len() == 2 && (tokens[1] == "--version" || tokens[1] == "--help")
}

static HELP_ELIGIBLE: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "asdf",
        "brew", "bun", "bundle",
        "cargo", "cmake", "codesign", "command", "composer", "conda", "csrutil", "curl",
        "defaults", "deno", "diskutil", "dotnet",
        "fnm",
        "gem", "gh", "git", "glab", "go", "gradle", "gradlew", "hostname",
        "jj",
        "launchctl", "lipo", "llm", "log",
        "magick", "man", "mise", "mvn", "mvnw",
        "networksetup", "npm", "nvm",
        "ollama",
        "pip", "pip3", "pkgutil", "plutil", "pmset", "pnpm", "poetry", "pyenv",
        "rbenv",
        "security", "spctl", "swift", "sysctl",
        "tea",
        "uv",
        "volta",
        "xcode-select", "xcodebuild", "xcrun", "xmllint",
        "yarn", "yq",
    ].into_iter().collect()
});

fn is_trailing_info_request(tokens: &[Token]) -> bool {
    tokens.len() >= 2
        && tokens.last().is_some_and(|t| *t == "--help" || *t == "--version")
        && !tokens.iter().any(|t| *t == "--")
}

pub fn dispatch(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    if is_bare_info_request(tokens) {
        return true;
    }
    let cmd = tokens[0].command_name();
    if is_trailing_info_request(tokens) && HELP_ELIGIBLE.contains(cmd) {
        return true;
    }
    None
        .or_else(|| shell::dispatch(cmd, tokens, is_safe))
        .or_else(|| wrappers::dispatch(cmd, tokens, is_safe))
        .or_else(|| vcs::dispatch(cmd, tokens, is_safe))
        .or_else(|| forges::dispatch(cmd, tokens, is_safe))
        .or_else(|| node::dispatch(cmd, tokens, is_safe))
        .or_else(|| ruby::dispatch(cmd, tokens, is_safe))
        .or_else(|| python::dispatch(cmd, tokens, is_safe))
        .or_else(|| rust::dispatch(cmd, tokens, is_safe))
        .or_else(|| go::dispatch(cmd, tokens, is_safe))
        .or_else(|| jvm::dispatch(cmd, tokens, is_safe))
        .or_else(|| php::dispatch(cmd, tokens, is_safe))
        .or_else(|| swift::dispatch(cmd, tokens, is_safe))
        .or_else(|| dotnet::dispatch(cmd, tokens, is_safe))
        .or_else(|| containers::dispatch(cmd, tokens, is_safe))
        .or_else(|| network::dispatch(cmd, tokens, is_safe))
        .or_else(|| ai::dispatch(cmd, tokens, is_safe))
        .or_else(|| system::dispatch(cmd, tokens, is_safe))
        .or_else(|| xcode::dispatch(cmd, tokens, is_safe))
        .or_else(|| perl::dispatch(cmd, tokens, is_safe))
        .or_else(|| coreutils::dispatch(cmd, tokens, is_safe))
        .or_else(|| magick::dispatch(cmd, tokens))
        .unwrap_or(false)
}

#[cfg(test)]
const HANDLED_CMDS: &[&str] = &[
    "sh", "bash", "xargs", "timeout", "time", "env", "nice", "ionice", "hyperfine",
    "git", "jj", "gh", "glab", "tea",
    "npm", "yarn", "pnpm", "bun", "deno", "npx", "bunx", "nvm", "fnm", "volta",
    "bundle", "gem", "rbenv",
    "pip", "pip3", "uv", "poetry", "pyenv", "conda",
    "cargo", "rustup",
    "go",
    "gradle", "gradlew", "mvn", "mvnw",
    "composer",
    "swift",
    "dotnet",
    "curl",
    "docker", "podman",
    "ollama", "llm",
    "brew", "mise", "asdf", "defaults", "pmset", "sysctl", "cmake",
    "networksetup", "launchctl", "diskutil", "security", "csrutil", "log",
    "xcodebuild", "plutil", "xcode-select", "xcrun", "pkgutil", "lipo", "codesign", "spctl",
    "perl",
    "grep", "egrep", "fgrep", "rg",
    "cat", "head", "tail", "wc", "cut", "tr", "uniq",
    "diff", "comm", "paste", "tac", "rev", "nl",
    "expand", "unexpand", "fold", "fmt", "col", "column", "iconv", "nroff",
    "echo", "printf", "seq", "test", "expr", "bc", "factor", "bat",
    "arch", "command", "hostname",
    "find", "sed", "sort", "yq", "xmllint", "awk", "gawk", "mawk", "nawk",
    "magick",
    "fd", "eza", "exa", "ls", "delta", "colordiff",
    "dirname", "basename", "realpath", "readlink",
    "file", "stat", "du", "df", "tree",
    "true", "false",
    "printenv", "type", "whereis", "which", "whoami", "date", "pwd", "cd", "unset",
    "uname", "nproc", "uptime", "id", "groups", "tty", "locale", "cal", "sleep",
    "who", "w", "last", "lastlog",
    "ps", "top", "htop", "iotop", "procs", "dust", "lsof", "pgrep",
    "jq", "base64", "xxd", "getconf", "uuidgen",
    "md5sum", "md5", "sha256sum", "shasum", "sha1sum", "sha512sum",
    "cksum", "b2sum", "sum", "strings", "hexdump", "od", "size",
    "sw_vers", "mdls", "otool", "nm", "system_profiler", "ioreg", "vm_stat", "mdfind", "man",
    "dig", "nslookup", "host", "whois", "netstat", "ss", "ifconfig", "route",
    "identify", "shellcheck", "cloc", "tokei", "cucumber", "branchdiff", "safe-chains",
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
    docs.extend(php::command_docs());
    docs.extend(swift::command_docs());
    docs.extend(dotnet::command_docs());
    docs.extend(containers::command_docs());
    docs.extend(ai::command_docs());
    docs.extend(network::command_docs());
    docs.extend(system::command_docs());
    docs.extend(xcode::command_docs());
    docs.extend(perl::command_docs());
    docs.extend(coreutils::command_docs());
    docs.extend(shell::command_docs());
    docs.extend(wrappers::command_docs());
    docs.extend(magick::command_docs());
    docs
}

#[cfg(test)]
#[derive(Debug)]
pub(crate) enum CommandEntry {
    Policy { cmd: &'static str },
    Positional { cmd: &'static str },
    Custom { cmd: &'static str, valid_prefix: Option<&'static str> },
    Subcommand { cmd: &'static str, subs: &'static [SubEntry] },
    Delegation { cmd: &'static str },
}

#[cfg(test)]
#[derive(Debug)]
pub(crate) enum SubEntry {
    Policy { name: &'static str },
    Nested { name: &'static str, subs: &'static [SubEntry] },
    Custom { name: &'static str, valid_suffix: Option<&'static str> },
    Positional { name: &'static str },
    Delegation { name: &'static str },
    Guarded { name: &'static str, valid_suffix: &'static str },
}

#[cfg(test)]
use crate::command::CommandDef;

#[cfg(test)]
const COMMAND_DEFS: &[&CommandDef] = &[
    &ai::OLLAMA, &ai::LLM,
    &containers::DOCKER, &containers::PODMAN,
    &dotnet::DOTNET,
    &go::GO,
    &jvm::GRADLE, &jvm::GRADLEW,
    &magick::MAGICK,
    &node::NPM, &node::PNPM, &node::BUN, &node::DENO,
    &node::NVM, &node::FNM, &node::VOLTA,
    &php::COMPOSER,
    &python::PIP, &python::PIP3, &python::UV, &python::POETRY,
    &python::PYENV, &python::CONDA,
    &ruby::BUNDLE, &ruby::GEM, &ruby::RBENV,
    &rust::CARGO, &rust::RUSTUP,
    &vcs::GIT,
    &swift::SWIFT,
    &system::BREW, &system::MISE, &system::ASDF, &system::DEFAULTS,
    &system::SECURITY, &system::CSRUTIL, &system::DISKUTIL,
    &system::LAUNCHCTL, &system::LOG,
    &xcode::XCODEBUILD, &xcode::PLUTIL,
];

#[cfg(test)]
fn full_registry() -> Vec<&'static CommandEntry> {
    let mut entries = Vec::new();
    entries.extend(shell::REGISTRY);
    entries.extend(wrappers::REGISTRY);
    entries.extend(vcs::REGISTRY);
    entries.extend(forges::REGISTRY);
    entries.extend(node::REGISTRY);
    entries.extend(jvm::REGISTRY);
    entries.extend(network::REGISTRY);
    entries.extend(system::REGISTRY);
    entries.extend(xcode::REGISTRY);
    entries.extend(perl::REGISTRY);
    entries.extend(coreutils::REGISTRY);
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
            CommandEntry::Policy { cmd } => {
                let test = format!("{cmd} {UNKNOWN_FLAG}");
                if crate::is_safe_command(&test) {
                    failures.push(format!("{cmd}: accepted unknown flag: {test}"));
                }
            }
            CommandEntry::Positional { .. } | CommandEntry::Delegation { .. } => {}
            CommandEntry::Custom { cmd, valid_prefix } => {
                let base = valid_prefix.unwrap_or(cmd);
                let test = format!("{base} {UNKNOWN_FLAG}");
                if crate::is_safe_command(&test) {
                    failures.push(format!("{cmd}: accepted unknown flag: {test}"));
                }
            }
            CommandEntry::Subcommand { cmd, subs } => {
                if crate::is_safe_command(cmd) {
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
            SubEntry::Positional { .. } | SubEntry::Delegation { .. } => {}
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
    fn registry_covers_handled_commands() {
        let registry = full_registry();
        let mut all_cmds: HashSet<&str> = registry
            .iter()
            .map(|e| match e {
                CommandEntry::Policy { cmd }
                | CommandEntry::Positional { cmd }
                | CommandEntry::Custom { cmd, .. }
                | CommandEntry::Subcommand { cmd, .. }
                | CommandEntry::Delegation { cmd } => *cmd,
            })
            .collect();
        for def in COMMAND_DEFS {
            all_cmds.insert(def.name);
        }
        let handled: HashSet<&str> = HANDLED_CMDS.iter().copied().collect();

        let missing: Vec<_> = handled.difference(&all_cmds).collect();
        assert!(missing.is_empty(), "not in registry or COMMAND_DEFS: {missing:?}");

        let extra: Vec<_> = all_cmds.difference(&handled).collect();
        assert!(extra.is_empty(), "not in HANDLED_CMDS: {extra:?}");
    }

    const HELP_EXCLUDED: &[&str] = &[
        "arch",
        "sh", "bash", "xargs", "timeout", "time", "env", "nice", "ionice", "hyperfine",
        "rustup", "find",
        "npx", "bunx",
        "docker", "podman",
        "grep", "egrep", "fgrep", "rg",
        "cat", "head", "tail", "wc", "cut", "tr", "uniq",
        "diff", "comm", "paste", "tac", "rev", "nl",
        "awk", "gawk", "mawk", "nawk", "sed", "sort", "perl",
        "expand", "unexpand", "fold", "fmt", "col", "column", "iconv", "nroff",
        "echo", "printf", "seq", "test", "expr", "bc", "factor", "bat",
        "fd", "eza", "exa", "ls", "delta", "colordiff",
        "dirname", "basename", "realpath", "readlink",
        "file", "stat", "du", "df", "tree",
        "true", "false",
        "printenv", "type", "whereis", "which", "whoami", "date", "pwd", "cd", "unset",
        "uname", "nproc", "uptime", "id", "groups", "tty", "locale", "cal", "sleep",
        "who", "w", "last", "lastlog",
        "ps", "top", "htop", "iotop", "procs", "dust", "lsof", "pgrep",
        "jq", "base64", "xxd", "getconf", "uuidgen",
        "md5sum", "md5", "sha256sum", "shasum", "sha1sum", "sha512sum",
        "cksum", "b2sum", "sum", "strings", "hexdump", "od", "size",
        "sw_vers", "mdls", "otool", "nm", "system_profiler", "ioreg", "vm_stat", "mdfind",
        "dig", "nslookup", "host", "whois", "netstat", "ss", "ifconfig", "route",
        "identify", "shellcheck", "cloc", "tokei", "cucumber", "branchdiff", "safe-chains",
    ];

    #[test]
    fn help_eligible_plus_excluded_equals_handled() {
        let eligible: HashSet<&str> = HELP_ELIGIBLE.iter().copied().collect();
        let excluded: HashSet<&str> = HELP_EXCLUDED.iter().copied().collect();
        let handled: HashSet<&str> = HANDLED_CMDS.iter().copied().collect();

        let combined: HashSet<&str> = eligible.union(&excluded).copied().collect();

        let missing: Vec<&&str> = handled.difference(&combined).collect();
        assert!(
            missing.is_empty(),
            "handled commands not in HELP_ELIGIBLE or HELP_EXCLUDED: {missing:?} — \
             add each to one of the two sets"
        );

        let extra: Vec<&&str> = combined.difference(&handled).collect();
        assert!(
            extra.is_empty(),
            "commands in HELP_ELIGIBLE/HELP_EXCLUDED but not in HANDLED_CMDS: {extra:?}"
        );

        let overlap: Vec<&&str> = eligible.intersection(&excluded).collect();
        assert!(
            overlap.is_empty(),
            "commands in both HELP_ELIGIBLE and HELP_EXCLUDED: {overlap:?}"
        );
    }
}
