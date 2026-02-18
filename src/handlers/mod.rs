pub mod coreutils;
pub mod gh;
pub mod packages;
pub mod shell;
pub mod vcs;
pub mod wrappers;

use std::collections::HashSet;
use std::sync::LazyLock;

static SAFE_CMDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "grep", "rg", "fd", "head", "tail", "cat", "ls", "wc", "uniq", "tr", "cut", "echo",
        "dirname", "basename", "realpath", "file", "stat", "du", "df", "printenv", "which",
        "whoami", "date", "pwd", "tree", "lsof", "jq", "base64", "xxd", "pgrep", "getconf",
        "ps", "uuidgen", "mdfind", "identify", "cd",
    ])
});

pub fn dispatch(cmd: &str, tokens: &[String], is_safe: &dyn Fn(&str) -> bool) -> bool {
    match cmd {
        "sh" | "bash" => shell::is_safe_shell(tokens, is_safe),
        "xargs" => shell::is_safe_xargs(tokens, is_safe),
        "gh" => gh::is_safe_gh(tokens),
        "git" => vcs::is_safe_git(tokens),
        "jj" => vcs::is_safe_jj(tokens),
        "yarn" => packages::is_safe_yarn(tokens),
        "npm" => packages::is_safe_npm(tokens),
        "pip" | "pip3" => packages::is_safe_pip(tokens),
        "bundle" => packages::is_safe_bundle(tokens),
        "gem" => packages::is_safe_gem(tokens),
        "brew" => packages::is_safe_brew(tokens),
        "cargo" => packages::is_safe_cargo(tokens),
        "npx" => packages::is_safe_npx(tokens),
        "mise" => packages::is_safe_mise(tokens),
        "asdf" => packages::is_safe_asdf(tokens),
        "env" => wrappers::is_safe_env(tokens),
        "python" | "python3" => wrappers::is_safe_python(tokens),
        "timeout" => wrappers::is_safe_timeout(tokens, is_safe),
        "time" => wrappers::is_safe_time(tokens, is_safe),
        "find" => coreutils::is_safe_find(tokens),
        "sed" => coreutils::is_safe_sed(tokens),
        "sort" => coreutils::is_safe_sort(tokens),
        _ => SAFE_CMDS.contains(cmd),
    }
}
