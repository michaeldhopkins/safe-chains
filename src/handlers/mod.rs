pub mod ai;
pub mod containers;
pub mod coreutils;
pub mod dotnet;
pub mod forges;
pub mod go;
pub mod jvm;
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

pub(crate) static SAFE_CMD_ENTRIES: &[(&str, &str)] = &[
    ("grep", "Search file contents"),
    ("rg", "Ripgrep search"),
    ("fd", "Find files"),
    ("bat", "Syntax-highlighted cat"),
    ("eza", "Modern ls replacement"),
    ("exa", "Modern ls replacement"),

    ("head", "Print first lines"),
    ("tail", "Print last lines"),
    ("cat", "Print file contents"),
    ("ls", "List directory"),
    ("wc", "Count lines/words/bytes"),
    ("uniq", "Filter duplicate lines"),
    ("tr", "Translate characters"),
    ("cut", "Extract fields from lines"),

    ("diff", "Compare files"),
    ("delta", "Syntax-highlighted diff viewer"),
    ("colordiff", "Colorized diff"),
    ("comm", "Compare sorted files"),
    ("paste", "Merge lines of files"),

    ("tac", "Print file in reverse"),
    ("rev", "Reverse lines"),
    ("nl", "Number lines"),
    ("expand", "Convert tabs to spaces"),
    ("unexpand", "Convert spaces to tabs"),
    ("fold", "Wrap lines"),
    ("fmt", "Reformat text"),
    ("column", "Format into columns"),
    ("iconv", "Convert character encoding"),

    ("echo", "Print text"),
    ("printf", "Format and print text"),
    ("seq", "Print number sequence"),
    ("expr", "Evaluate expression"),
    ("test", "Evaluate conditional expression"),
    ("true", "Return success exit code"),
    ("false", "Return failure exit code"),
    ("bc", "Calculator"),
    ("factor", "Print prime factors"),

    ("dirname", "Strip filename from path"),
    ("basename", "Strip directory from path"),
    ("realpath", "Resolve path"),
    ("readlink", "Resolve symlink"),
    ("file", "Detect file type"),
    ("stat", "File status"),
    ("du", "Disk usage"),
    ("df", "Disk free space"),

    ("printenv", "Print environment variables"),
    ("which", "Locate command"),
    ("whoami", "Print current user"),
    ("date", "Display date and time"),
    ("pwd", "Print working directory"),
    ("tree", "Directory tree"),
    ("cd", "Change directory"),
    ("command", "Run command or check existence"),
    ("unset", "Unset environment variables"),

    ("hostname", "Print hostname"),
    ("uname", "System information"),
    ("arch", "Print machine architecture"),
    ("nproc", "Print number of CPUs"),
    ("uptime", "System uptime"),
    ("id", "Print user/group IDs"),
    ("groups", "Print group memberships"),
    ("tty", "Print terminal name"),
    ("locale", "Print locale info"),
    ("cal", "Display calendar"),
    ("sleep", "Pause execution"),
    ("who", "Show logged-in users"),
    ("w", "Show logged-in users and activity"),
    ("last", "Show login history"),
    ("lastlog", "Show last login for all users"),

    ("ps", "List processes"),
    ("top", "Process monitor"),
    ("htop", "Interactive process viewer"),
    ("iotop", "I/O usage monitor"),
    ("procs", "Modern process viewer"),
    ("dust", "Disk usage viewer"),
    ("lsof", "List open files"),
    ("pgrep", "Search for processes"),

    ("jq", "JSON processor"),
    ("base64", "Base64 encode/decode"),
    ("xxd", "Hex dump"),
    ("getconf", "Get system configuration values"),
    ("uuidgen", "Generate UUID"),

    ("md5sum", "MD5 checksum"),
    ("md5", "MD5 checksum (macOS)"),
    ("sha256sum", "SHA-256 checksum"),
    ("shasum", "SHA checksum"),
    ("sha1sum", "SHA-1 checksum"),
    ("sha512sum", "SHA-512 checksum"),
    ("cksum", "File checksum"),
    ("b2sum", "BLAKE2 checksum"),
    ("sum", "File checksum"),
    ("strings", "Find printable strings in binary"),
    ("hexdump", "Display file in hex"),
    ("od", "Octal dump"),
    ("size", "Object file section sizes"),

    ("sw_vers", "macOS version info"),
    ("mdls", "File metadata (macOS)"),
    ("otool", "Object file tool (macOS)"),
    ("nm", "List object file symbols"),
    ("system_profiler", "macOS hardware/software info"),
    ("ioreg", "macOS I/O Registry viewer"),
    ("vm_stat", "Virtual memory statistics"),
    ("mdfind", "Spotlight search (macOS)"),

    ("dig", "DNS lookup"),
    ("nslookup", "DNS lookup"),
    ("host", "DNS lookup"),
    ("whois", "Domain registration lookup"),
    ("netstat", "Network connections and statistics"),
    ("ss", "Socket statistics"),
    ("ifconfig", "Network interface info"),
    ("route", "Routing table"),

    ("identify", "ImageMagick identify"),
    ("shellcheck", "Shell script linter"),
    ("cloc", "Count lines of code"),
    ("tokei", "Code statistics"),
    ("cucumber", "BDD test runner"),
    ("branchdiff", "Branch diff tool"),
    ("safe-chains", "Safe command checker"),
];

pub(crate) static SAFE_CMDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    SAFE_CMD_ENTRIES.iter().map(|&(name, _)| name).collect()
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

pub fn dispatch(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    let cmd = tokens[0].command_name();
    match cmd {
        "sh" | "bash" => shell::is_safe_shell(tokens, is_safe),
        "xargs" => shell::is_safe_xargs(tokens, is_safe),
        "timeout" => wrappers::is_safe_timeout(tokens, is_safe),
        "time" => wrappers::is_safe_time(tokens, is_safe),
        "env" => wrappers::is_safe_env(tokens, is_safe),
        "nice" | "ionice" => wrappers::is_safe_nice(tokens, is_safe),
        "hyperfine" => wrappers::is_safe_hyperfine(tokens, is_safe),

        "git" => vcs::is_safe_git(tokens),
        "jj" => vcs::is_safe_jj(tokens),
        "gh" => forges::is_safe_gh(tokens),
        "glab" => forges::is_safe_glab(tokens),
        "tea" => forges::is_safe_tea(tokens),

        "npm" => node::is_safe_npm(tokens),
        "yarn" => node::is_safe_yarn(tokens),
        "pnpm" => node::is_safe_pnpm(tokens),
        "bun" => node::is_safe_bun(tokens),
        "deno" => node::is_safe_deno(tokens),
        "npx" => node::is_safe_npx(tokens),
        "bunx" => node::is_safe_bunx(tokens),
        "nvm" => node::is_safe_nvm(tokens),
        "fnm" => node::is_safe_fnm(tokens),
        "volta" => node::is_safe_volta(tokens),

        "bundle" => ruby::is_safe_bundle(tokens),
        "gem" => ruby::is_safe_gem(tokens),
        "rbenv" => ruby::is_safe_rbenv(tokens),

        "pip" | "pip3" => python::is_safe_pip(tokens),
        "uv" => python::is_safe_uv(tokens),
        "poetry" => python::is_safe_poetry(tokens),
        "pyenv" => python::is_safe_pyenv(tokens),
        "conda" => python::is_safe_conda(tokens),

        "cargo" => rust::is_safe_cargo(tokens),
        "rustup" => rust::is_safe_rustup(tokens, is_safe),

        "go" => go::is_safe_go(tokens),

        "gradle" | "gradlew" => jvm::is_safe_gradle(tokens),
        "mvn" | "mvnw" => jvm::is_safe_mvn(tokens),

        "composer" => php::is_safe_composer(tokens),

        "swift" => swift::is_safe_swift(tokens),

        "dotnet" => dotnet::is_safe_dotnet(tokens),

        "docker" | "podman" => containers::is_safe_docker(tokens),

        "ollama" => ai::is_safe_ollama(tokens),
        "llm" => ai::is_safe_llm(tokens),

        "brew" => system::is_safe_brew(tokens),
        "mise" => system::is_safe_mise(tokens),
        "asdf" => system::is_safe_asdf(tokens),
        "defaults" => system::is_safe_defaults(tokens),
        "sysctl" => system::is_safe_sysctl(tokens),
        "cmake" => system::is_safe_cmake(tokens),
        "networksetup" => system::is_safe_networksetup(tokens),
        "launchctl" => system::is_safe_launchctl(tokens),
        "diskutil" => system::is_safe_diskutil(tokens),
        "security" => system::is_safe_security(tokens),
        "csrutil" => system::is_safe_csrutil(tokens),
        "log" => system::is_safe_log(tokens),

        "xcodebuild" => xcode::is_safe_xcodebuild(tokens),
        "plutil" => xcode::is_safe_plutil(tokens),
        "xcode-select" => xcode::is_safe_xcode_select(tokens),
        "xcrun" => xcode::is_safe_xcrun(tokens),
        "pkgutil" => xcode::is_safe_pkgutil(tokens),
        "lipo" => xcode::is_safe_lipo(tokens),
        "codesign" => xcode::is_safe_codesign(tokens),

        "perl" => perl::is_safe_perl(tokens),

        "find" => coreutils::is_safe_find(tokens, is_safe),
        "sed" => coreutils::is_safe_sed(tokens),
        "sort" => coreutils::is_safe_sort(tokens),
        "yq" => coreutils::is_safe_yq(tokens),
        "xmllint" => coreutils::is_safe_xmllint(tokens),
        "awk" | "gawk" | "mawk" | "nawk" => coreutils::is_safe_awk(tokens),

        _ => SAFE_CMDS.contains(cmd) || is_bare_info_request(tokens),
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
    "docker", "podman",
    "ollama", "llm",
    "brew", "mise", "asdf", "defaults", "sysctl", "cmake",
    "networksetup", "launchctl", "diskutil", "security", "csrutil", "log",
    "xcodebuild", "plutil", "xcode-select", "xcrun", "pkgutil", "lipo", "codesign",
    "perl",
    "find", "sed", "sort", "yq", "xmllint", "awk", "gawk", "mawk", "nawk",
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
    docs.extend(system::command_docs());
    docs.extend(xcode::command_docs());
    docs.extend(perl::command_docs());
    docs.extend(coreutils::command_docs());
    docs.extend(shell::command_docs());
    docs.extend(wrappers::command_docs());
    docs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn safe_cmd_entries_no_duplicates() {
        let mut seen = HashSet::new();
        for &(name, _) in SAFE_CMD_ENTRIES {
            assert!(seen.insert(name), "duplicate SAFE_CMD_ENTRIES name: {name}");
        }
    }

    #[test]
    fn safe_cmd_entries_no_empty_descriptions() {
        for &(name, desc) in SAFE_CMD_ENTRIES {
            assert!(!desc.is_empty(), "empty description for SAFE_CMD_ENTRIES: {name}");
        }
    }

    #[test]
    fn safe_cmd_entries_no_overlap_with_handlers() {
        let handled: HashSet<&str> = HANDLED_CMDS.iter().copied().collect();
        for &(name, _) in SAFE_CMD_ENTRIES {
            assert!(
                !handled.contains(name),
                "{name} is in both SAFE_CMD_ENTRIES and dispatch — the dispatch handler shadows it"
            );
        }
    }

    #[test]
    fn handled_cmds_matches_dispatch() {
        let handled: HashSet<&str> = HANDLED_CMDS.iter().copied().collect();
        let safe: HashSet<&str> = SAFE_CMD_ENTRIES.iter().map(|&(n, _)| n).collect();
        for name in &handled {
            assert!(
                !safe.contains(name),
                "{name} is in both HANDLED_CMDS and SAFE_CMD_ENTRIES"
            );
        }
    }
}
