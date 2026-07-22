//! Filesystem locus classification: which rung of the `LocalLocus` ladder a path argument
//! reaches (v1.4 §2.2). The path knowledge itself lives as DATA in `regions/default.toml`
//! (HP-20) — this module is the seam: it resolves the path against the harness cwd/root
//! (HP-19), applies the fail-closed `$VAR`/`..` guard (§0), then reads the region's role
//! through the operation-appropriate FACE.
//!
//! - `read_locus`  — the face a READ reaches. A recognized world-readable system path
//!   (`/etc/hosts`, `/proc/cpuinfo`) projects DOWN to a rung read-local admits, so the read
//!   passes; a secret store or unknown path stays at `machine`.
//! - `write_locus` — the face a WRITE reaches. System paths stay at `machine` (denied); it
//!   reproduces the pre-HP-20 single-locus behavior, so existing write call sites are
//!   unchanged. `classify_locus` is its alias (the conservative default face).

use std::borrow::Cow;

use super::regions::{classify_region, is_hidden_peer};
use crate::engine::facet::LocalLocus;

/// The locus a READ of `path` reaches (the read face of its region role).
pub(crate) fn read_locus(path: &str) -> LocalLocus {
    face(path, false)
}

/// The locus a WRITE of `path` reaches (the write face of its region role). The conservative
/// face — a system path stays at `machine` even where its read face is lower.
pub(crate) fn write_locus(path: &str) -> LocalLocus {
    face(path, true)
}

fn face(path: &str, want_write: bool) -> LocalLocus {
    // Scheme-aware: a URL is not an ordinary local path. `file:` names a LOCAL file, so classify
    // the path it points at (`file:///etc/shadow` denies like reading /etc/shadow). Any other
    // scheme (`http://`, `s3://`, `ssh://`, …) is a NETWORK endpoint — not a local filesystem
    // operation — so it admits here (the command's own handler gates the network) and a URL's
    // `..` is never misread as a filesystem escape. This is the one place the notion of a URL
    // lives; individual command handlers no longer special-case `file:`.
    if let Some(local) = file_url_local(path) {
        return classify_local(local, want_write);
    }
    if is_network_url(path) {
        // A URL consumed as network I/O admits at worktree — its handler gates the network, and a
        // URL's own `..` (`https://host/a/../b`) is a path segment, not a filesystem escape. But a
        // GENERIC read/write command (`cat`, `cp`, a redirect) treats `scheme://../../x` as a
        // LITERAL local path and the OS walks the `..`, climbing out of the workspace. So admit
        // only when the URL, read as a path, does NOT net-escape cwd (and carries no `$`/cmdsub).
        if path.contains('$') || path.contains("__SAFE_CHAINS_CMDSUB__") || url_escapes_cwd(path) {
            return LocalLocus::Machine;
        }
        return LocalLocus::Worktree;
    }
    classify_local(path, want_write)
}

/// Whether a scheme-URL string, read as a LOCAL filesystem path (how a generic reader like `cat`
/// treats it), would climb ABOVE cwd. The scheme label (`s3:`) and each normal segment are one
/// level down; each `..` one up. If depth ever goes negative, the `..`s escape the workspace
/// (`s3://../../x`) and it must not admit as a network endpoint. A real URL whose `..` stay
/// within their own path (`https://host/a/../b`) never goes negative — safe to admit.
fn url_escapes_cwd(url: &str) -> bool {
    let mut depth: i32 = 0;
    for seg in url.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                depth -= 1;
                if depth < 0 {
                    return true;
                }
            }
            _ => depth += 1,
        }
    }
    false
}

fn classify_local(path: &str, want_write: bool) -> LocalLocus {
    // A bound `for`-loop variable expands to its list's representative item first (its read or
    // write representative), so `$f` inherits the list's locus; then the ambient cwd/root.
    let expanded = crate::pathctx::expand_vars(path, want_write);
    let resolved = crate::pathctx::resolve(&expanded);
    let canonical = canonicalize(&resolved);
    if is_unpinnable(&canonical) {
        return LocalLocus::Machine;
    }
    let role = classify_region(&canonical);
    if want_write { role.write_locus } else { role.read_locus }
}

/// Normalize path SPELLINGS that name the same file so the region model — chiefly the
/// exact-match config pin and the grant/shield lookups, which compare by string — can't be
/// dodged. Collapses `//` and `/.`-segments and rewrites an absolute `$HOME` prefix to `~`, so
/// `/Users/me/.config/safe-chains.toml`, `~/.config/./safe-chains.toml`, and `~/.config//…`
/// all reduce to the canonical `~/.config/safe-chains.toml`. `..` is left in place on purpose —
/// `is_unpinnable` rejects it (a normalized `..` would silently defeat that guard).
fn canonicalize(path: &str) -> Cow<'_, str> {
    let home = std::env::var("HOME").ok();
    let home_abs = home
        .as_deref()
        .filter(|h| !h.is_empty() && path.strip_prefix(*h).is_some_and(|r| r.is_empty() || r.starts_with('/')));
    let dotty = path.contains("//") || path.contains("/./") || path.ends_with("/.");
    if home_abs.is_none() && !dotty {
        return Cow::Borrowed(path);
    }
    let tilded = match home_abs {
        Some(h) if path.len() == h.len() => "~".to_string(),
        Some(h) => format!("~{}", &path[h.len()..]),
        None => path.to_string(),
    };
    if !(tilded.contains("//") || tilded.contains("/./") || tilded.ends_with("/.")) {
        return Cow::Owned(tilded);
    }
    let absolute = tilded.starts_with('/');
    let joined = tilded
        .split('/')
        .filter(|seg| !seg.is_empty() && *seg != ".")
        .collect::<Vec<_>>()
        .join("/");
    Cow::Owned(if absolute { format!("/{joined}") } else { joined })
}

/// The LOCAL path a `file:` URL names, or `None` when `path` is not a `file:` URL. Schemes are
/// case-insensitive; handles `file:///p`, `file://host/p`, and `file:/p`.
fn file_url_local(path: &str) -> Option<&str> {
    if path.len() < 5 || !path.as_bytes()[..5].eq_ignore_ascii_case(b"file:") {
        return None;
    }
    let rest = &path[5..];
    Some(rest.strip_prefix("//").map_or(rest, |authority| {
        authority.find('/').map_or("", |i| &authority[i..])
    }))
}

/// Whether `path` is a network URL: a `scheme://…` whose scheme is well-formed (a letter, then
/// letters / digits / `+` / `-` / `.`). A local path that merely contains `://` is not a URL.
fn is_network_url(path: &str) -> bool {
    let Some(idx) = path.find("://") else {
        return false;
    };
    let scheme = &path[..idx];
    scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
}

/// The default (write) face — kept as `classify_locus` so every existing write-side call site
/// reads unchanged.
pub(crate) fn classify_locus(path: &str) -> LocalLocus {
    write_locus(path)
}

/// Whether reading `path` extracts a secret (a known credential store). Consumed by the
/// secret-facet enrichment (follow-on); also drives the overreach nudge's credential-store wording.
pub(crate) fn reads_secret(path: &str) -> bool {
    let expanded = crate::pathctx::expand_vars(path, false);
    let resolved = crate::pathctx::resolve(&expanded);
    !is_unpinnable(&resolved) && classify_region(&resolved).reads_secret
}

/// Whether `path` reaches a HIDDEN file inside a co-located peer project — a would-be-`adjacent`
/// path frozen only by the dot-shield. Drives the overreach nudge so it can explain the shield
/// instead of the generic "outside the working directory". Mirrors `classify_local`'s front-end so
/// it sees the same `~`-form path `adjacent_role` classifies.
pub(crate) fn hidden_peer_reach(path: &str) -> bool {
    let expanded = crate::pathctx::expand_vars(path, false);
    let resolved = crate::pathctx::resolve(&expanded);
    let canonical = canonicalize(&resolved);
    is_hidden_peer(&canonical)
}

/// Fail-closed guard (§0): a `$VAR` expansion, a `..` escape, or a COMMAND-substitution result
/// (`$(…)` / backticks, which the CST evaluates to the `__SAFE_CHAINS_CMDSUB__` placeholder)
/// could name ANYTHING, so no positive region classification is sound — worst-case to `machine`.
/// Without the substitution case, `rm $(echo /)` classifies the placeholder as a worktree path
/// and auto-approves `rm -rf /`. (Process substitution is a pipe whose inner command is checked
/// separately, so its distinct placeholder is NOT worst-cased here.)
pub(crate) fn is_unpinnable(path: &str) -> bool {
    path.contains('$') || path.contains("__SAFE_CHAINS_CMDSUB__") || is_parent_escape(path)
}

fn is_parent_escape(path: &str) -> bool {
    path == ".." || path.starts_with("../") || path.contains("/../") || path.ends_with("/..")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::facet::LocalLocus;

    // The detailed path→role table is tested in `regions.rs` and end-to-end in the HP-20
    // scenario suite. Here we pin the SEAM: the fail-closed guard, the read/write asymmetry,
    // and that the write face reproduces the pre-HP-20 rungs.

    #[test]
    fn the_unpinnable_guard_worst_cases_both_faces() {
        for p in ["$HOME/.ssh/id_rsa", "$OUT/file", "../secret", "a/../../etc/passwd", "dir/.."] {
            assert_eq!(read_locus(p), LocalLocus::Machine, "read {p}");
            assert_eq!(write_locus(p), LocalLocus::Machine, "write {p}");
        }
    }

    #[test]
    fn write_face_admits_only_workspace_temp_and_streams() {
        assert_eq!(write_locus("/dev/null"), LocalLocus::Process);
        assert_eq!(write_locus("/tmp/scratch"), LocalLocus::Temp);
        assert_eq!(write_locus("notes.md"), LocalLocus::Worktree);
        assert_eq!(write_locus("src/engine/mod.rs"), LocalLocus::Worktree);
        assert_eq!(write_locus(".git/config"), LocalLocus::WorktreeTrusted, "in-project but write-frozen");
        // the retreat: everything outside the workspace denies — home, system, other users, devices
        assert_eq!(write_locus("~"), LocalLocus::Machine);
        assert_eq!(write_locus("~/notes"), LocalLocus::Machine);
        assert_eq!(write_locus("/etc/hosts"), LocalLocus::Machine);
        assert_eq!(write_locus("/usr/local/bin/x"), LocalLocus::Machine);
        assert_eq!(write_locus("/dev/rdisk0"), LocalLocus::Machine);
        assert_eq!(write_locus("~bob/.ssh/id_rsa"), LocalLocus::Machine, "another user's home");
    }

    #[test]
    fn read_face_admits_only_the_workspace_not_system_paths() {
        // the retreat: system/public paths are NO LONGER admitted — they deny (the harness then
        // prompts, or the user adds a read grant). We stopped modeling the filesystem.
        assert_eq!(read_locus("/etc/hosts"), LocalLocus::Machine);
        assert_eq!(read_locus("/usr/bin/python3"), LocalLocus::Machine);
        assert_eq!(read_locus("/etc/shadow"), LocalLocus::Machine);
        assert_eq!(read_locus("~/.ssh/id_rsa"), LocalLocus::Machine);
        assert_eq!(read_locus("~/notes"), LocalLocus::Machine);
        assert_eq!(read_locus("/some/unmapped/thing"), LocalLocus::Machine);
        // only the workspace and /tmp read
        assert_eq!(read_locus("notes.md"), LocalLocus::Worktree);
        assert_eq!(read_locus("/tmp/x"), LocalLocus::Temp);
    }

    #[test]
    fn file_urls_classify_the_local_path_they_name() {
        // every `file:` form, any case, resolves to the underlying local path
        for p in [
            "file:///etc/shadow",
            "file://localhost/etc/shadow",
            "file:/etc/shadow",
            "FILE:///etc/shadow",
            "File:///etc/shadow",
        ] {
            assert_eq!(read_locus(p), LocalLocus::Machine, "read {p}");
            assert_eq!(write_locus(p), LocalLocus::Machine, "write {p}");
        }
        // a file: URL classifies the local path it names — a system path denies both faces now
        assert_eq!(read_locus("file:///etc/hosts"), LocalLocus::Machine);
        assert_eq!(write_locus("file:///etc/hosts"), LocalLocus::Machine);
        // file: to a worktree-relative path stays worktree
        assert_eq!(read_locus("file:notes.txt"), LocalLocus::Worktree);
        // a `..` inside a file: URL is still a filesystem escape
        assert_eq!(read_locus("file://../../etc/shadow"), LocalLocus::Machine);
    }

    #[test]
    fn network_urls_are_not_local_operations() {
        // a network scheme admits (the network is the command handler's job), and a URL's `..`
        // is NOT a filesystem escape — so no over-deny.
        for p in ["http://example.com/a", "https://x/a/../b", "ftp://h/f", "s3://bucket/key", "ssh://h/p"] {
            assert_eq!(read_locus(p), LocalLocus::Worktree, "read {p}");
            assert_eq!(write_locus(p), LocalLocus::Worktree, "write {p}");
        }
        // a local path that merely contains `://` is NOT a URL
        assert_eq!(write_locus("/tmp/weird://name"), LocalLocus::Temp);
        assert_eq!(read_locus("./a:b"), LocalLocus::Worktree);
    }

    #[test]
    fn a_scheme_url_that_net_escapes_cwd_is_not_admitted() {
        // a real URL whose `..` stay within its own path still admits (no over-deny)
        assert_eq!(read_locus("https://x/a/../b"), LocalLocus::Worktree);
        assert_eq!(read_locus("s3://bucket/../key"), LocalLocus::Worktree);
        // but a `scheme://../../x` climbs above cwd when read as a local path → machine
        for p in ["s3://../../secret.txt", "gopher://../../etc/passwd", "s3://a/../../../etc/x"] {
            assert_eq!(read_locus(p), LocalLocus::Machine, "read {p}");
            assert_eq!(write_locus(p), LocalLocus::Machine, "write {p}");
        }
        // a `$` in a URL still worst-cases (an unpinnable value hiding as a URL)
        assert_eq!(read_locus("s3://$SECRET/x"), LocalLocus::Machine);
    }

    #[test]
    fn canonicalize_folds_equivalent_spellings() {
        // `//` and `/.` segments collapse so an exact-match region node can't be dodged
        assert_eq!(canonicalize("~/.config//safe-chains.toml"), "~/.config/safe-chains.toml");
        assert_eq!(canonicalize("~/.config/./safe-chains.toml"), "~/.config/safe-chains.toml");
        assert_eq!(canonicalize("/a//b/./c"), "/a/b/c");
        // `..` is left in place — the unpinnable guard rejects it (a folded `..` would defeat it)
        assert_eq!(canonicalize("~/a/../b"), "~/a/../b");
        // a clean path is returned untouched (borrowed)
        assert_eq!(canonicalize("~/.config/safe-chains.toml"), "~/.config/safe-chains.toml");
        // an absolute `$HOME` prefix rewrites to `~` so it hits the same node as the tilde form
        if let Some(home) = std::env::var("HOME").ok().filter(|h| h.starts_with('/')) {
            assert_eq!(canonicalize(&format!("{home}/.config/safe-chains.toml")), "~/.config/safe-chains.toml");
        }
    }

    #[test]
    fn credential_stores_read_secret() {
        assert!(reads_secret("~/.ssh/id_rsa"));
        assert!(reads_secret("~/.aws/credentials"));
        assert!(reads_secret("~/.gnupg/secring.gpg"));
        assert!(!reads_secret("/etc/hosts")); // denied, but not a credential store
        assert!(!reads_secret("notes.md"));
    }

    use proptest::prelude::*;
    proptest! {
        #[test]
        fn a_dollar_anywhere_forces_machine(s in ".{0,30}") {
            prop_assert_eq!(read_locus(&format!("{s}$")), LocalLocus::Machine);
            prop_assert_eq!(write_locus(&format!("{s}$")), LocalLocus::Machine);
        }

        #[test]
        fn a_parent_escape_forces_machine(s in "[a-zA-Z0-9/_]{0,20}") {
            prop_assert_eq!(write_locus(&format!("{s}/../x")), LocalLocus::Machine);
            prop_assert_eq!(write_locus(&format!("../{s}")), LocalLocus::Machine);
        }
    }
}
