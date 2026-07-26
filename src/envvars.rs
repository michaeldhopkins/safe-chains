//! Classifying the `VAR=value` assignments that precede a command.
//!
//! `VAR=value cmd` is not `cmd`. An assignment can make the process run different code, load code
//! from a different place, or reach a different system, without changing the command name the rest
//! of the classifier looks at. `envvars.toml` is the researched list of variables where that is
//! true; this module classifies their values.
//!
//! # Only the listed names are inspected
//!
//! An unlisted assignment is ignored exactly as before, and an absent assignment changes nothing.
//! This adds no denials outside the table. That is the property that makes the list safe to grow one
//! entry at a time, and it is why the earlier "an undeclared name denies" design was abandoned — the
//! set of harmless variables is unbounded (any program may read any name), so an allowlist over it
//! could never be completed.
//!
//! # The value is judged by rules that already exist
//!
//! Nothing here decides whether a name is dangerous in the abstract. `GIT_PAGER=cat` passes because
//! `cat` passes; `GIT_PAGER='sh -c evil'` denies because that command denies. `GIT_DIR=/tmp/evil`
//! denies because `/tmp` is out-of-worktree under the ordinary locus rules. The table only says
//! WHICH rule to apply.
//!
//! See `docs/design/env-prefix-classification.md` for the measurements behind each entry.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

use crate::verdict::{SafetyLevel, Verdict};

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Shape {
    /// The value is a command line — recursed through `command_verdict`, the same way `sudo X`
    /// classifies its inner command.
    Command,
    /// The value names a path SUPPLYING CODE — gated at the executor locus, as cargo's
    /// `--manifest-path` is. Deliberately not the read rule: reading `/tmp/x.so` is fine, while
    /// loading it into the process is not.
    ExecPath,
    /// The value names a path read or written as data — ordinary locus rules.
    DataPath,
    /// The variable's effect CANNOT be certified from its own value, because the meaning depends on
    /// another assignment. `GIT_CONFIG_VALUE_0` is a command when `GIT_CONFIG_KEY_0` is
    /// `core.pager` and inert when it is `user.name`; nothing in this per-assignment model can see
    /// the pair. Presence denies (§0 fail-closed), which is also what the FLAG spelling does —
    /// `git -c user.name=x log` already denies because that key is not on git's permitted list.
    Opaque,
}

#[derive(Deserialize, Debug)]
struct Entry {
    shape: Shape,
    #[allow(dead_code)] // authored rationale, carried for the docs and for review
    because: String,
    #[allow(dead_code)] // false = enumerated from docs, not probed on a live toolchain
    #[serde(default)]
    measured: Option<bool>,
}

#[derive(Deserialize, Debug)]
struct Table {
    env: HashMap<String, Entry>,
}

struct Compiled {
    exact: HashMap<String, Shape>,
    /// `(prefix, suffix, shape)` from a name containing a single `*`. Needed because some variables
    /// carry a variable segment: cargo spells its per-target keys `CARGO_TARGET_<TRIPLE>_RUNNER`,
    /// and git numbers its config pairs `GIT_CONFIG_KEY_0`, `_1`, …
    globs: Vec<(String, String, Shape)>,
}

static TABLE: LazyLock<Compiled> = LazyLock::new(|| {
    let src = include_str!("../envvars.toml");
    let parsed: Table = toml::from_str(src).expect("embedded envvars.toml must parse");
    let mut exact = HashMap::new();
    let mut globs = Vec::new();
    for (name, entry) in parsed.env {
        match name.split_once('*') {
            Some((pre, suf)) => {
                assert!(!suf.contains('*'), "envvars.toml: `{name}` has more than one `*`");
                globs.push((pre.to_string(), suf.to_string(), entry.shape));
            }
            None => {
                exact.insert(name, entry.shape);
            }
        }
    }
    Compiled { exact, globs }
});

/// The shape of a listed variable's value, or `None` for a name we do not inspect.
pub(crate) fn shape_of(name: &str) -> Option<Shape> {
    if let Some(s) = TABLE.exact.get(name) {
        return Some(*s);
    }
    TABLE.globs.iter().find_map(|(pre, suf, shape)| {
        // `len()` guard so a single `*` cannot match the empty middle twice over — `A_*_B` must not
        // match `A__B` by letting prefix and suffix overlap.
        (name.len() >= pre.len() + suf.len() && name.starts_with(pre.as_str()) && name.ends_with(suf.as_str()))
            .then_some(*shape)
    })
}

/// The verdict for one `NAME=value` assignment. `Inert` for any name not in the table — the
/// overwhelmingly common case, and the reason this costs nothing for ordinary commands.
pub(crate) fn assignment_verdict(name: &str, value: &str) -> Verdict {
    let Some(shape) = shape_of(name) else {
        return Verdict::Allowed(SafetyLevel::Inert);
    };
    // An EMPTY value un-sets the behaviour rather than pointing it anywhere: `GIT_PAGER=` disables
    // the pager, `LD_PRELOAD=` preloads nothing. Denying that would be a false positive on the one
    // spelling that is unambiguously inert.
    if value.is_empty() {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    match shape {
        Shape::Command => crate::command_verdict(value),
        Shape::ExecPath => exec_path_verdict(value),
        Shape::DataPath => data_path_verdict(value),
        Shape::Opaque => Verdict::Denied,
    }
}

/// A path supplying CODE, judged at the EXECUTOR locus — the rule cargo's `--manifest-path` uses.
/// Code from the worktree is the project's own and already trusted; code from `/tmp`, `$HOME` or a
/// system path is foreign and denies.
///
/// Deliberately NOT the read rule. `cat /tmp/x.so` is a perfectly ordinary read, so a read-locus
/// test would admit `LD_PRELOAD=/tmp/x.so`. Loading that file into the process is a different act
/// from reading it.
///
/// A colon-separated list is judged by its WORST element: one attacker-controlled entry is enough,
/// because the loader searches all of them.
fn exec_path_verdict(value: &str) -> Verdict {
    // `execute_file_verdict` IS the executor-locus rule — the same function pathgate's `exec` role
    // uses for `cargo --manifest-path`. Reused rather than re-derived: an earlier cut compared
    // `classify_locus(p) <= Worktree` and admitted `/tmp`, because the locus ladder orders by the
    // blast radius of a WRITE (`temp` sits below `worktree`) and not by trust for EXECUTION.
    worst_element(value, crate::engine::resolve::execute_file_verdict)
}

/// A path read or written as DATA — the ordinary read locus, which already refuses credential and
/// system paths while allowing the worktree.
fn data_path_verdict(value: &str) -> Verdict {
    worst_element(value, crate::engine::resolve::read_content_verdict)
}

fn worst_element(value: &str, judge: fn(&str) -> Verdict) -> Verdict {
    value
        .split(':')
        .filter(|s| !s.is_empty())
        .map(judge)
        .fold(Verdict::Allowed(SafetyLevel::Inert), Verdict::combine)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_parses_and_covers_the_measured_vectors() {
        for name in [
            "LD_PRELOAD", "DYLD_INSERT_LIBRARIES", "BASH_ENV", "PYTHONPATH", "PHPRC",
            "PHP_INI_SCAN_DIR", "RUSTC_WRAPPER", "GIT_SSH_COMMAND", "GIT_DIR",
            "DOTNET_STARTUP_HOOKS", "PERL5LIB", "RUBYLIB",
        ] {
            assert!(shape_of(name).is_some(), "envvars.toml is missing a measured vector: {name}");
        }
    }

    #[test]
    fn an_unlisted_name_is_inert() {
        assert!(shape_of("PROJECT").is_none());
        assert!(shape_of("NODE_ENV").is_none());
        assert!(shape_of("RUSTUP_TOOLCHAIN").is_none());
        for n in ["PROJECT", "NODE_ENV", "RUSTUP_TOOLCHAIN", "WRITE", "FOO"] {
            assert_eq!(assignment_verdict(n, "anything"), Verdict::Allowed(SafetyLevel::Inert));
        }
    }

    /// A variable whose name carries a variable segment — cargo's target triple, git's config
    /// index — is reachable only by pattern.
    #[test]
    fn a_name_glob_matches_the_variable_segment() {
        assert_eq!(shape_of("CARGO_TARGET_X86_64_APPLE_DARWIN_RUNNER"), Some(Shape::Command));
        assert_eq!(shape_of("CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER"), Some(Shape::Command));
        assert_eq!(shape_of("GIT_CONFIG_KEY_0"), Some(Shape::Opaque));
        assert_eq!(shape_of("GIT_CONFIG_VALUE_17"), Some(Shape::Opaque));
        // ...and does not over-reach onto neighbouring names.
        assert_eq!(shape_of("CARGO_TARGET_DIR"), None);
        assert_eq!(shape_of("CARGO_TERM_COLOR"), None);
    }

    /// The prefix and suffix must not overlap to satisfy a pattern with nothing in the middle.
    #[test]
    fn a_glob_needs_a_real_middle_segment() {
        assert_eq!(shape_of("CARGO_TARGET__RUNNER"), Some(Shape::Command), "empty middle is still a match");
        assert_eq!(shape_of("CARGO_TARGET_RUNNER"), None, "prefix and suffix must not overlap");
    }

    #[test]
    fn an_empty_value_is_inert_even_for_a_listed_name() {
        for n in ["GIT_PAGER", "LD_PRELOAD", "GIT_DIR"] {
            assert_eq!(assignment_verdict(n, ""), Verdict::Allowed(SafetyLevel::Inert));
        }
    }
}

/// End-to-end behaviour through `command_verdict`, which is what actually ships.
#[cfg(test)]
mod integration_tests {
    /// The vectors measured in `docs/design/env-prefix-classification.md`. Each was proved to
    /// execute on a real toolchain before being listed, so each must deny here.
    #[test]
    fn every_measured_injection_vector_denies() {
        for cmd in [
            "LD_PRELOAD=/tmp/evil.so ls",
            "LD_AUDIT=/tmp/evil.so ls",
            "DYLD_INSERT_LIBRARIES=/tmp/evil.dylib cat notes.txt",
            "BASH_ENV=/tmp/evil.sh bash -c ls",
            "PYTHONPATH=/tmp/inj python3 --version",
            "PHPRC=/tmp/evil/php.ini php --version",
            "PHP_INI_SCAN_DIR=/tmp/evil php --version",
            "RUSTC_WRAPPER=/tmp/evil cargo build",
            "CARGO_BUILD_RUSTC_WRAPPER=/tmp/evil cargo build",
            "DOTNET_STARTUP_HOOKS=/tmp/evil.dll dotnet --info",
            "GIT_SSH_COMMAND='sh -c evil' git status",
            "GIT_DIR=/tmp/evil git log",
            "PERL5LIB=/tmp/inj perl --version",
            "RUBYLIB=/tmp/inj ruby --version",
        ] {
            assert!(!crate::is_safe_command(cmd), "a measured injection vector was allowed: {cmd}");
        }
    }

    /// The whole point of a LISTED table: everything outside it behaves exactly as before. A
    /// regression here is friction imposed on every user for no security gain, which is the failure
    /// mode that sank the earlier "undeclared name denies" design.
    #[test]
    fn an_unlisted_assignment_changes_nothing() {
        for cmd in [
            "FOO=bar ls",
            "RUSTUP_TOOLCHAIN=stable cargo test",
            "RACK_ENV=test bundle install",
            "RAILS_ENV=test bundle exec rspec",
            "NODE_ENV=production npm ci --ignore-scripts",
            "PROJECT=safe-chains QUERY=x ls",
            "TZ=UTC date",
        ] {
            assert!(crate::is_safe_command(cmd), "an unlisted assignment caused a denial: {cmd}");
        }
    }

    /// A listed name is not a ban: the value decides. These are the benign spellings of variables
    /// that appear in the table, and they must keep working.
    #[test]
    fn a_listed_name_with_a_benign_value_still_allows() {
        for cmd in [
            "GIT_PAGER=cat git log",
            "GIT_PAGER= git log",          // empty un-sets it
            "GIT_DIR=.git git log",
            "PYTHONPATH=./lib ls",
            "PERL5LIB=./lib ls",
            "LD_PRELOAD= ls",
        ] {
            assert!(crate::is_safe_command(cmd), "a benign value was denied: {cmd}");
        }
    }

    /// A colon-separated list is judged by its WORST element — the loader searches all of them, so
    /// one attacker-controlled entry is enough.
    #[test]
    fn a_path_list_is_judged_by_its_worst_element() {
        assert!(crate::is_safe_command("PYTHONPATH=./a:./b ls"));
        assert!(!crate::is_safe_command("PYTHONPATH=./a:/tmp/evil ls"));
        assert!(!crate::is_safe_command("PYTHONPATH=/tmp/evil:./b ls"));
    }

    /// Env twins of flags already denied. In each case the danger was documented in the command's
    /// own TOML and only the environment spelling went unchecked.
    #[test]
    fn an_env_twin_of_a_denied_flag_also_denies() {
        for (flag_form, env_form) in [
            ("cargo test --config target.x86_64-apple-darwin.runner=/tmp/evil",
             "CARGO_TARGET_X86_64_APPLE_DARWIN_RUNNER=/tmp/evil cargo test"),
            ("cargo build --config build.rustc-wrapper=/tmp/evil",
             "RUSTC_WRAPPER=/tmp/evil cargo build"),
            ("git -c core.pager='sh -c evil' log",
             "GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.pager GIT_CONFIG_VALUE_0='sh -c evil' git log"),
        ] {
            assert!(!crate::is_safe_command(flag_form), "sanity: the flag form should deny: {flag_form}");
            assert!(!crate::is_safe_command(env_form), "the env twin was allowed: {env_form}");
        }
    }

    /// A twin entry names a mechanism, not a ban: a benign runner is still a benign command.
    #[test]
    fn a_twin_with_a_benign_value_still_allows() {
        assert!(crate::is_safe_command("CARGO_TARGET_X86_64_APPLE_DARWIN_RUNNER=echo cargo test"));
        assert!(crate::is_safe_command("CARGO_TERM_COLOR=always cargo test"));
        assert!(crate::is_safe_command("CARGO_TARGET_DIR=./target cargo build"));
    }

    /// The exec-path rule is NOT the read rule, and the distinction is load-bearing: reading
    /// `/tmp/x.so` is ordinary, while loading it into the process is not. An earlier cut used a
    /// locus comparison that admitted `/tmp` because `temp` sits BELOW `worktree` on a ladder
    /// ordered by write blast-radius rather than execution trust.
    #[test]
    fn loading_from_temp_denies_even_though_reading_it_allows() {
        assert!(crate::is_safe_command("cat /tmp/evil.so"), "reading /tmp is ordinary");
        assert!(!crate::is_safe_command("LD_PRELOAD=/tmp/evil.so ls"), "loading it must not be");
    }
}
