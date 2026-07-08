//! The profile resolver — turning a parsed command into its behavior profile
//! (annex `behavioral-taxonomy-engine`). Dormant: nothing here is wired into the
//! live classifier yet.
//!
//! This module starts with argument classification — the reusable predicates that
//! read a facet off an argument value. `classify_locus` refines the existing
//! `is_safe_write_target` branch order (`src/cst/check.rs`, a 2-bucket boolean) into
//! the full [`LocalLocus`] ladder (v1.4 §2.2).

use super::facet::LocalLocus;

/// The filesystem rung a path reaches (v1.4 §2.2). A value that cannot be pinned —
/// a `$VAR` expansion or a `..` parent-escape — takes the worst-case fs rung
/// (`machine`), matching the allowlist floor `is_safe_write_target` already enforces
/// by denying such targets.
///
/// The same classifier serves reads and writes; the *level* draws the line
/// (`read-local` admits `<= user`, `write-local` admits `<= worktree`), which is the
/// refinement the facet model buys over the old single boolean.
pub fn classify_locus(path: &str) -> LocalLocus {
    // Unpinnable FIRST (§0 fail-closed): a `$VAR` expansion or a `..` escape could name
    // anything, so no positive (lower) classification is sound — not even a `/tmp/`
    // prefix, since `/tmp/$X` can expand through `..` to anywhere. Worst-case to
    // `machine` (the top fs rung; raw devices need an explicit /dev/ match).
    if path.contains('$') || is_parent_escape(path) {
        return LocalLocus::Machine;
    }
    // Standard streams — no real filesystem is touched.
    if matches!(path, "/dev/null" | "/dev/stdout" | "/dev/stderr" | "/dev/tty")
        || path.starts_with("/dev/fd/")
    {
        return LocalLocus::Process;
    }
    // Raw block/char devices — beneath the filesystem (dd of=/dev/rdisk0, mount).
    if is_block_device(path) {
        return LocalLocus::Device;
    }
    // Temp — process-scoped scratch.
    if path.starts_with("/tmp/")
        || path.starts_with("/private/tmp/")
        || path.starts_with("/var/tmp/")
    {
        return LocalLocus::Temp;
    }
    // Files another tool auto-executes or trusts (.git/ hooks & config, .envrc).
    if has_trusted_segment(path) {
        return LocalLocus::WorktreeTrusted;
    }
    // The user's own home / keychain scope.
    if path.starts_with('~') {
        return LocalLocus::User;
    }
    // Any other absolute path — /etc, /usr, services, another user's home.
    if path.starts_with('/') {
        return LocalLocus::Machine;
    }
    // A plain relative path inside the working tree.
    LocalLocus::Worktree
}

/// A raw block/char device node (not a standard stream — those are handled first).
/// The prefix list is curated and conservative; unmatched `/dev/*` nodes fall through
/// to the general `machine` rule.
fn is_block_device(path: &str) -> bool {
    const DEVICE_PREFIXES: &[&str] = &[
        "/dev/disk",
        "/dev/rdisk",
        "/dev/sd",
        "/dev/nvme",
        "/dev/hd",
        "/dev/vd",
        "/dev/mmcblk",
        "/dev/loop",
    ];
    DEVICE_PREFIXES.iter().any(|p| path.starts_with(p))
}

fn is_parent_escape(path: &str) -> bool {
    path == ".." || path.starts_with("../") || path.contains("/../") || path.ends_with("/..")
}

/// Whether any path segment is a directory a tool auto-executes or trusts. Matches
/// today's `is_safe_write_target` (`.git`, `.envrc`); CI-config trees (`.github/`,
/// `.gitlab-ci.yml`) are a future refinement of this set.
fn has_trusted_segment(path: &str) -> bool {
    path.split('/').any(|seg| seg == ".git" || seg == ".envrc")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_streams_are_process_scoped() {
        assert_eq!(classify_locus("/dev/null"), LocalLocus::Process);
        assert_eq!(classify_locus("/dev/stdout"), LocalLocus::Process);
        assert_eq!(classify_locus("/dev/fd/3"), LocalLocus::Process);
    }

    #[test]
    fn raw_devices_are_device_rung() {
        assert_eq!(classify_locus("/dev/rdisk0"), LocalLocus::Device);
        assert_eq!(classify_locus("/dev/sda1"), LocalLocus::Device);
        assert_eq!(classify_locus("/dev/nvme0n1"), LocalLocus::Device);
    }

    #[test]
    fn temp_paths_are_temp() {
        assert_eq!(classify_locus("/tmp/scratch"), LocalLocus::Temp);
        assert_eq!(classify_locus("/private/tmp/x"), LocalLocus::Temp);
        assert_eq!(classify_locus("/var/tmp/y"), LocalLocus::Temp);
    }

    #[test]
    fn plain_relative_paths_are_worktree() {
        assert_eq!(classify_locus("notes.md"), LocalLocus::Worktree);
        assert_eq!(classify_locus("src/engine/mod.rs"), LocalLocus::Worktree);
        assert_eq!(classify_locus("build/out"), LocalLocus::Worktree);
    }

    #[test]
    fn trusted_dotdirs_are_worktree_trusted() {
        assert_eq!(classify_locus(".git/hooks/pre-commit"), LocalLocus::WorktreeTrusted);
        assert_eq!(classify_locus(".git/config"), LocalLocus::WorktreeTrusted);
        assert_eq!(classify_locus(".envrc"), LocalLocus::WorktreeTrusted);
        assert_eq!(classify_locus("nested/.git/x"), LocalLocus::WorktreeTrusted);
    }

    #[test]
    fn home_paths_are_user() {
        assert_eq!(classify_locus("~/.ssh/id_rsa"), LocalLocus::User);
        assert_eq!(classify_locus("~/.config/foo"), LocalLocus::User);
        assert_eq!(classify_locus("~"), LocalLocus::User);
    }

    #[test]
    fn other_absolute_paths_are_machine() {
        assert_eq!(classify_locus("/etc/hosts"), LocalLocus::Machine);
        assert_eq!(classify_locus("/usr/local/bin/x"), LocalLocus::Machine);
        assert_eq!(classify_locus("/Users/someone/notes"), LocalLocus::Machine);
    }

    #[test]
    fn unresolvable_paths_worst_case_to_machine() {
        assert_eq!(classify_locus("$HOME/.ssh/id_rsa"), LocalLocus::Machine);
        assert_eq!(classify_locus("$OUT/file"), LocalLocus::Machine);
        assert_eq!(classify_locus("../secret"), LocalLocus::Machine);
        assert_eq!(classify_locus("a/../../etc/passwd"), LocalLocus::Machine);
        assert_eq!(classify_locus("dir/.."), LocalLocus::Machine);
    }

    #[test]
    fn an_unpinnable_marker_dominates_every_otherwise_safe_prefix() {
        // conservative: an unpinnable segment can't be trusted, even under a safe prefix
        assert_eq!(classify_locus("build/$ARTIFACT"), LocalLocus::Machine);
        assert_eq!(classify_locus("/tmp/$X"), LocalLocus::Machine, "$ beats the /tmp prefix");
        assert_eq!(classify_locus("/tmp/a/../../etc/passwd"), LocalLocus::Machine, ".. escapes /tmp");
        assert_eq!(classify_locus("/dev/null$"), LocalLocus::Machine, "$ beats /dev/null");
    }

    use proptest::prelude::*;

    proptest! {
        /// Fail-closed (§0): a `$` anywhere forces the worst rung, whatever the rest
        /// looks like — the classifier can never be talked below `machine` by a
        /// safe-looking prefix wrapped around an unpinnable expansion.
        #[test]
        fn a_dollar_anywhere_forces_machine(s in ".{0,30}") {
            prop_assert_eq!(classify_locus(&format!("{s}$")), LocalLocus::Machine);
        }

        /// Fail-closed: a `..` parent-escape forces the worst rung.
        #[test]
        fn a_parent_escape_forces_machine(s in "[a-zA-Z0-9/_]{0,20}") {
            prop_assert_eq!(classify_locus(&format!("{s}/../x")), LocalLocus::Machine);
            prop_assert_eq!(classify_locus(&format!("../{s}")), LocalLocus::Machine);
        }
    }
}
