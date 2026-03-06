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
mod tests {
    use super::*;
    use std::collections::HashSet;

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
