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
    ("fd", "Find files", AnyArgs),
    ("eza", "Modern ls replacement", AnyArgs),
    ("exa", "Modern ls replacement", AnyArgs),

    ("ls", "List directory", AnyArgs),

    ("delta", "Syntax-highlighted diff viewer", AnyArgs),
    ("colordiff", "Colorized diff", AnyArgs),

    ("true", "Return success exit code", Bare),
    ("false", "Return failure exit code", Bare),

    ("dirname", "Strip filename from path", AnyArgs),
    ("basename", "Strip directory from path", AnyArgs),
    ("realpath", "Resolve path", AnyArgs),
    ("readlink", "Resolve symlink", AnyArgs),
    ("file", "Detect file type", AnyArgs),
    ("stat", "File status", AnyArgs),
    ("du", "Disk usage", AnyArgs),
    ("df", "Disk free space", AnyArgs),

    ("printenv", "Print environment variables", AnyArgs),
    ("type", "Identify command type", AnyArgs),
    ("whereis", "Locate binary, source, and man page", AnyArgs),
    ("which", "Locate command", AnyArgs),
    ("whoami", "Print current user", Bare),
    ("date", "Display date and time", AnyArgs),
    ("pwd", "Print working directory", AnyArgs),
    ("tree", "Directory tree", AnyArgs),
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

#[allow(clippy::too_many_lines)]
pub fn dispatch(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    if is_bare_info_request(tokens) {
        return true;
    }
    let cmd = tokens[0].command_name();
    if is_trailing_info_request(tokens) && HELP_ELIGIBLE.contains(cmd) {
        return true;
    }
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

        "curl" => network::is_safe_curl(tokens),

        "ollama" => ai::is_safe_ollama(tokens),
        "llm" => ai::is_safe_llm(tokens),

        "brew" => system::is_safe_brew(tokens),
        "mise" => system::is_safe_mise(tokens),
        "asdf" => system::is_safe_asdf(tokens),
        "defaults" => system::is_safe_defaults(tokens),
        "pmset" => system::is_safe_pmset(tokens),
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
        "spctl" => xcode::is_safe_spctl(tokens),

        "perl" => perl::is_safe_perl(tokens),

        "grep" | "egrep" | "fgrep" => coreutils::is_safe_grep(tokens),
        "rg" => coreutils::is_safe_rg(tokens),
        "cat" => coreutils::is_safe_cat(tokens),
        "head" => coreutils::is_safe_head(tokens),
        "tail" => coreutils::is_safe_tail(tokens),
        "wc" => coreutils::is_safe_wc(tokens),
        "cut" => coreutils::is_safe_cut(tokens),
        "tr" => coreutils::is_safe_tr(tokens),
        "uniq" => coreutils::is_safe_uniq(tokens),
        "diff" => coreutils::is_safe_diff(tokens),
        "comm" => coreutils::is_safe_comm(tokens),
        "paste" => coreutils::is_safe_paste(tokens),
        "tac" => coreutils::is_safe_tac(tokens),
        "rev" => coreutils::is_safe_rev(tokens),
        "nl" => coreutils::is_safe_nl(tokens),
        "expand" => coreutils::is_safe_expand(tokens),
        "unexpand" => coreutils::is_safe_unexpand(tokens),
        "fold" => coreutils::is_safe_fold(tokens),
        "fmt" => coreutils::is_safe_fmt(tokens),
        "column" => coreutils::is_safe_column(tokens),
        "iconv" => coreutils::is_safe_iconv(tokens),
        "nroff" => coreutils::is_safe_nroff(tokens),
        "echo" => coreutils::is_safe_echo(tokens),
        "printf" => coreutils::is_safe_printf(tokens),
        "seq" => coreutils::is_safe_seq(tokens),
        "test" => coreutils::is_safe_test(tokens),
        "expr" => coreutils::is_safe_expr(tokens),
        "bc" => coreutils::is_safe_bc(tokens),
        "factor" => coreutils::is_safe_factor(tokens),
        "bat" => coreutils::is_safe_bat(tokens),

        "arch" => coreutils::is_safe_arch(tokens),
        "command" => coreutils::is_safe_command_builtin(tokens),
        "hostname" => coreutils::is_safe_hostname(tokens),

        "find" => coreutils::is_safe_find(tokens, is_safe),
        "sed" => coreutils::is_safe_sed(tokens),
        "sort" => coreutils::is_safe_sort(tokens),
        "yq" => coreutils::is_safe_yq(tokens),
        "xmllint" => coreutils::is_safe_xmllint(tokens),
        "awk" | "gawk" | "mawk" | "nawk" => coreutils::is_safe_awk(tokens),

        "magick" => is_safe_subcmd(tokens, &MAGICK_SAFE, &[]),

        _ => SAFE_CMDS.contains(cmd),
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
