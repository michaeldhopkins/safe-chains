//! The `option-string` shape: an interpreter flag string judged token by token.

use super::{PathFlagEntry, PathRole, Sep, data_path_verdict, exec_path_verdict};
use crate::verdict::{SafetyLevel, Verdict};

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
pub(super) fn option_string_verdict(value: &str, allowed: &[String], path_flags: &[PathFlagEntry]) -> Verdict {
    let raw: Vec<&str> = value.split_whitespace().collect();
    let mut verdict = Verdict::Allowed(SafetyLevel::Inert);
    let mut i = 0;
    while i < raw.len() {
        // A declared path flag is judged on its VALUE by the ordinary locus rules, so an in-worktree
        // target allows and an out-of-worktree one denies — the same answer the command-line
        // spelling of that path would get. Checked before `allowed` so a flag cannot be listed in
        // both and reach the ungated branch.
        if let Some((role, path, used)) = path_flag_match(raw[i], raw.get(i + 1).copied(), path_flags) {
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
        let unit = if raw[i].len() <= 2 && raw[i].starts_with('-') && i + 1 < raw.len() && !raw[i + 1].starts_with('-') {
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
fn path_flag_match<'a>(tok: &'a str, next: Option<&'a str>, path_flags: &[PathFlagEntry]) -> Option<(PathRole, &'a str, usize)> {
    path_flags.iter().find_map(|pf| match pf.sep {
        Sep::Equals => tok.split_once('=').filter(|(f, _)| *f == pf.flag).map(|(_, v)| (pf.role, v, 1)),
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
pub(super) fn flag_matches(token: &str, entry: &str) -> bool {
    match entry.strip_suffix('*') {
        Some(prefix) => token.starts_with(prefix),
        None => token == entry || token.strip_prefix(entry).is_some_and(|r| r.starts_with('=')),
    }
}
