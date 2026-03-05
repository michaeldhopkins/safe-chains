pub mod ai;
pub mod containers;
pub mod coreutils;
pub mod dotnet;
pub mod forges;
pub mod go;
pub mod jvm;
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

use crate::parse::{Segment, Token, WordSet};

static MAGICK_SAFE: WordSet = WordSet::new(&["--help", "--version", "identify"]);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SafeKind {
    Bare,
    AnyArgs,
}

use SafeKind::{AnyArgs, Bare};

pub(crate) static SAFE_CMD_ENTRIES: &[(&str, &str, SafeKind)] = &[
    ("true", "Return success exit code", Bare),
    ("false", "Return failure exit code", Bare),

    ("printenv", "Print environment variables", AnyArgs),
    ("type", "Identify command type", AnyArgs),
    ("whereis", "Locate binary, source, and man page", AnyArgs),
    ("which", "Locate command", AnyArgs),
    ("whoami", "Print current user", Bare),
    ("date", "Display date and time", AnyArgs),
    ("pwd", "Print working directory", AnyArgs),
    ("cd", "Change directory", AnyArgs),
    ("unset", "Unset environment variables", AnyArgs),

    ("uname", "System information", AnyArgs),
    ("nproc", "Print number of CPUs", AnyArgs),
    ("uptime", "System uptime", AnyArgs),
    ("id", "Print user/group IDs", AnyArgs),
    ("groups", "Print group memberships", AnyArgs),
    ("tty", "Print terminal name", AnyArgs),
    ("locale", "Print locale info", AnyArgs),
    ("cal", "Display calendar", AnyArgs),
    ("sleep", "Pause execution", AnyArgs),
    ("who", "Show logged-in users", AnyArgs),
    ("w", "Show logged-in users and activity", AnyArgs),
    ("last", "Show login history", AnyArgs),
    ("lastlog", "Show last login for all users", AnyArgs),

    ("ps", "List processes", AnyArgs),
    ("top", "Process monitor", AnyArgs),
    ("htop", "Interactive process viewer", AnyArgs),
    ("iotop", "I/O usage monitor", AnyArgs),
    ("procs", "Modern process viewer", AnyArgs),
    ("dust", "Disk usage viewer", AnyArgs),
    ("lsof", "List open files", AnyArgs),
    ("pgrep", "Search for processes", AnyArgs),

    ("jq", "JSON processor", AnyArgs),
    ("base64", "Base64 encode/decode", AnyArgs),
    ("xxd", "Hex dump", AnyArgs),
    ("getconf", "Get system configuration values", AnyArgs),
    ("uuidgen", "Generate UUID", AnyArgs),

    ("md5sum", "MD5 checksum", AnyArgs),
    ("md5", "MD5 checksum (macOS)", AnyArgs),
    ("sha256sum", "SHA-256 checksum", AnyArgs),
    ("shasum", "SHA checksum", AnyArgs),
    ("sha1sum", "SHA-1 checksum", AnyArgs),
    ("sha512sum", "SHA-512 checksum", AnyArgs),
    ("cksum", "File checksum", AnyArgs),
    ("b2sum", "BLAKE2 checksum", AnyArgs),
    ("sum", "File checksum", AnyArgs),
    ("strings", "Find printable strings in binary", AnyArgs),
    ("hexdump", "Display file in hex", AnyArgs),
    ("od", "Octal dump", AnyArgs),
    ("size", "Object file section sizes", AnyArgs),

    ("sw_vers", "macOS version info", AnyArgs),
    ("mdls", "File metadata (macOS)", AnyArgs),
    ("otool", "Object file tool (macOS)", AnyArgs),
    ("nm", "List object file symbols", AnyArgs),
    ("system_profiler", "macOS hardware/software info", AnyArgs),
    ("ioreg", "macOS I/O Registry viewer", AnyArgs),
    ("vm_stat", "Virtual memory statistics", AnyArgs),
    ("mdfind", "Spotlight search (macOS)", AnyArgs),

    ("dig", "DNS lookup", AnyArgs),
    ("nslookup", "DNS lookup", AnyArgs),
    ("host", "DNS lookup", AnyArgs),
    ("whois", "Domain registration lookup", AnyArgs),
    ("netstat", "Network connections and statistics", AnyArgs),
    ("ss", "Socket statistics", AnyArgs),
    ("ifconfig", "Network interface info", AnyArgs),
    ("route", "Routing table", AnyArgs),

    ("identify", "ImageMagick identify", AnyArgs),
    ("shellcheck", "Shell script linter", AnyArgs),
    ("cloc", "Count lines of code", AnyArgs),
    ("tokei", "Code statistics", AnyArgs),
    ("cucumber", "BDD test runner", AnyArgs),
    ("branchdiff", "Branch diff tool", AnyArgs),
    ("safe-chains", "Safe command checker", AnyArgs),
];

pub(crate) static SAFE_CMDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    SAFE_CMD_ENTRIES.iter().map(|&(name, _, _)| name).collect()
});

pub(crate) fn is_safe_subcmd(
    tokens: &[Token],
    simple: &WordSet,
    multi: &[(&str, WordSet)],
) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if simple.contains(&tokens[1]) {
        return true;
    }
    for (prefix, actions) in multi {
        if tokens[1] == *prefix {
            return tokens.get(2).is_some_and(|a| actions.contains(a));
        }
    }
    false
}

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
        "magick", "mise", "mvn", "mvnw",
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
        .or_else(|| dispatch_magick(cmd, tokens))
        .unwrap_or_else(|| SAFE_CMDS.contains(cmd))
}

fn dispatch_magick(cmd: &str, tokens: &[Token]) -> Option<bool> {
    match cmd {
        "magick" => Some(is_safe_subcmd(tokens, &MAGICK_SAFE, &[])),
        _ => None,
    }
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
    "expand", "unexpand", "fold", "fmt", "column", "iconv", "nroff",
    "echo", "printf", "seq", "test", "expr", "bc", "factor", "bat",
    "arch", "command", "hostname",
    "find", "sed", "sort", "yq", "xmllint", "awk", "gawk", "mawk", "nawk",
    "magick",
    "fd", "eza", "exa", "ls", "delta", "colordiff",
    "dirname", "basename", "realpath", "readlink",
    "file", "stat", "du", "df", "tree",
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
    docs.push(crate::docs::CommandDoc::handler("magick",
        crate::docs::doc(&MAGICK_SAFE).build()));
    docs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn safe_cmd_entries_no_duplicates() {
        let mut seen = HashSet::new();
        for &(name, _, _) in SAFE_CMD_ENTRIES {
            assert!(seen.insert(name), "duplicate SAFE_CMD_ENTRIES name: {name}");
        }
    }

    #[test]
    fn safe_cmd_entries_no_empty_descriptions() {
        for &(name, desc, _) in SAFE_CMD_ENTRIES {
            assert!(!desc.is_empty(), "empty description for SAFE_CMD_ENTRIES: {name}");
        }
    }

    #[test]
    fn safe_cmd_entries_no_overlap_with_handlers() {
        let handled: HashSet<&str> = HANDLED_CMDS.iter().copied().collect();
        for &(name, _, _) in SAFE_CMD_ENTRIES {
            assert!(
                !handled.contains(name),
                "{name} is in both SAFE_CMD_ENTRIES and dispatch — the dispatch handler shadows it"
            );
        }
    }

    #[test]
    fn handled_cmds_matches_dispatch() {
        let handled: HashSet<&str> = HANDLED_CMDS.iter().copied().collect();
        let safe: HashSet<&str> = SAFE_CMD_ENTRIES.iter().map(|&(n, _, _)| n).collect();
        for name in &handled {
            assert!(
                !safe.contains(name),
                "{name} is in both HANDLED_CMDS and SAFE_CMD_ENTRIES"
            );
        }
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
        "expand", "unexpand", "fold", "fmt", "column", "iconv", "nroff",
        "echo", "printf", "seq", "test", "expr", "bc", "factor", "bat",
        "fd", "eza", "exa", "ls", "delta", "colordiff",
        "dirname", "basename", "realpath", "readlink",
        "file", "stat", "du", "df", "tree",
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

#[cfg(test)]
mod magick_tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        magick_identify: "magick identify /tmp/image.png",
        magick_identify_verbose: "magick identify -verbose /tmp/image.png",
        magick_identify_multi: "magick identify /tmp/a.png /tmp/b.png",
        magick_help: "magick --help",
        magick_version: "magick --version",
    }

    denied! {
        magick_convert_denied: "magick input.png output.jpg",
        magick_mogrify_denied: "magick mogrify -resize 50% image.png",
        magick_composite_denied: "magick composite overlay.png base.png result.png",
        magick_conjure_denied: "magick conjure script.msl",
        bare_magick_denied: "magick",
    }
}
