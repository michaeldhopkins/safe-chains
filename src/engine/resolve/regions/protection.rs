//! Which region nodes protect something, whether a workspace root sits above one of them, and what
//! the table can say about a name with no folder to place it in.

use std::sync::LazyLock;

use super::{Frozen, Matcher, REGIONS, Role, absolute_other_home, current_os, secret_node};
use crate::engine::facet::LocalLocus;

/// Whether a role is a PROTECTION (a credential/secret shield, the pinned config, or a
/// write-freeze) rather than an admit — it makes some face stricter than an ordinary worktree.
/// Only protection nodes are matched case-insensitively on a case-insensitive filesystem: folding
/// an ADMIT (`/tmp`, worktree) could admit a case-variant that is a DIFFERENT path on a
/// case-sensitive volume (fail-open), whereas folding a protection only ever denies more.
pub(super) fn role_is_protective(role: &Role) -> bool {
    role.reads_secret
        || role.frozen != Frozen::Nothing
        || role.write_locus > LocalLocus::Worktree
        || role.read_locus > LocalLocus::WorktreeTrusted
}

/// Whether absolute `abs`, inside the workspace `root`, must keep its absolute spelling instead of
/// becoming a root-relative worktree path.
///
/// A root that contains a place protected at a fixed location (`/etc/shadow`,
/// `~/.config/safe-chains.toml`, `/boot/`) would otherwise turn that place, and every directory
/// above it, into ordinary worktree files. So a path at, inside or above such a place stays
/// absolute (a glob counts as its literal directory), and classifies as from any other workspace.
/// A root at or above a home directory (`/`, `/Users`, `$HOME`, and `/private`) keeps EVERY path absolute: the
/// whole home is the protection there, read but never written. A root that sits INSIDE a protected
/// place (`/root/app`) contains none and is unaffected.
///
/// The root comes from the harness as spelled, so paths are compared in a normal form: `.` and
/// `..` collapsed, macOS's `/private` firmlinks folded (a session started in `/etc` reports
/// `/private/etc`), and case folded on macOS. A folded comparison only ever finds more.
pub(crate) fn keeps_absolute(abs: &str, root: &str) -> bool {
    let root = comparable(root);
    let home = std::env::var("HOME").ok().filter(|h| h.starts_with('/'));
    let under = |inner: &str, outer: &str| outer == "/" || inner == outer || inner.strip_prefix(outer).is_some_and(|r| r.starts_with('/'));
    let homes = ["/Users", "/home", "/private"].into_iter().map(str::to_string).chain(home.clone());
    if homes.map(|h| comparable(&h)).any(|h| under(&h, &root)) {
        return true;
    }
    let path = comparable(&literal_dir(abs));
    REGIONS
        .nodes
        .iter()
        .filter(|n| n.applies_here() && role_is_protective(&n.role))
        .filter_map(|n| fixed_place(&n.matcher))
        .any(|(place, subtree)| {
            let place = match (place.strip_prefix('~'), home.as_deref()) {
                (Some(rest), Some(h)) => comparable(&format!("{h}{rest}")),
                (Some(_), None) => return true,
                (None, _) => comparable(&place),
            };
            under(&place, &root) && ((subtree && under(&path, &place)) || under(&place, &path))
        })
}

/// `path` up to its first component holding a glob character: the directory a glob can reach into.
fn literal_dir(path: &str) -> String {
    path.split('/').take_while(|c| !c.contains(['*', '?', '['])).collect::<Vec<_>>().join("/")
}

/// An absolute path in the form `keeps_absolute` compares: lexically collapsed, the
/// `/private/{etc,var,tmp}` firmlinks folded, lowercased on macOS, no trailing slash but `/`.
fn comparable(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    let mut out = format!("/{}", parts.join("/"));
    if current_os() == "macos" {
        out = out.to_ascii_lowercase();
    }
    for firm in ["/etc", "/var", "/tmp"] {
        let private = format!("/private{firm}");
        if out == private || out.starts_with(&format!("{private}/")) {
            out = out["/private".len()..].to_string();
        }
    }
    out
}

/// The literal directory or file a node is anchored at, and whether it covers what is below it
/// (an exact node names one path only), or `None` for an anchorless segment node.
fn fixed_place(matcher: &Matcher) -> Option<(String, bool)> {
    match matcher {
        Matcher::Exact(s) => Some((s.clone(), false)),
        Matcher::Prefix(s) => Some((s.clone(), true)),
        Matcher::StringPrefix(s) => Some((s.rsplit_once('/').map_or_else(String::new, |(dir, _)| dir.to_string()), true)),
        Matcher::Glob(parts) => Some((parts.iter().take_while(|p| !p.contains('*')).cloned().collect::<Vec<_>>().join("/"), true)),
        Matcher::Segment(_) => None,
    }
}

/// Whether any protection, anchored or not, covers absolute `path`: a workspace rooted inside one
/// (`~/.ssh`) is where the user chose to work, but a `~/` spelling is not resolved into it.
pub(crate) fn protection_covers(path: &str) -> bool {
    let path = super::super::locus::canonicalize(path);
    let fold_shields = current_os() == "macos";
    absolute_other_home(&path)
        || REGIONS
            .nodes
            .iter()
            .filter(|n| n.applies_here() && role_is_protective(&n.role))
            .any(|n| n.matcher.specificity(&path, n.fold && fold_shields).is_some())
}

/// Every declared region path at a fixed place that a protection node applying on this platform
/// covers: the witnesses for the root-above-a-protection property.
#[cfg(test)]
pub(crate) fn anchored_protected_paths_here() -> Vec<String> {
    super::declared_region_paths()
        .into_iter()
        .filter(|p| p.starts_with('/') || p.starts_with('~'))
        .filter(|p| {
            REGIONS
                .nodes
                .iter()
                .any(|n| n.applies_here() && role_is_protective(&n.role) && n.matcher.specificity(p, false).is_some())
        })
        .collect()
}

/// Whether `path` falls under any node of the region table, whatever its role, ignoring grants.
///
/// The unknown-folder classifier asks this of a relative path twice, once as written (a segment
/// node such as `.git` or `.ssh` bites at any depth) and once placed under `~`, so a node added to
/// the table later is a sensitive name without anyone extending a list.
pub(crate) fn names_a_region(path: &str) -> bool {
    let path = &super::super::locus::canonicalize(path);
    let fold = current_os() == "macos";
    secret_node(path).is_some()
        || REGIONS
            .nodes
            .iter()
            .any(|n| n.applies_here() && n.matcher.specificity(path, n.fold && fold).is_some())
}

/// A node under `~`, for the unknown-folder classifier, which has a relative path and no folder to
/// put it in: its segments, whether it pins one file, and whether it shields a secret.
pub(crate) struct HomeNode {
    pub segments: Vec<String>,
    pub exact: bool,
    pub secret: bool,
}

/// Every node under `~`. A relative path that starts with the end of one (`LaunchAgents/x` for
/// `~/Library/LaunchAgents/`, `git/config` for `~/.config/git/`) can be that place whichever folder
/// under home the command runs in, and an exact node's file name (`credentials` from
/// `~/.cargo/credentials`) can be that file.
pub(crate) fn home_nodes() -> &'static [HomeNode] {
    static NODES: LazyLock<Vec<HomeNode>> = LazyLock::new(|| {
        REGIONS
            .nodes
            .iter()
            .filter_map(|n| match &n.matcher {
                Matcher::Exact(p) => p.strip_prefix("~/").map(|p| (p, true, n.role.reads_secret)),
                Matcher::Prefix(p) => p.strip_prefix("~/").map(|p| (p, false, n.role.reads_secret)),
                _ => None,
            })
            .map(|(p, exact, secret)| HomeNode {
                segments: p.split('/').filter(|s| !s.is_empty()).map(str::to_string).collect(),
                exact,
                secret,
            })
            .filter(|n| !n.segments.is_empty())
            .collect()
    });
    &NODES
}

/// Whether `path` falls under a node that shields a secret (`.ssh`, `~/.aws/`).
pub(crate) fn names_a_secret(path: &str) -> bool {
    secret_node(&super::super::locus::canonicalize(path)).is_some()
}

#[cfg(test)]
mod names_tests {
    use super::*;

    #[test]
    fn segment_nodes_name_a_relative_path_and_home_nodes_a_home_one() {
        for named in
            [".git/hooks/pre-commit", "a/.ssh/id_rsa", ".envrc", "~/.ssh/config", "~/.config", "~/Library/LaunchAgents/x.plist", "~/bin/ls"]
        {
            assert!(names_a_region(named), "{named}");
        }
        for unnamed in ["src/main.rs", "notes.md", "~/notes.md", "~/projects/app/src/x.rs", "bin/ls"] {
            assert!(!names_a_region(unnamed), "{unnamed}");
        }
    }

    #[test]
    fn home_nodes_carry_their_segments_and_whether_they_shield_a_secret() {
        let find = |p: &[&str]| home_nodes().iter().find(|n| n.segments.iter().map(String::as_str).eq(p.iter().copied()));
        let creds = find(&[".cargo", "credentials"]).expect("~/.cargo/credentials");
        assert!(creds.exact && creds.secret);
        let agents = find(&["Library", "LaunchAgents"]).expect("~/Library/LaunchAgents/");
        assert!(!agents.exact && !agents.secret);
        assert!(home_nodes().iter().all(|n| !n.segments.is_empty() && n.segments.iter().all(|s| !s.is_empty())));
    }
}

#[cfg(test)]
mod tests {
    use super::super::with_os;
    use super::*;

    fn role(read: LocalLocus, write: LocalLocus, reads_secret: bool, frozen: Frozen) -> Role {
        Role { read_locus: read, write_locus: write, rebind_locus: write, reads_secret, frozen }
    }

    #[test]
    fn each_stricter_face_alone_makes_a_role_protective() {
        let w = LocalLocus::Worktree;
        assert!(!role_is_protective(&role(w, w, false, Frozen::Nothing)), "the worktree itself");
        assert!(!role_is_protective(&role(LocalLocus::WorktreeTrusted, w, false, Frozen::Nothing)), "a trusted read at the bound");
        assert!(role_is_protective(&role(w, w, true, Frozen::Nothing)), "a secret alone");
        assert!(role_is_protective(&role(w, w, false, Frozen::Rebind)), "a freeze alone");
        assert!(role_is_protective(&role(w, LocalLocus::WorktreeTrusted, false, Frozen::Nothing)), "a write past the worktree alone");
        assert!(role_is_protective(&role(LocalLocus::Machine, w, false, Frozen::Nothing)), "a read past the trusted rung alone");
    }

    #[test]
    fn a_root_holding_only_admitted_places_still_resolves_relative() {
        assert!(!keeps_absolute("/tmp/x", "/tmp"), "the scratch node admits, it does not protect");
    }

    #[test]
    fn a_glob_node_is_anchored_at_its_literal_directory() {
        assert!(keeps_absolute("/proc/1/environ", "/proc"));
        assert!(keeps_absolute("/proc", "/proc"), "the root itself is above the protected place");
    }

    #[test]
    fn protection_covers_only_protections_and_folds_case_only_on_macos() {
        assert!(!protection_covers("/tmp/x"), "an admit node is not a protection");
        assert!(!protection_covers("/srv/app/src"));
        assert!(protection_covers("/srv/app/.ssh/id_rsa"));
        assert!(with_os("macos", || protection_covers("/srv/app/.SSH/id_rsa")), "one file on a case-insensitive volume");
        assert!(!with_os("linux", || protection_covers("/srv/app/.SSH/id_rsa")), "a different directory on Linux");
    }
}
