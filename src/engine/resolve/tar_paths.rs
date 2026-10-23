//! How tar places the paths it names: a member or archive against the active `-C` folder, and
//! an archive name GNU tar opens on another machine.

/// A tar member/archive path resolved against an active `-C` directory: `DIR/path` for a
/// relative path, or `path` unchanged when there is no `-C` or the path is absolute (an
/// absolute member ignores `-C`).
pub(crate) fn tar_bound(dir: Option<&str>, path: &str) -> String {
    match dir {
        Some(d) if !path.starts_with('/') && !path.starts_with('~') && !path.starts_with('-') => {
            format!("{}/{}", d.trim_end_matches('/'), path)
        }
        _ => path.to_string(),
    }
}

/// GNU tar reads an archive name with a colon before its first slash as `[user@]host:path` and
/// opens it on that host over rsh or ssh.
pub(super) fn names_a_remote_archive(archive: &str) -> bool {
    archive.split('/').next().is_some_and(|head| head.contains(':'))
}

#[cfg(test)]
mod tests {
    use super::{names_a_remote_archive, tar_bound};

    proptest::proptest! {
        /// A relative path lands below the `-C` folder, and an absolute or home path ignores it.
        #[test]
        fn a_relative_path_lands_below_the_change_directory_and_an_anchored_one_ignores_it(
            dir in proptest::string::string_regex("[a-z]{1,6}(/[a-z]{1,6}){0,2}/?").unwrap(),
            path in proptest::string::string_regex("[a-z]{1,6}(/[a-z]{1,6}){0,2}").unwrap(),
            anchor in proptest::sample::select(vec!["/", "~/"]),
        ) {
            proptest::prop_assert_eq!(tar_bound(Some(&dir), &path), format!("{}/{path}", dir.trim_end_matches('/')));
            let anchored = format!("{anchor}{path}");
            proptest::prop_assert_eq!(tar_bound(Some(&dir), &anchored), anchored.clone());
            proptest::prop_assert_eq!(tar_bound(None, &path), path);
        }

        /// A colon before the first slash names a host; one below a folder is part of a file name.
        #[test]
        fn only_a_colon_before_the_first_slash_names_a_host(
            host in proptest::string::string_regex("([a-z]{1,4}@)?[a-z][a-z0-9.]{0,8}").unwrap(),
            dir in proptest::string::string_regex("[a-z.]{1,6}").unwrap(),
            name in proptest::string::string_regex("[a-z]{1,6}(/[a-z]{1,6}){0,2}").unwrap(),
        ) {
            let remote = format!("{host}:{name}.tar");
            proptest::prop_assert!(names_a_remote_archive(&remote), "{}", remote);
            let local = format!("{dir}/{remote}");
            proptest::prop_assert!(!names_a_remote_archive(&local), "{}", local);
        }
    }
}
