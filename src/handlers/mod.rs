macro_rules! handler_module {
    ($($sub:ident),+ $(,)?) => {
        $(mod $sub;)+

        pub(crate) fn dispatch(cmd: &str, tokens: &[crate::parse::Token]) -> Option<crate::verdict::Verdict> {
            None$(.or_else(|| $sub::dispatch(cmd, tokens)))+
        }

        pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
            let mut docs = Vec::new();
            $(docs.extend($sub::command_docs());)+
            docs
        }

        #[cfg(test)]
        pub(super) fn full_registry() -> Vec<&'static super::CommandEntry> {
            let mut v = Vec::new();
            $(v.extend($sub::REGISTRY);)+
            v
        }
    };
}

pub mod android;
pub mod coreutils;
pub mod forges;
pub mod fuzzy;
pub mod interpreter;
pub mod jvm;
pub mod magick;
pub mod network;
pub mod node;
pub mod perl;
pub mod php;
pub mod ruby;
pub mod shell;
pub mod system;
pub mod tilt;
pub mod vcs;
pub mod wrappers;

use std::collections::HashMap;

use crate::parse::Token;
use crate::verdict::Verdict;

type HandlerFn = fn(&[Token]) -> Verdict;

pub fn custom_cmd_handlers() -> HashMap<&'static str, HandlerFn> {
    HashMap::from([
        ("gh", forges::gh::is_safe_gh as HandlerFn),
        ("glab", forges::glab::is_safe_glab as HandlerFn),
        ("interpreter", interpreter::check_interpreter as HandlerFn),
        ("magick", magick::is_safe_magick as HandlerFn),
        ("php", php::is_safe_php as HandlerFn),
        ("ssh", system::ssh::check_ssh as HandlerFn),
        ("sysctl", system::sysctl::is_safe_sysctl as HandlerFn),
        ("tilt", tilt::check_tilt as HandlerFn),
    ])
}

pub fn custom_sub_handlers() -> HashMap<&'static str, HandlerFn> {
    HashMap::from([
        ("bun_x", node::bun::check_bun_x as HandlerFn),
        ("bundle_config", ruby::bundle::check_bundle_config as HandlerFn),
        ("bundle_exec", ruby::bundle::check_bundle_exec as HandlerFn),
        ("gh_api", forges::gh::is_safe_gh_api as HandlerFn),
        ("laravel_cache_clear", php::check_laravel_cache_clear as HandlerFn),
        ("plutil_convert", system::plutil::check_plutil_convert as HandlerFn),
        ("plutil_extract", system::plutil::check_plutil_extract as HandlerFn),
    ])
}

pub fn dispatch(tokens: &[Token]) -> Verdict {
    let cmd = tokens[0].command_name();
    let verdict = None
        .or_else(|| crate::registry::custom_dispatch(tokens))
        .or_else(|| shell::dispatch(cmd, tokens))
        .or_else(|| wrappers::dispatch(cmd, tokens))
        .or_else(|| node::dispatch(cmd, tokens))
        .or_else(|| jvm::dispatch(cmd, tokens))
        .or_else(|| android::dispatch(cmd, tokens))
        .or_else(|| network::dispatch(cmd, tokens))
        .or_else(|| system::dispatch(cmd, tokens))
        .or_else(|| perl::dispatch(cmd, tokens))
        .or_else(|| coreutils::dispatch(cmd, tokens))
        .or_else(|| fuzzy::dispatch(cmd, tokens))
        .or_else(|| vcs::dispatch(cmd, tokens))
        .or_else(|| crate::registry::toml_dispatch(tokens))
        .unwrap_or(Verdict::Denied);
    // Cross-cutting path-operand gate (audit fix): a legacy content-reader/writer must not
    // read/write a sensitive path just because its own handler ignored the operand's locus.
    // Canonicalize through the alias map (`gtee` → `tee`) so a Homebrew g-alias hits the same
    // role table its base name does — otherwise the alias sails past this gate (`gtee /etc/x`).
    if verdict.is_allowed()
        && crate::pathgate::should_deny(crate::registry::canonical_name(cmd), tokens)
    {
        return Verdict::Denied;
    }
    verdict
}

pub fn handler_docs() -> Vec<crate::docs::CommandDoc> {
    let mut docs = Vec::new();
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
    docs.extend(vcs::command_docs());
    docs.extend(crate::registry::toml_command_docs());
    docs
}

#[cfg(test)]
#[derive(Debug)]
pub(crate) enum CommandEntry {
    Custom { cmd: &'static str, valid_prefix: Option<&'static str> },
    Paths { cmd: &'static str, bare_ok: bool, paths: &'static [&'static str] },
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
    entries.extend(forges::full_registry());
    entries.extend(jvm::full_registry());
    entries.extend(android::full_registry());
    entries.extend(network::REGISTRY);
    entries.extend(coreutils::full_registry());
    entries.extend(fuzzy::full_registry());
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNKNOWN_FLAG: &str = "--xyzzy-unknown-42";
    const UNKNOWN_SUB: &str = "xyzzy-unknown-42";

    fn check_entry(entry: &CommandEntry, failures: &mut Vec<String>) {
        match entry {
            CommandEntry::Custom { cmd, valid_prefix } => {
                let base = valid_prefix.unwrap_or(cmd);
                let test = format!("{base} {UNKNOWN_FLAG}");
                if crate::is_safe_command(&test) {
                    failures.push(format!("{cmd}: accepted unknown flag: {test}"));
                }
            }
            CommandEntry::Paths { cmd, bare_ok, paths } => {
                if !bare_ok && crate::is_safe_command(cmd) {
                    failures.push(format!("{cmd}: accepted bare invocation"));
                }
                let test = format!("{cmd} {UNKNOWN_SUB}");
                if crate::is_safe_command(&test) {
                    failures.push(format!("{cmd}: accepted unknown subcommand: {test}"));
                }
                for path in *paths {
                    let test = format!("{path} {UNKNOWN_FLAG}");
                    if crate::is_safe_command(&test) {
                        failures.push(format!("{path}: accepted unknown flag: {test}"));
                    }
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
    fn process_substitution_safe_inner() {
        let safe = ["echo <(cat ./data.txt)", "grep pattern <(ls)", "diff <(sort a.txt) <(sort b.txt)", "comm -23 file.txt <(sort other.txt)"];
        for cmd in &safe {
            assert!(crate::is_safe_command(cmd), "safe process substitution rejected: {cmd}");
        }
    }

    #[test]
    fn process_substitution_unsafe_inner() {
        let unsafe_cmds = ["echo >(rm -rf /)", "diff <(sort a.txt) <(rm -rf /)"];
        for cmd in &unsafe_cmds {
            assert!(!crate::is_safe_command(cmd), "unsafe process substitution approved: {cmd}");
        }
    }

}
