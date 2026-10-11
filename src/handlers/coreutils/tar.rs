//! `tar` is owned by the ENGINE resolver (`engine::resolve::resolve_tar`), which is
//! authoritative — it models tar's operation (list/create/extract), locus (which files the
//! members and archive touch), and the cumulative `-C` chdir positively, so there is no legacy
//! handler here (the old one was a fail-open flag DENYLIST, fully superseded). This module keeps
//! only tar's DOCS and registry entry; the tests below are engine integration tests for tar.

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] =
    &[crate::handlers::CommandEntry::Custom { cmd: "tar", valid_prefix: Some("tar -tf archive.tar") }];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        tar_list: "tar -tf archive.tar",
        tar_list_verbose: "tar -tvf archive.tar",
        tar_list_gz: "tar -tzf archive.tar.gz",
        tar_list_long: "tar --list --file archive.tar",
        tar_list_bz2: "tar -tjf archive.tar.bz2",
        tar_list_xz: "tar -tJf archive.tar.xz",
        tar_list_separate: "tar -t -f archive.tar",
        tar_list_v_separate: "tar -t -v -f archive.tar",
        tar_old_style_tz: "tar tz",
        tar_old_style_tf: "tar tf archive.tar",
        tar_old_style_tvf: "tar tvf archive.tar",
        tar_old_style_tzf: "tar tzf archive.tar.gz",
        tar_old_style_tjf: "tar tjf archive.tar.bz2",
        // engine-authoritative: creating/appending a WORKTREE archive from WORKTREE members
        // is admitted at write-local. Extraction stays denied (the archive controls the
        // written paths), as do the bare/no-op forms.
        tar_create: "tar -cf archive.tar files/",
        tar_append: "tar -rf archive.tar newfile",
        tar_update: "tar -uf archive.tar newfile",
        tar_bundled_create: "tar -tcf archive.tar",
        tar_old_style_cf: "tar cf archive.tar files/",
        // -C DIR binds the members to DIR (like find {}→path): worktree members still allow.
        tar_change_dir_worktree: "tar -cf out.tar -C ./src main.rs lib.rs",
        tar_change_dir_long: "tar -cf out.tar --directory=./build a.o",
        tar_change_dir_glued: "tar -cf out.tar -C./src main.rs",
    }

    denied! {
        tar_extract: "tar -xf archive.tar",
        // -C into a system dir → the members are system paths → deny (a secret member here;
        // note /etc/passwd|hosts are public-config and archiving THOSE is a legit read).
        tar_change_dir_secret: "tar -cf out.tar -C /etc shadow",
        tar_change_dir_home_key: "tar -cf out.tar -C ~/.ssh id_rsa",
        tar_change_dir_absolute_member: "tar -cf out.tar -C /tmp /etc/shadow",
        // still worst-cased: value-taking options we don't model.
        tar_files_from_denied: "tar -cf out.tar -T /etc/shadow",
        // tar applies -C CUMULATIVELY (chdir per -C): `-C / -C etc` resolves to /etc, so the
        // member reads a system file — must deny (was a false-allow when -C was "last wins").
        tar_cumulative_c_etc: "tar cf out.tar -C / -C etc shadow",
        tar_cumulative_c_stream: "tar cf - -C / -C etc shadow",
        tar_cumulative_c_home_key: "tar cf out.tar -C /home -C victim .ssh/id_rsa",
        tar_cumulative_c_root_key: "tar cf out.tar -C / -C root -C .ssh id_rsa",
        tar_bare: "tar",
        tar_no_list: "tar -f archive.tar",
        tar_bundled_extract: "tar -txf archive.tar",
        tar_old_style_xf: "tar xf archive.tar",
    }

    // Property tests: invariants of `is_safe_command` / `command_verdict` as exercised on tar.
    // The file's test module only uses `is_safe_command` (via `check`), but properties are written
    // against the public projection pair because no other pure function lives in this file.
    proptest::proptest! {
        // Oracle / differential: `is_safe_command` is defined as the projection of
        // `command_verdict(...).is_allowed()`, so the two must agree on allowance for every input.
        #[test]
        fn command_verdict_and_is_safe_command_agree_on_allowance(c in ".*") {
            proptest::prop_assert_eq!(
                crate::is_safe_command(&c),
                crate::command_verdict(&c).is_allowed(),
                "is_safe_command disagreed with command_verdict().is_allowed() for input: `{}`",
                c,
            );
        }

        // Invariant: tar's `-t` (list) operation only reads the archive, so with a benign archive
        // name it never escalates into a write/exec, and the verdict is allowed. (Path-traversal
        // names are excluded on purpose — they're not in scope of the "benign" class.)
        #[test]
        fn tar_list_with_benign_archive_is_allowed(
            name in proptest::string::string_regex("[a-z][a-z0-9]{0,8}").unwrap(),
        ) {
            let line = format!("tar -tf {name}.tar");
            proptest::prop_assert!(
                crate::is_safe_command(&line),
                "tar list with benign archive was denied: `{}`",
                line,
            );
        }

        // Metamorphic: `-v` is a purely display flag for tar, so inserting it into a `-t` form
        // must not change the verdict. We check equality (allowed ⟺ allowed, denied ⟺ denied)
        // so the property holds on every input, not just on safe ones.
        #[test]
        fn tar_list_verdict_is_invariant_under_verbose_flag(
            name in proptest::string::string_regex("[a-z][a-z0-9]{0,8}").unwrap(),
        ) {
            let base = format!("tar -tf {name}.tar");
            let verbose = format!("tar -tvf {name}.tar");
            proptest::prop_assert_eq!(
                crate::is_safe_command(&base),
                crate::is_safe_command(&verbose),
                "-v flipped tar list verdict: base=`{}` verbose=`{}`",
                base,
                verbose,
            );
        }

        // Invariant: tar's `-x` (extract) operation writes files whose paths are taken from the
        // archive itself — that is unmodeled, so any extract form with a benign archive name
        // must be denied. (The deny verdict for `tar -xf <name>.tar` is the canonical case.)
        #[test]
        fn tar_extract_with_benign_archive_is_denied(
            name in proptest::string::string_regex("[a-z][a-z0-9]{0,8}").unwrap(),
        ) {
            let line = format!("tar -xf {name}.tar");
            proptest::prop_assert!(
                !crate::is_safe_command(&line),
                "tar extract with benign archive was allowed: `{}`",
                line,
            );
        }
    }
}
