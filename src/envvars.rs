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
    /// The value is a FLAG STRING for an interpreter (`RUBYOPT`, `NODE_OPTIONS`, `RUSTFLAGS`).
    /// Every token must match a researched-inert prefix in the entry's `allowed` list; anything
    /// else denies. Allowlist-shaped, and bounded because each interpreter's accepted set is small
    /// and documented — ruby and perl even refuse `-e` themselves, so the eval vector is already
    /// closed upstream.
    OptionString,
}

#[derive(Deserialize, Debug)]
struct Entry {
    shape: Shape,
    /// For `option-string`: token prefixes that are inert. A token matching none of these denies.
    #[serde(default)]
    allowed: Vec<String>,
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

#[derive(Clone)]
pub(crate) struct Rule {
    pub(crate) shape: Shape,
    allowed: Vec<String>,
}

struct Compiled {
    exact: HashMap<String, Rule>,
    /// `(prefix, suffix, shape)` from a name containing a single `*`. Needed because some variables
    /// carry a variable segment: cargo spells its per-target keys `CARGO_TARGET_<TRIPLE>_RUNNER`,
    /// and git numbers its config pairs `GIT_CONFIG_KEY_0`, `_1`, …
    globs: Vec<(String, String, Rule)>,
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
                globs.push((pre.to_string(), suf.to_string(),
                            Rule { shape: entry.shape, allowed: entry.allowed }));
            }
            None => {
                exact.insert(name, Rule { shape: entry.shape, allowed: entry.allowed });
            }
        }
    }
    Compiled { exact, globs }
});

fn rule_of(name: &str) -> Option<&'static Rule> {
    if let Some(r) = TABLE.exact.get(name) {
        return Some(r);
    }
    TABLE.globs.iter().find_map(|(pre, suf, rule)| {
        // `len()` guard so a single `*` cannot match the empty middle twice over — `A_*_B` must not
        // match `A__B` by letting prefix and suffix overlap.
        (name.len() >= pre.len() + suf.len() && name.starts_with(pre.as_str()) && name.ends_with(suf.as_str()))
            .then_some(rule)
    })
}

/// The shape of a listed variable's value, or `None` for a name we do not inspect. Test-only: the
/// classifier itself goes through `rule_of`, which also carries the option-string allowlist.
#[cfg(test)]
pub(crate) fn shape_of(name: &str) -> Option<Shape> {
    rule_of(name).map(|r| r.shape)
}

/// The verdict for one `NAME=value` assignment. `Inert` for any name not in the table — the
/// overwhelmingly common case, and the reason this costs nothing for ordinary commands.
pub(crate) fn assignment_verdict(name: &str, value: &str) -> Verdict {
    let Some(rule) = rule_of(name) else {
        return Verdict::Allowed(SafetyLevel::Inert);
    };
    // An EMPTY value un-sets the behaviour rather than pointing it anywhere: `GIT_PAGER=` disables
    // the pager, `LD_PRELOAD=` preloads nothing. Denying that would be a false positive on the one
    // spelling that is unambiguously inert.
    if value.is_empty() {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    match rule.shape {
        Shape::Command => crate::command_verdict(value),
        Shape::ExecPath => exec_path_verdict(value),
        Shape::DataPath => data_path_verdict(value),
        Shape::Opaque => Verdict::Denied,
        Shape::OptionString => option_string_verdict(value, &rule.allowed),
    }
}

/// Every token of an interpreter flag string must match a researched-inert prefix.
///
/// Allowlist-shaped on purpose: the dangerous flags never need enumerating, so a switch invented
/// tomorrow denies by not being listed. That matters here more than elsewhere, because these
/// variables carry the loader/debugger/permission flags — `--require`, `-C linker=`, `--inspect` —
/// and the accepted sets are large enough that a denylist would rot.
///
/// `-C key=value` is accepted in both spellings: rust and friends allow `-C linker=x` and
/// `-Clinker=x`, so a lone one-or-two-character flag is joined to the token after it before
/// matching. Without that, allowing a bare `-C` would admit `-C linker=/tmp/evil`.
fn option_string_verdict(value: &str, allowed: &[String]) -> Verdict {
    let raw: Vec<&str> = value.split_whitespace().collect();
    let mut tokens: Vec<String> = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        let tok = raw[i];
        // A bare short flag that takes a separate value (`-C opt-level=3`, `-I lib`) is glued to
        // the next token so the pair is matched as one unit.
        if tok.len() <= 2 && tok.starts_with('-') && i + 1 < raw.len() && !raw[i + 1].starts_with('-') {
            tokens.push(format!("{tok}{}", raw[i + 1]));
            i += 2;
        } else {
            tokens.push(tok.to_string());
            i += 1;
        }
    }
    for tok in &tokens {
        if !allowed.iter().any(|a| tok.starts_with(a.as_str())) {
            return Verdict::Denied;
        }
    }
    Verdict::Allowed(SafetyLevel::Inert)
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

    /// Interpreter flag strings: every token must be a researched-inert prefix.
    #[test]
    fn an_option_string_denies_a_flag_that_is_not_allowlisted() {
        for cmd in [
            // measured to execute, per the design note
            "RUSTFLAGS='-C linker=/tmp/evil' cargo build",
            "CARGO_BUILD_RUSTFLAGS='-Clinker=/tmp/evil' cargo build",
            "NODE_OPTIONS='--require /tmp/inj.js' node app.js",
            "NODE_OPTIONS='--import file:///tmp/inj.mjs' node app.js",
            "RUBYOPT='-r/tmp/inj' ruby app.rb",
            "PERL5OPT='-I/tmp -MInject' perl app.pl",
            "PERL5OPT='-I/tmp -d:Inj' perl app.pl",
            // not code loading, but equally out of scope for an auto-approval
            "NODE_OPTIONS='--inspect=0.0.0.0:9229' node app.js",
            "NODE_OPTIONS='--allow-child-process' node app.js",
            "JAVA_TOOL_OPTIONS='-javaagent:/tmp/e.jar' java -version",
        ] {
            assert!(!crate::is_safe_command(cmd), "an option-string vector was allowed: {cmd}");
        }
    }

    /// The everyday spellings have to keep working, or the entry is friction with no gain.
    #[test]
    fn an_option_string_allows_the_researched_inert_flags() {
        for cmd in [
            "RUSTFLAGS='-D warnings' cargo build",
            "RUSTFLAGS='-C opt-level=3' cargo build",
            "RUSTFLAGS='-Copt-level=3 -Cdebuginfo=0' cargo build",
            "RUSTDOCFLAGS='-D warnings' cargo doc",
            "NODE_OPTIONS='--max-old-space-size=4096' node app.js",
            "NODE_OPTIONS='--enable-source-maps' node app.js",
            "RUBYOPT='-w' ruby app.rb",
            // A trivial base command: `perl app.pl` denies on its own terms, so using it here would
            // assert nothing about PERL5OPT. Isolating the variable is the point of the case.
            "PERL5OPT='-w -T' ls",
        ] {
            assert!(crate::is_safe_command(cmd), "an inert flag string was denied: {cmd}");
        }
    }

    /// `-C key=value` and `-Ckey=value` are the same flag, so the split spelling must not sneak a
    /// denied key past by hiding it in the following token. Without joining, allowing a bare `-C`
    /// would admit `-C linker=/tmp/evil`.
    #[test]
    fn a_split_short_flag_is_judged_as_one_unit() {
        assert!(!crate::is_safe_command("RUSTFLAGS='-C linker=/tmp/evil' cargo build"));
        assert!(!crate::is_safe_command("RUSTFLAGS='-Clinker=/tmp/evil' cargo build"));
        assert!(crate::is_safe_command("RUSTFLAGS='-C opt-level=3' cargo build"));
        assert!(crate::is_safe_command("RUSTFLAGS='-Copt-level=3' cargo build"));
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
