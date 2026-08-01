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

/// The role a path-flag's value plays, spelled as `registry::types::PathRole` spells it: read is
/// gated by the read locus, write by the write locus.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PathRole {
    Read,
    Write,
    /// The value supplies CODE — gated at the executor locus, exactly as the `exec-path` shape is,
    /// so `-cp ./lib` and `CLASSPATH=./lib` give the same answer.
    Exec,
}

/// How a flag is joined to its value. Measured per flag, because a tool generally accepts only one
/// spelling each: `--module-path=<path>` takes equals and rejects a space, `-cp <path>` the reverse.
///
/// Colon-joined flags (`-Xbootclasspath/a:<path>`) exist but no entry declares one, so that variant
/// is deliberately absent rather than written ahead of a use — an unexercised branch in a classifier
/// is a branch nothing has ever checked. Adding a colon flag means adding the variant with it.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
enum Sep {
    #[default]
    Equals,
    Space,
}

/// An option-string flag whose VALUE is a path, gated by locus instead of being refused outright.
/// The analogue of `[[command.sub.flag]]`'s `path_flags` for a flag that lives inside an environment
/// variable's option string rather than on the command line.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PathFlagEntry {
    flag: String,
    role: PathRole,
    #[serde(default)]
    sep: Sep,
}

// `deny_unknown_fields` as `registry::types` does it: a typo'd key must fail at parse time, not be
// silently dropped. Every typo available today happens to fail CLOSED (a missing `allowed` denies
// everything, a missing `path_flags` denies the flag), but that is luck rather than design — a
// future field that RESTRICTS would fail open under the same silence.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct Entry {
    shape: Shape,
    /// For `option-string`: token prefixes that are inert. A token matching none of these denies.
    #[serde(default)]
    allowed: Vec<String>,
    /// For `option-string`: flags whose `=value` is a PATH, judged by the ordinary locus rules.
    #[serde(default)]
    path_flags: Vec<PathFlagEntry>,
    #[allow(dead_code)] // authored rationale, carried for the docs and for review
    because: String,
    #[allow(dead_code)] // false = enumerated from docs, not probed on a live toolchain
    #[serde(default)]
    measured: Option<bool>,
    /// The FLAG spelling of the same thing, as an invocation template with `{v}` for the value —
    /// e.g. `rsync -e {v} ./src/ ./dst/` for `RSYNC_RSH`.
    ///
    /// Gating one spelling and not the other is not a partial fix: the ungated spelling is a
    /// complete bypass, and the gated one reads as closed while it is not. Every executor flag
    /// gated in the registry was bypassable through its env twin until these were paired.
    /// This variable holds ONE value, not a colon-separated list — a command like `BORG_RSH`,
    /// not a search path like `PYTHONPATH`. Splitting such a value makes the env gate disagree
    /// with the flag gate that judges the very same string. Opt-OUT because the reverse mistake
    /// (a real list judged whole) is a fail-open.
    #[serde(default)]
    single_value: bool,
    #[allow(dead_code)] // read by the twin-pairing guard (cfg(test))
    #[serde(default)]
    twin_flag: Option<String>,
    /// The command the variable prefixes, for building the env spelling of the same invocation —
    /// e.g. `rsync ./src/ ./dst/`. Required with `twin_flag`.
    #[allow(dead_code)] // read by the twin-pairing guard (cfg(test))
    #[serde(default)]
    twin_base: Option<String>,
}

/// Declared (env-name, flag-template, base-command) triples, for the pairing guard. Re-parses the
/// embedded TOML because the compiled table keeps only what classification needs.
#[cfg(test)]
pub(crate) fn declared_twins() -> Vec<(String, String, String)> {
    let parsed: Table =
        toml::from_str(include_str!("../envvars.toml")).expect("embedded envvars.toml must parse");
    let mut out: Vec<(String, String, String)> = parsed
        .env
        .into_iter()
        .filter_map(|(name, e)| Some((name, e.twin_flag?, e.twin_base?)))
        .collect();
    out.sort();
    out
}

#[derive(Deserialize, Debug)]
struct Table {
    env: HashMap<String, Entry>,
}

#[derive(Clone)]
pub(crate) struct Rule {
    pub(crate) shape: Shape,
    pub(crate) single_value: bool,
    allowed: Vec<String>,
    path_flags: Vec<PathFlagEntry>,
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
                            Rule { shape: entry.shape, single_value: entry.single_value, allowed: entry.allowed, path_flags: entry.path_flags }));
            }
            None => {
                exact.insert(name, Rule { shape: entry.shape, single_value: entry.single_value, allowed: entry.allowed, path_flags: entry.path_flags });
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
    // `NAME+=VALUE` is bash's APPEND assignment: it sets NAME, not a variable called `NAME+`. A
    // POSIX variable name cannot contain `+`, so a trailing one can only have come from that
    // operator. Normalised here rather than at each call site, so every spelling inherits it —
    // `export LD_PRELOAD+=/tmp/evil.so` and `declare -x LD_PRELOAD+=…` both appended to the real
    // LD_PRELOAD while classifying as the unlisted name `LD_PRELOAD+`, and auto-approved.
    let name = name.strip_suffix('+').unwrap_or(name);
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
        Shape::ExecPath => exec_path_verdict(value, !rule.single_value),
        Shape::DataPath => data_path_verdict(value, !rule.single_value),
        Shape::Opaque => Verdict::Denied,
        Shape::OptionString => option_string_verdict(value, &rule.allowed, &rule.path_flags),
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
///
/// An entry matches a WHOLE FLAG unless it ends in `*`. Plain `starts_with` matching silently
/// admitted every longer flag that happened to share a listed one's spelling: `-mod` let
/// `-modfile=/tmp/evil.mod` through, and `-v` let `-vet=off` through — flags nobody researched,
/// admitted by an accident of naming. No mechanical rule can tell those apart from the legitimate
/// glued values, since rustc's `-D warnings` glues a letter exactly as `-modfile` does, so the
/// author states it: `-Xmx*` is a prefix, `-mod` is a whole flag that may carry `=value`.
fn option_string_verdict(value: &str, allowed: &[String], path_flags: &[PathFlagEntry]) -> Verdict {
    let raw: Vec<&str> = value.split_whitespace().collect();
    let mut verdict = Verdict::Allowed(SafetyLevel::Inert);
    let mut i = 0;
    while i < raw.len() {
        // A declared path flag is judged on its VALUE by the ordinary locus rules, so an in-worktree
        // target allows and an out-of-worktree one denies — the same answer the command-line
        // spelling of that path would get. Checked before `allowed` so a flag cannot be listed in
        // both and reach the ungated branch.
        if let Some((role, path, used)) = path_flag_match(raw[i], raw.get(i + 1).copied(), path_flags)
        {
            if path.is_empty() {
                return Verdict::Denied;
            }
            verdict = verdict.combine(gate_path(role, path));
            i += used;
            continue;
        }
        // A bare short flag that takes a separate value (`-C opt-level=3`, `-I lib`) is glued to the
        // next token so the pair is matched as one unit.
        let joined;
        let unit = if raw[i].len() <= 2
            && raw[i].starts_with('-')
            && i + 1 < raw.len()
            && !raw[i + 1].starts_with('-')
        {
            joined = format!("{}{}", raw[i], raw[i + 1]);
            i += 2;
            joined.as_str()
        } else {
            i += 1;
            raw[i - 1]
        };
        if !allowed.iter().any(|a| flag_matches(unit, a)) {
            return Verdict::Denied;
        }
    }
    verdict
}

/// `(role, value, tokens consumed)` when `tok` is a declared path flag. A space-separated flag with
/// nothing after it yields an empty value, which the caller refuses — a flag whose path we cannot
/// see is not the bare flag.
fn path_flag_match<'a>(
    tok: &'a str,
    next: Option<&'a str>,
    path_flags: &[PathFlagEntry],
) -> Option<(PathRole, &'a str, usize)> {
    path_flags.iter().find_map(|pf| match pf.sep {
        Sep::Equals => tok
            .split_once('=')
            .filter(|(f, _)| *f == pf.flag)
            .map(|(_, v)| (pf.role, v, 1)),
        // A `-`-prefixed next token is NOT the value: `java` rejects `-cp -XX:…` outright with
        // "requires class path specification". Refusing to consume it keeps that token in the
        // normal flag path instead of swallowing it unvalidated, which is the same rule the
        // short-flag join below follows.
        Sep::Space => (tok == pf.flag).then_some(match next {
            Some(v) if !v.starts_with('-') => (pf.role, v, 2),
            _ => (pf.role, "", 1),
        }),
    })
}

/// A path flag's value, judged by the same rules the standalone shapes use — so `-cp ./lib` and
/// `CLASSPATH=./lib` cannot disagree.
fn gate_path(role: PathRole, path: &str) -> Verdict {
    // These are the values of flags INSIDE an option-string (`JDK_JAVA_OPTIONS='-cp …'`), and the
    // ones that carry paths are classpath-shaped, so they split like the standalone list shapes.
    match role {
        PathRole::Read => data_path_verdict(path, true),
        PathRole::Write => crate::engine::resolve::write_target_verdict(path),
        PathRole::Exec => exec_path_verdict(path, true),
    }
}

/// `entry*` admits anything beginning with `entry`; a bare `entry` admits only `entry` itself or
/// `entry=value`, so a longer flag that merely starts the same way is not swept in.
fn flag_matches(token: &str, entry: &str) -> bool {
    match entry.strip_suffix('*') {
        Some(prefix) => token.starts_with(prefix),
        None => token == entry || token.strip_prefix(entry).is_some_and(|r| r.starts_with('=')),
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
fn exec_path_verdict(value: &str, split_list: bool) -> Verdict {
    // `execute_file_verdict` IS the executor-locus rule — the same function pathgate's `exec` role
    // uses for `cargo --manifest-path`. Reused rather than re-derived: an earlier cut compared
    // `classify_locus(p) <= Worktree` and admitted `/tmp`, because the locus ladder orders by the
    // blast radius of a WRITE (`temp` sits below `worktree`) and not by trust for EXECUTION.
    worst_element(value, crate::engine::resolve::execute_file_verdict, split_list)
}

/// A path read or written as DATA — the ordinary read locus, which already refuses credential and
/// system paths while allowing the worktree.
fn data_path_verdict(value: &str, split_list: bool) -> Verdict {
    worst_element(value, crate::engine::resolve::read_content_verdict, split_list)
}

/// Delegates to the ONE shared rule so the environment gate and the flag gate cannot drift apart
/// again — they had separate copies and disagreed on `x:/tmp/evil`.
fn worst_element(value: &str, judge: fn(&str) -> Verdict, split_list: bool) -> Verdict {
    crate::engine::resolve::worst_path_element(value, judge, split_list)
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
            "CLASSPATH", "JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS", "GOFLAGS",
            "LUA_INIT", "JULIA_LOAD_PATH", "JULIA_DEPOT_PATH", "R_PROFILE_USER",
        ] {
            assert!(shape_of(name).is_some(), "envvars.toml is missing a measured vector: {name}");
        }
    }

    /// The shared list rule: split only when asked, and never split a URL.
    #[test]
    fn worst_path_element_splits_only_a_list_and_never_a_url() {
        use crate::engine::resolve::{execute_file_verdict, worst_path_element};

        // As a LIST, an out-of-workspace element condemns the whole value.
        assert!(!worst_path_element("./ok:/tmp/evil", execute_file_verdict, true).is_allowed());
        assert!(!worst_path_element("/tmp/evil:./ok", execute_file_verdict, true).is_allowed());
        assert!(worst_path_element("./a:./b", execute_file_verdict, true).is_allowed());

        // As a SINGLE value the same string is one filename, judged whole. This is what lets the
        // env spelling agree with the flag spelling, which judges it the same way.
        assert!(worst_path_element("x:/tmp/evil", execute_file_verdict, false).is_allowed());
        assert!(!worst_path_element("x:/tmp/evil", execute_file_verdict, true).is_allowed());

        // A URL is never a path list. Splitting `https://example.com` into `https` and
        // `//example.com` denied every curl invocation in the suite.
        for url in ["https://example.com", "http://a.b/c", "ssh://host/path"] {
            assert!(
                worst_path_element(url, execute_file_verdict, true).is_allowed()
                    == worst_path_element(url, execute_file_verdict, false).is_allowed(),
                "splitting changed the verdict for a URL: {url}"
            );
        }

        // Splitting can only ever be STRICTER, never looser — the property that makes it the safe
        // default for anything not explicitly marked single-valued.
        for v in ["./a", "/tmp/x", "a:b", ":/:", "./ok:/etc/shadow", "x", "/", ".."] {
            let split = worst_path_element(v, execute_file_verdict, true).is_allowed();
            let whole = worst_path_element(v, execute_file_verdict, false).is_allowed();
            assert!(!split || whole, "splitting was LOOSER than judging whole for {v:?}");
        }
    }

    /// A variable declared `single_value` must not be split, and one that is not must be.
    ///
    /// The direction matters and is not symmetric: a real LIST judged whole is a fail-open
    /// (`PYTHONPATH=/tmp/evil:/ok` matches no locus rule as one string), while a single value
    /// split is merely stricter. So splitting is the default and `single_value` is the exception.
    #[test]
    fn list_variables_still_split_and_single_value_ones_do_not() {
        for listy in ["PYTHONPATH", "CLASSPATH", "PERL5LIB", "RUBYLIB", "NODE_PATH"] {
            assert!(
                !assignment_verdict(listy, "/tmp/evil:./ok").is_allowed(),
                "{listy} stopped splitting; an out-of-workspace element now rides in behind a prefix"
            );
        }
        for single in ["BORG_RSH", "RSYNC_RSH", "RESTIC_PASSWORD_COMMAND", "BORG_REMOTE_PATH"] {
            assert!(
                !assignment_verdict(single, "/tmp/evil").is_allowed(),
                "{single} must still refuse a foreign executor"
            );
            assert!(
                assignment_verdict(single, "ssh").is_allowed(),
                "{single} must still accept a bare name on $PATH"
            );
        }
    }

    /// A variable tagged with its FLAG twin must classify the same as that flag, value for value.
    ///
    /// Gating one spelling and leaving the other open is not a partial fix — the open spelling is a
    /// complete bypass, and the closed one reads as done while it is not. Every executor flag gated
    /// in the registry was reachable through its environment twin until these were paired:
    /// `borg --rsh /tmp/evil` denied while `BORG_RSH=/tmp/evil borg` sailed through.
    ///
    /// Checks EQUALITY rather than "both deny", so it fails in both directions: an env twin that
    /// stays open, and a flag that over-denies a value its twin accepts. The corpus spans a foreign
    /// path, a workspace path and a bare name, and the pair must discriminate between them — a
    /// tag whose two sides agree only by refusing everything proves nothing.
    #[test]
    fn a_tagged_env_var_classifies_the_same_as_its_flag_twin() {
        let twins = super::declared_twins();
        assert!(!twins.is_empty(), "no env/flag twins are declared; the guard would be vacuous");

        let mut failures = Vec::new();
        for (name, flag_tmpl, base) in &twins {
            let mut verdicts = Vec::new();
            // Widened after the metamorphic fuzz target found a disagreement on `:/:` that the
            // original three values missed. Three values was never a proof; these span the shapes
            // that actually distinguish the two gates - colons, traversal, roots, bare names.
            for value in [
                "/tmp/evil", "./bin/tool", "ssh", ":/:", "x:/tmp/evil", "a:b", "..", "/",
                "./a:./b", "~/.ssh/id_rsa", ".", "/etc/shadow", "sub/dir/tool",
                // Command LINES. Every value above is a single token, so the corpus could not see
                // the case where one gate treats the value as a path and the other as a command:
                // `--rsh 'sh -c evil'` was auto-approved (the pathgate pre-filter skipped it as
                // "not path-shaped") while `BORG_RSH='sh -c evil'` denied. The nightly fuzz found
                // only a lookalike symptom (`fiLe:~`) because its transform excludes whitespace,
                // so this shape has to be carried here.
                "sh -c evil", "ssh -p 2222", "/bin/sh -c x",
            ] {
                // Multi-word values are quoted so both spellings still see ONE value; unquoted,
                // the extra words would become separate arguments and the two forms would stop
                // denoting the same operation.
                let quoted = if value.contains(char::is_whitespace) {
                    format!("'{value}'")
                } else {
                    value.to_string()
                };
                let flag_form = flag_tmpl.replace("{v}", &quoted);
                let env_form = format!("{name}={quoted} {base}");
                let by_flag = crate::is_safe_command(&flag_form);
                let by_env = crate::is_safe_command(&env_form);
                if by_flag != by_env {
                    failures.push(format!(
                        "{name}: `{flag_form}` -> {by_flag} but `{env_form}` -> {by_env}"
                    ));
                }
                verdicts.push(by_flag);
            }
            if verdicts.iter().all(|v| *v) || verdicts.iter().all(|v| !*v) {
                failures.push(format!(
                    "{name}: the pair never discriminates (every value classifies the same), so \
                     agreement between the spellings proves nothing"
                ));
            }
        }
        assert!(failures.is_empty(), "env/flag twin mismatches:\n  {}", failures.join("\n  "));
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
            // Probed on OpenJDK 26.0.2, go1.26.5, Lua 5.5.0, Julia 1.12.6 and R 4.6.1. Each of
            // these was watched writing a marker file before being listed here.
            "CLASSPATH=/tmp/evil java Run",
            "JAVA_TOOL_OPTIONS='-javaagent:/tmp/e.jar' java -version",
            "_JAVA_OPTIONS='-javaagent:/tmp/e.jar' java -version",
            "JDK_JAVA_OPTIONS='-javaagent:/tmp/e.jar' java -version",
            "JAVA_TOOL_OPTIONS='-XX:OnOutOfMemoryError=/tmp/e.sh' java -version",
            "JAVA_TOOL_OPTIONS='-XX:OnError=/tmp/e.sh' java -version",
            "GOFLAGS='-toolexec=/tmp/evil' go list ./...",
            "GOFLAGS='-overlay=/tmp/o.json' go list ./...",
            "GOFLAGS='-modfile=/tmp/alt.mod' go list ./...",
            "LUA_INIT='os.execute(\"id\")' ls",
            "LUA_INIT=@/tmp/init.lua ls",
            "JULIA_LOAD_PATH=/tmp/evil ls",
            "JULIA_DEPOT_PATH=/tmp/evil ls",
            "R_PROFILE_USER=/tmp/evil.R ls",
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
            // Java. Documented rather than measured, so the corpus is wider: every excluded class
            // gets a case. `java -version` allows on its own, so each denial is the variable's.
            "JAVA_TOOL_OPTIONS='-javaagent:/tmp/e.jar' java -version",
            "JAVA_TOOL_OPTIONS='-agentlib:jdwp=transport=dt_socket' java -version",
            "JAVA_TOOL_OPTIONS='-agentpath:/tmp/e.dylib' java -version",
            "JAVA_TOOL_OPTIONS='-Xbootclasspath/a:/tmp/e.jar' java -version",
            "JAVA_TOOL_OPTIONS='-cp /tmp/evil' java -version",
            "JAVA_TOOL_OPTIONS='--module-path /tmp/evil' java -version",
            // -XX: is not a blanket prefix: these two run a command, wearing the same prefix as the
            // GC tuning flags that ARE listed.
            "JAVA_TOOL_OPTIONS='-XX:OnError=id' java -version",
            "JAVA_TOOL_OPTIONS='-XX:OnOutOfMemoryError=id' java -version",
            "JAVA_TOOL_OPTIONS='-XX:VMOptionsFile=/tmp/o' java -version",
            // -D is not listed: system properties reach remote class loading and self-attach.
            "JAVA_TOOL_OPTIONS='-Djava.rmi.server.codebase=http://x/' java -version",
            "_JAVA_OPTIONS='-javaagent:/tmp/e.jar' java -version",
            "JDK_JAVA_OPTIONS='-XX:OnError=id' java -version",
            // Go. Isolated against `ls`, since `go build` denies on its own terms.
            "GOFLAGS='-toolexec=/tmp/evil' ls",
            "GOFLAGS='-exec=/tmp/evil' ls",
            "GOFLAGS='-overlay=/tmp/o.json' ls",
            "GOFLAGS='-ldflags=-linkmode=external' ls",
            "GOFLAGS='-gcflags=-N' ls",
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
            // Heap and GC tuning is the overwhelmingly common reason to set these at all; denying
            // it would make the entry pure friction.
            "JAVA_TOOL_OPTIONS='-Xmx2g' java -version",
            "JAVA_TOOL_OPTIONS='-Xms512m -Xmx4g -XX:+UseG1GC' java -version",
            "JAVA_TOOL_OPTIONS='-XX:MaxGCPauseMillis=200' java -version",
            "JAVA_TOOL_OPTIONS='-verbose:gc' java -version",
            "_JAVA_OPTIONS='-Xmx2g' java -version",
            "JDK_JAVA_OPTIONS='-Xss4m' java -version",
            "GOFLAGS='-mod=readonly -v' ls",
            "GOFLAGS='-count=1' ls",
            "GOFLAGS='-tags=integration -race' ls",
        ] {
            assert!(crate::is_safe_command(cmd), "an inert flag string was denied: {cmd}");
        }
    }

    /// The three JVM variables are NOT one mechanism — measured, `-cp` is rejected by the VM in the
    /// first two and runs a planted class in the launcher-read third. Their `allowed` lists are kept
    /// identical anyway, as the conservative intersection, because nothing in the file format keeps
    /// three hand-written lists in step: editing one and forgetting the others would leave a flag
    /// denied in one spelling and allowed in another. `JDK_JAVA_OPTIONS` may carry EXTRA path flags
    /// on top (the launcher-only classpath options), so it is a superset, never a divergence.
    #[test]
    fn the_jvm_option_variables_classify_identically() {
        let names = ["JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS"];
        let rules: Vec<_> = names
            .iter()
            .map(|n| super::rule_of(n).unwrap_or_else(|| panic!("{n} is missing from envvars.toml")))
            .collect();
        for (name, rule) in names.iter().zip(&rules) {
            assert_eq!(rule.shape, rules[0].shape, "{name} disagrees on shape");
            assert_eq!(rule.allowed, rules[0].allowed, "{name} has a divergent allowlist");
            // JDK_JAVA_OPTIONS is launcher-read and legitimately carries more; the VM-read pair
            // must match exactly, and every flag the pair declares must survive in the superset.
            if *name == "JDK_JAVA_OPTIONS" {
                for pf in &rules[0].path_flags {
                    assert!(
                        rule.path_flags.contains(pf),
                        "JDK_JAVA_OPTIONS dropped the path flag {}, so the two spellings disagree",
                        pf.flag,
                    );
                }
            } else {
                assert_eq!(rule.path_flags, rules[0].path_flags, "{name} has divergent path flags");
            }
        }
        assert!(!rules[0].allowed.is_empty(), "an empty allowlist would deny every JVM tuning flag");
    }

    /// A listed flag must not act as a prefix for a LONGER flag nobody researched. `-mod` admitted
    /// `-modfile=`, which redirects Go's module resolution, and `-v` admitted `-vet=off` — both
    /// purely because the spellings overlap. `go list` and `go vet` are allowlisted, so GOFLAGS
    /// reaches real invocations and this was not theoretical.
    #[test]
    fn a_listed_flag_does_not_admit_a_longer_flag_that_merely_starts_the_same_way() {
        for cmd in [
            "GOFLAGS='-modfile=/tmp/evil.mod' go list ./...",
            "GOFLAGS='-vet=off' go vet ./...",
            "GOFLAGS='-nolocalimports' go list ./...",
            "GOFLAGS='-runtime=x' go list ./...",
            "RUSTDOCFLAGS='--cfgevil' cargo doc",
        ] {
            assert!(!crate::is_safe_command(cmd), "a longer flag was admitted by prefix: {cmd}");
        }
        // ...while the flag itself, bare or carrying a value, still works.
        for cmd in [
            "GOFLAGS='-mod=vendor' go list ./...",
            "GOFLAGS='-v -x' go list ./...",
            "GOFLAGS=-trimpath go list ./...",
        ] {
            assert!(crate::is_safe_command(cmd), "a listed flag was denied: {cmd}");
        }
    }

    /// A path-valued flag inside an option string is judged on its VALUE by the ordinary locus
    /// rules, so the env spelling and the command-line spelling agree. `-Cincremental` was
    /// originally in `allowed`, where the matcher reads the flag NAME and never the value — so
    /// `RUSTFLAGS='-Cincremental=/etc/cron.d' cargo build` was allowed while `cargo build
    /// --target-dir /etc/x` was denied, the env-twin-of-a-guarded-flag shape this module exists to
    /// close, reintroduced inside it. Declaring it in `path_flags` gates the value instead of
    /// refusing the flag outright.
    #[test]
    fn a_declared_path_flag_is_gated_by_its_locus() {
        // Out-of-worktree: denied, matching what the command-line spelling of that path gets.
        for cmd in [
            "RUSTFLAGS='-Cincremental=/etc/cron.d' cargo build",
            "CARGO_BUILD_RUSTFLAGS='-Cincremental=/etc/x' cargo build",
            "JAVA_TOOL_OPTIONS='-XX:+HeapDumpOnOutOfMemoryError -XX:HeapDumpPath=/etc/x' java -version",
            // No value to judge — fail closed rather than treat it as the bare flag.
            "RUSTFLAGS='-Cincremental=' cargo build",
        ] {
            assert!(!crate::is_safe_command(cmd), "an out-of-worktree path was allowed: {cmd}");
        }
        // In-worktree: allowed, because writing there is what the level already permits.
        for cmd in [
            "RUSTFLAGS='-Cincremental=./target/inc' cargo build",
            "RUSTFLAGS='-Copt-level=3 -Cincremental=./target/inc' cargo build",
            "JAVA_TOOL_OPTIONS='-XX:+HeapDumpOnOutOfMemoryError -XX:HeapDumpPath=./dump.hprof' java -version",
        ] {
            assert!(crate::is_safe_command(cmd), "an in-worktree path was denied: {cmd}");
        }
    }

    /// EVERY SPELLING THAT PUTS A VARIABLE INTO THE ENVIRONMENT MUST AGREE. The prefix form was
    /// classified while `env NAME=V cmd`, `export NAME=V`, `declare -x NAME=V` and `typeset -x
    /// NAME=V` were not — four ways to set `LD_PRELOAD` that auto-approved while the fifth denied.
    /// `export` alone is enough: it applies to every command the shell spawns afterwards, so
    /// `export LD_PRELOAD=/tmp/evil.so && cargo build` was a complete bypass.
    ///
    /// Walks the REAL table rather than a hand-picked list, so a variable added tomorrow is covered
    /// in every spelling without anyone remembering to add a case.
    #[test]
    fn every_spelling_of_an_assignment_agrees() {
        // Dangerous under every shape: a command that is an out-of-worktree binary, a path that is
        // out-of-worktree, an unlisted option-string token, and any value at all for `opaque`.
        const BAD: &str = "/tmp/evil";
        let mut checked = 0;
        let names: Vec<String> = super::TABLE
            .exact
            .keys()
            .cloned()
            .chain(super::TABLE.globs.iter().map(|(pre, suf, _)| format!("{pre}X{suf}")))
            .collect();
        for name in names {
            // Skip safely: if the prefix spelling does not deny this value, there is no denial for
            // the other spellings to match, and the case proves nothing.
            if crate::is_safe_command(&format!("{name}={BAD} ls")) {
                continue;
            }
            checked += 1;
            for spelling in [
                format!("env {name}={BAD} ls"),
                format!("export {name}={BAD}"),
                format!("export {name}={BAD} && ls"),
                format!("declare -x {name}={BAD}"),
                format!("typeset -x {name}={BAD}"),
                // `NAME+=VALUE` is bash's APPEND assignment — it sets NAME, so it is the same
                // capability. Splitting on `=` yielded the unlisted name `NAME+`, and all three
                // of these auto-approved.
                format!("export {name}+={BAD}"),
                format!("declare -x {name}+={BAD}"),
                format!("env {name}+={BAD} ls"),
                // `mise set` writes the variable into mise.toml, so it PERSISTS across sessions
                // and injects into every command run in that directory.
                format!("mise set {name}={BAD}"),
            ] {
                assert!(
                    !crate::is_safe_command(&spelling),
                    "`{name}={BAD}` denies as a prefix but `{spelling}` was allowed — one spelling \
                     of the assignment escapes classification",
                );
            }
        }
        assert!(checked > 20, "only {checked} variables exercised; the table was not walked");
    }

    /// Agreement is on the LEVEL too, not just allow/deny — the same lesson as the prefix wiring.
    #[test]
    fn the_alternate_spellings_carry_the_same_level() {
        for (prefix, alternate) in [
            ("PYTHONPATH=./lib true", "export PYTHONPATH=./lib"),
            ("RUSTFLAGS='-Cincremental=./x' true", "export RUSTFLAGS='-Cincremental=./x'"),
            ("FOO=bar true", "export FOO=bar"),
        ] {
            assert_eq!(
                crate::command_verdict(prefix),
                crate::command_verdict(alternate),
                "`{prefix}` and `{alternate}` disagree on level",
            );
        }
    }

    /// An assignment that resolves to a LEVEL must carry that level into the command, not merely
    /// deny or not. `simple_verdict` used to propagate only `Denied` from the env and discard the
    /// level, so `RUSTFLAGS='-Cincremental=./x' echo hi` classified Inert and passed at `paranoid`
    /// while the identical write spelled `touch ./x` did not. Asserting the LEVEL, not allow/deny,
    /// is the point — an allow/deny test passes either way.
    #[test]
    fn an_assignments_level_reaches_the_commands_verdict() {
        use crate::verdict::{SafetyLevel, Verdict};
        // A worktree-write assignment on an inert command word IS a worktree write.
        assert_eq!(
            crate::command_verdict("RUSTFLAGS='-Cincremental=./x' echo hi"),
            crate::command_verdict("touch ./x"),
            "an env-spelled worktree write must classify as that write does",
        );
        assert_eq!(
            crate::command_verdict("RUSTFLAGS='-Cincremental=./x' echo hi"),
            Verdict::Allowed(SafetyLevel::SafeWrite),
        );
        // ...and an assignment with no effect on state does not escalate anything.
        for cmd in ["RUSTFLAGS='-Copt-level=3' echo hi", "FOO=bar echo hi", "echo hi"] {
            assert_eq!(
                crate::command_verdict(cmd),
                Verdict::Allowed(SafetyLevel::Inert),
                "an inert assignment escalated: {cmd}",
            );
        }
    }

    /// A code-supplying flag is gated at the EXECUTOR locus, so it agrees with the environment
    /// variable that expresses the same capability. `CLASSPATH=./lib mvn test` allowed while
    /// `JDK_JAVA_OPTIONS='-cp ./lib' mvn test` denied — one spelling checked, on commands that
    /// really do auto-approve.
    ///
    /// Only JDK_JAVA_OPTIONS carries these: measured on OpenJDK 26.0.2, `-cp` in the VM-read pair is
    /// `Unrecognized option` and the JVM refuses to start, so listing it there would allow a form
    /// that cannot run.
    #[test]
    fn a_classpath_flag_agrees_with_the_classpath_variable() {
        for (env_form, flag_form) in [
            ("CLASSPATH=./lib mvn test", "JDK_JAVA_OPTIONS='-cp ./lib' mvn test"),
            ("CLASSPATH=/tmp/evil mvn test", "JDK_JAVA_OPTIONS='-cp /tmp/evil' mvn test"),
            ("CLASSPATH=./lib gradle build", "JDK_JAVA_OPTIONS='-classpath ./lib' gradle build"),
            ("CLASSPATH=~/.m2 mvn test", "JDK_JAVA_OPTIONS='--class-path ~/.m2' mvn test"),
        ] {
            assert_eq!(
                crate::is_safe_command(env_form),
                crate::is_safe_command(flag_form),
                "the two spellings of one capability disagree: `{env_form}` vs `{flag_form}`",
            );
        }
        assert!(crate::is_safe_command("JDK_JAVA_OPTIONS='-cp ./lib' mvn test"), "worktree classpath");
        assert!(!crate::is_safe_command("JDK_JAVA_OPTIONS='-cp /tmp/evil' mvn test"), "foreign classpath");
        // A classpath is a LIST: one bad element poisons it, as it does for CLASSPATH itself.
        assert!(!crate::is_safe_command("JDK_JAVA_OPTIONS='-cp ./lib:/tmp/evil' mvn test"));
        // The flag with nothing after it has no path to judge, so it is not the bare flag.
        assert!(!crate::is_safe_command("JDK_JAVA_OPTIONS='-cp' mvn test"));
        // Nor does it swallow a following FLAG as if it were the path. `java` rejects that outright
        // ("-cp requires class path specification"), and consuming it here would take a token out of
        // the flag path unvalidated.
        for cmd in [
            "JDK_JAVA_OPTIONS='-cp -XX:OnError=/tmp/e.sh' mvn test",
            "JDK_JAVA_OPTIONS='-cp -javaagent:/tmp/e.jar' mvn test",
            "JDK_JAVA_OPTIONS='-cp -Xmx4g' mvn test",
        ] {
            assert!(!crate::is_safe_command(cmd), "a path flag swallowed a flag as its value: {cmd}");
        }
        // ...and the VM-read pair still refuses it, because the JVM does.
        for cmd in ["JAVA_TOOL_OPTIONS='-cp ./lib' mvn test", "_JAVA_OPTIONS='-cp ./lib' mvn test"] {
            assert!(!crate::is_safe_command(cmd), "a launcher-only flag was admitted to the VM pair: {cmd}");
        }
    }

    /// `path_flags` opens nothing on its own: a flag nobody declared is still judged by `allowed`,
    /// where it is absent, so it denies with or without a value.
    #[test]
    fn an_undeclared_path_flag_still_denies() {
        for cmd in [
            "RUSTFLAGS='-Cprofile-use=./x.profdata' cargo build",
            "JAVA_TOOL_OPTIONS='-XX:VMOptionsFile=./opts' java -version",
            "JAVA_TOOL_OPTIONS='-XX:CompilerDirectivesFile=./d.json' java -version",
            "GOFLAGS='-overlay=./o.json' go list ./...",
            // The declared flag without its `=value` is not the bare flag either.
            "RUSTFLAGS='-Cincremental' cargo build",
            "JAVA_TOOL_OPTIONS='-XX:HeapDumpPath' java -version",
        ] {
            assert!(!crate::is_safe_command(cmd), "an undeclared path flag was allowed: {cmd}");
        }
    }

    /// A flag declared in `path_flags` must NOT also sit in `allowed`. `allowed` is checked second
    /// and matches on the name alone, so a flag in both would reach the ungated branch whenever the
    /// gated one did not match — exactly the hole `path_flags` exists to close. Walks the real
    /// table, so a future entry is covered without anyone adding a case.
    #[test]
    fn no_path_flag_is_also_listed_as_an_ungated_flag() {
        let mut checked = 0;
        for rule in super::TABLE.exact.values().chain(super::TABLE.globs.iter().map(|(_, _, r)| r)) {
            for pf in &rule.path_flags {
                checked += 1;
                assert!(
                    !rule.allowed.iter().any(|a| super::flag_matches(&pf.flag, a)),
                    "{} is declared as a path flag AND matched by `allowed`, so its value escapes \
                     the locus gate",
                    pf.flag,
                );
            }
        }
        assert!(checked > 0, "no path flags seen; the table was not walked");
    }

    /// `allowed` and `path_flags` mean nothing on a shape that is not `option-string` — the other
    /// shapes judge the whole value and never consult either list. Silently ignoring them would let
    /// an author write a restriction that reads as protective and does nothing, so the pairing is
    /// asserted over the real table instead.
    #[test]
    fn option_string_fields_appear_only_on_option_string_entries() {
        for rule in super::TABLE.exact.values().chain(super::TABLE.globs.iter().map(|(_, _, r)| r)) {
            if rule.shape == super::Shape::OptionString {
                assert!(
                    !rule.allowed.is_empty(),
                    "an option-string entry with an empty `allowed` denies every value; say so with \
                     a different shape instead"
                );
                continue;
            }
            assert!(rule.allowed.is_empty(), "`allowed` is set on a non-option-string entry");
            assert!(rule.path_flags.is_empty(), "`path_flags` is set on a non-option-string entry");
        }
    }

    /// An allowlist entry must be a NON-EMPTY FLAG. Two authoring mistakes are invisible otherwise:
    /// a bare `"*"` strips to the empty prefix and `starts_with("")` is always true, so one
    /// character would admit the entire flag surface of that interpreter; and an entry not starting
    /// with `-` matches bare VALUE tokens rather than flags. `every_option_string_entry_obeys_its_own_star`
    /// does not catch either — a bare `*` satisfies "a starred entry accepts its own extension".
    #[test]
    fn every_option_string_entry_is_a_nonempty_flag() {
        for rule in super::TABLE.exact.values().chain(super::TABLE.globs.iter().map(|(_, _, r)| r)) {
            if rule.shape != super::Shape::OptionString {
                continue;
            }
            for entry in &rule.allowed {
                let core = entry.strip_suffix('*').unwrap_or(entry);
                assert!(!core.is_empty(), "a bare `*` entry admits every token of the option string");
                assert!(
                    core.starts_with('-'),
                    "`{entry}` does not start with `-`, so it matches bare value tokens, not a flag"
                );
            }
            for pf in &rule.path_flags {
                assert!(pf.flag.starts_with('-'), "path flag `{}` is not a flag", pf.flag);
            }
        }
    }

    /// The mechanism itself, over the REAL entries rather than a hand-picked pair: a bare entry must
    /// reject an extension of itself, and a `*` entry must accept one. Enumerating the table means a
    /// future entry is covered without anyone remembering to add a case.
    #[test]
    fn every_option_string_entry_obeys_its_own_star() {
        let mut checked = 0;
        for rule in super::TABLE.exact.values().chain(super::TABLE.globs.iter().map(|(_, _, r)| r)) {
            if rule.shape != super::Shape::OptionString {
                continue;
            }
            for entry in &rule.allowed {
                checked += 1;
                match entry.strip_suffix('*') {
                    Some(prefix) => assert!(
                        super::flag_matches(&format!("{prefix}zz"), entry),
                        "{entry} is starred but rejects its own extension"
                    ),
                    None => {
                        assert!(super::flag_matches(entry, entry), "{entry} rejects itself");
                        assert!(
                            !super::flag_matches(&format!("{entry}zz"), entry),
                            "{entry} admits the unrelated flag {entry}zz — star it only if the \
                             value glues on with no delimiter"
                        );
                    }
                }
            }
        }
        assert!(checked > 50, "only {checked} entries seen; the table was not walked");
    }

    /// LUA_INIT is Lua SOURCE unless it starts with `@`, in which case it is a path. One shape
    /// cannot express that, so presence denies — including for the path spelling, which would
    /// otherwise look like an ordinary worktree read.
    #[test]
    fn lua_init_denies_in_both_of_its_forms() {
        assert!(!crate::is_safe_command("LUA_INIT='os.execute(\"id\")' ls"));
        assert!(!crate::is_safe_command("LUA_INIT=@./init.lua ls"));
    }

    /// Julia's load and depot paths supply code, so they follow the executor locus rather than the
    /// read locus: in-worktree allows, /tmp and home do not.
    #[test]
    fn julia_paths_follow_the_executor_locus() {
        assert!(crate::is_safe_command("JULIA_LOAD_PATH=./deps ls"));
        assert!(!crate::is_safe_command("JULIA_LOAD_PATH=/tmp/evil ls"));
        assert!(!crate::is_safe_command("JULIA_DEPOT_PATH=/tmp/evil ls"));
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
