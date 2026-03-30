pub mod android;
pub mod coreutils;
pub mod forges;
pub mod fuzzy;
pub mod jvm;
pub mod network;
pub mod node;
pub mod perl;
pub mod ruby;
pub mod shell;
pub mod system;
pub mod vcs;
pub mod wrappers;


use std::collections::HashMap;

use crate::parse::Token;
use crate::verdict::Verdict;

type HandlerFn = fn(&[Token]) -> Verdict;

pub fn custom_cmd_handlers() -> HashMap<&'static str, HandlerFn> {
    HashMap::from([
        ("sysctl", system::sysctl::is_safe_sysctl as HandlerFn),
    ])
}

pub fn custom_sub_handlers() -> HashMap<&'static str, HandlerFn> {
    HashMap::from([
        ("bun_x", node::bun::check_bun_x as HandlerFn),
        ("bundle_config", ruby::bundle::check_bundle_config as HandlerFn),
        ("bundle_exec", ruby::bundle::check_bundle_exec as HandlerFn),
        ("git_remote", vcs::git::check_git_remote as HandlerFn),
    ])
}

pub fn dispatch(tokens: &[Token]) -> Verdict {
    let cmd = tokens[0].command_name();
    None
        .or_else(|| shell::dispatch(cmd, tokens))
        .or_else(|| wrappers::dispatch(cmd, tokens))
        .or_else(|| forges::dispatch(cmd, tokens))
        .or_else(|| node::dispatch(cmd, tokens))
        .or_else(|| jvm::dispatch(cmd, tokens))
        .or_else(|| android::dispatch(cmd, tokens))
        .or_else(|| network::dispatch(cmd, tokens))
        .or_else(|| system::dispatch(cmd, tokens))
        .or_else(|| perl::dispatch(cmd, tokens))
        .or_else(|| coreutils::dispatch(cmd, tokens))
        .or_else(|| fuzzy::dispatch(cmd, tokens))
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
    "docker", "podman", "kubectl", "orbctl", "orb", "qemu-img",
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
    docs.extend(forges::command_docs());
    docs.extend(node::command_docs());
    docs.extend(jvm::command_docs());
    docs.extend(android::command_docs());
    docs.extend(network::command_docs());
    docs.extend(system::command_docs());
    docs.extend(perl::command_docs());
    docs.extend(coreutils::command_docs());
    docs.extend(fuzzy::command_docs());
    docs.extend(shell::command_docs());
    docs.extend(wrappers::command_docs());
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

pub fn all_opencode_patterns() -> Vec<String> {
    let mut patterns = Vec::new();
    patterns.sort();
    patterns.dedup();
    patterns
}

#[cfg(test)]
fn full_registry() -> Vec<&'static CommandEntry> {
    let mut entries = Vec::new();
    entries.extend(shell::REGISTRY);
    entries.extend(wrappers::REGISTRY);
    entries.extend(forges::full_registry());
    entries.extend(node::full_registry());
    entries.extend(jvm::full_registry());
    entries.extend(android::full_registry());
    entries.extend(network::REGISTRY);
    entries.extend(system::full_registry());
    entries.extend(perl::REGISTRY);
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
        for name in crate::registry::toml_command_names() {
            all_cmds.insert(name);
        }
        let handled: HashSet<&str> = HANDLED_CMDS.iter().copied().collect();

        let missing: Vec<_> = handled.difference(&all_cmds).collect();
        assert!(missing.is_empty(), "not in registry: {missing:?}");

        let extra: Vec<_> = all_cmds.difference(&handled).collect();
        assert!(extra.is_empty(), "not in HANDLED_CMDS: {extra:?}");
    }

}
