pub mod containers;
pub mod coreutils;
pub mod dotnet;
pub mod gh;
pub mod go;
pub mod jvm;
pub mod node;
pub mod php;
pub mod python;
pub mod ruby;
pub mod rust;
pub mod shell;
pub mod swift;
pub mod system;
pub mod vcs;
pub mod wrappers;

use std::collections::HashSet;
use std::sync::LazyLock;

static SAFE_CMDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "grep", "rg", "fd", "head", "tail", "cat", "ls", "wc", "uniq", "tr", "cut", "echo",
        "dirname", "basename", "realpath", "file", "stat", "du", "df", "printenv", "which",
        "whoami", "date", "pwd", "tree", "lsof", "jq", "base64", "xxd", "pgrep", "getconf",
        "ps", "uuidgen", "mdfind", "identify", "cd", "command", "cucumber", "branchdiff",
        "diff", "comm", "paste", "tac", "rev", "nl", "expand", "unexpand", "fold", "fmt",
        "column", "printf", "seq", "expr", "test", "true", "false", "bc", "factor",
        "colordiff", "iconv",
        "readlink", "hostname", "uname", "arch", "nproc", "uptime", "id", "groups", "tty",
        "locale", "cal", "sleep",
        "md5sum", "md5", "sha256sum", "shasum", "sha1sum", "sha512sum", "cksum", "b2sum",
        "sum", "strings", "hexdump", "od", "size",
        "sw_vers", "mdls", "otool", "nm",
        "dig", "nslookup", "host", "whois",
        "shellcheck", "cloc", "tokei",
    ])
});

pub fn dispatch(cmd: &str, tokens: &[String], is_safe: &dyn Fn(&str) -> bool) -> bool {
    match cmd {
        "sh" | "bash" => shell::is_safe_shell(tokens, is_safe),
        "xargs" => shell::is_safe_xargs(tokens, is_safe),
        "timeout" => wrappers::is_safe_timeout(tokens, is_safe),
        "time" => wrappers::is_safe_time(tokens, is_safe),
        "env" => wrappers::is_safe_env(tokens, is_safe),
        "nice" | "ionice" => wrappers::is_safe_nice(tokens, is_safe),

        "git" => vcs::is_safe_git(tokens),
        "jj" => vcs::is_safe_jj(tokens),
        "gh" => gh::is_safe_gh(tokens),

        "npm" => node::is_safe_npm(tokens),
        "yarn" => node::is_safe_yarn(tokens),
        "pnpm" => node::is_safe_pnpm(tokens),
        "bun" => node::is_safe_bun(tokens),
        "deno" => node::is_safe_deno(tokens),
        "npx" => node::is_safe_npx(tokens),
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
        "python" | "python3" => wrappers::is_safe_python(tokens),

        "cargo" => rust::is_safe_cargo(tokens),
        "rustup" => rust::is_safe_rustup(tokens),

        "go" => go::is_safe_go(tokens),

        "gradle" | "gradlew" => jvm::is_safe_gradle(tokens),
        "mvn" | "mvnw" => jvm::is_safe_mvn(tokens),

        "composer" => php::is_safe_composer(tokens),

        "swift" => swift::is_safe_swift(tokens),

        "dotnet" => dotnet::is_safe_dotnet(tokens),

        "docker" | "podman" => containers::is_safe_docker(tokens),

        "brew" => system::is_safe_brew(tokens),
        "mise" => system::is_safe_mise(tokens),
        "asdf" => system::is_safe_asdf(tokens),
        "defaults" => system::is_safe_defaults(tokens),
        "sysctl" => system::is_safe_sysctl(tokens),
        "xcodebuild" => system::is_safe_xcodebuild(tokens),
        "cmake" => system::is_safe_cmake(tokens),

        "find" => coreutils::is_safe_find(tokens),
        "sed" => coreutils::is_safe_sed(tokens),
        "sort" => coreutils::is_safe_sort(tokens),
        "yq" => coreutils::is_safe_yq(tokens),
        "xmllint" => coreutils::is_safe_xmllint(tokens),

        _ => SAFE_CMDS.contains(cmd),
    }
}
