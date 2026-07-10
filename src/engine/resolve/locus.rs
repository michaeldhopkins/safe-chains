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

use super::regions::classify_region;
use crate::engine::facet::LocalLocus;

/// The locus a READ of `path` reaches (the read face of its region role).
pub(crate) fn read_locus(path: &str) -> LocalLocus {
    let resolved = crate::pathctx::resolve(path);
    if is_unpinnable(&resolved) {
        return LocalLocus::Machine;
    }
    classify_region(&resolved).read_locus
}

/// The locus a WRITE of `path` reaches (the write face of its region role). The conservative
/// face — a system path stays at `machine` even where its read face is lower.
pub(crate) fn write_locus(path: &str) -> LocalLocus {
    let resolved = crate::pathctx::resolve(path);
    if is_unpinnable(&resolved) {
        return LocalLocus::Machine;
    }
    classify_region(&resolved).write_locus
}

/// The default (write) face — kept as `classify_locus` so every existing write-side call site
/// reads unchanged.
pub(crate) fn classify_locus(path: &str) -> LocalLocus {
    write_locus(path)
}

/// Whether reading `path` extracts a secret (a known credential store). Consumed by the
/// secret-facet enrichment (follow-on); today the read face already denies these by locus.
#[allow(dead_code)]
pub(crate) fn reads_secret(path: &str) -> bool {
    let resolved = crate::pathctx::resolve(path);
    !is_unpinnable(&resolved) && classify_region(&resolved).reads_secret
}

/// Fail-closed guard (§0): a `$VAR` expansion or a `..` escape could name anything, so no
/// positive region classification is sound — worst-case to `machine`.
fn is_unpinnable(path: &str) -> bool {
    path.contains('$') || is_parent_escape(path)
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
    fn write_face_reproduces_pre_hp20_rungs() {
        assert_eq!(write_locus("/dev/null"), LocalLocus::Process);
        assert_eq!(write_locus("/dev/rdisk0"), LocalLocus::Device);
        assert_eq!(write_locus("/tmp/scratch"), LocalLocus::Temp);
        assert_eq!(write_locus("notes.md"), LocalLocus::Worktree);
        assert_eq!(write_locus("src/engine/mod.rs"), LocalLocus::Worktree);
        assert_eq!(write_locus(".git/config"), LocalLocus::WorktreeTrusted);
        assert_eq!(write_locus("~"), LocalLocus::User);
        assert_eq!(write_locus("~/notes"), LocalLocus::User);
        assert_eq!(write_locus("/etc/hosts"), LocalLocus::Machine, "writing system config → machine");
        assert_eq!(write_locus("/usr/local/bin/x"), LocalLocus::Machine);
        assert_eq!(write_locus("~bob/.ssh/id_rsa"), LocalLocus::Machine, "another user's home");
    }

    #[test]
    fn read_face_lowers_recognized_public_paths_only() {
        // world-readable system paths become readable (the HP-20 point). Use cross-platform
        // nodes; /proc etc. are linux-scoped and asserted under cfg(linux) below.
        assert_eq!(read_locus("/etc/hosts"), LocalLocus::WorktreeTrusted);
        assert_eq!(read_locus("/usr/bin/python3"), LocalLocus::WorktreeTrusted);
        #[cfg(target_os = "linux")]
        assert_eq!(read_locus("/proc/cpuinfo"), LocalLocus::WorktreeTrusted);
        // …but secrets, home, and unknown paths do NOT lower
        assert_eq!(read_locus("/etc/shadow"), LocalLocus::Machine);
        assert_eq!(read_locus("~/.ssh/id_rsa"), LocalLocus::Machine);
        assert_eq!(read_locus("~/notes"), LocalLocus::User);
        assert_eq!(read_locus("/some/unmapped/thing"), LocalLocus::Machine);
        // worktree reads unchanged
        assert_eq!(read_locus("notes.md"), LocalLocus::Worktree);
    }

    #[test]
    fn credential_stores_read_secret() {
        assert!(reads_secret("~/.ssh/id_rsa"));
        assert!(reads_secret("/etc/shadow") || cfg!(target_os = "macos")); // shadow is linux-scoped
        assert!(reads_secret("~/.aws/credentials"));
        assert!(!reads_secret("/etc/hosts"));
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
