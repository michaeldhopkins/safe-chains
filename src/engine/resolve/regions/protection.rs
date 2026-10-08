//! Which region nodes protect something, and whether a workspace root sits above one of them.

use super::{Frozen, Matcher, REGIONS, Role, absolute_other_home, current_os};
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
