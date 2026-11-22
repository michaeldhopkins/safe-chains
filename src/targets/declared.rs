use super::UNKNOWN_WORKDIR;

/// The directory a command runs in when the call can name one (`Cwd`, `dir_path`, `directory`)
/// beside the cwd the harness reports. A call that names none runs in the reported folder; one that
/// names an absolute folder runs there. Anything else (a relative or empty name) is a directory the
/// hook cannot place, so the command is judged in the unknown folder.
pub(crate) fn declared_cwd(reported: Option<String>, declared: Option<&str>) -> Option<String> {
    match declared {
        None => reported,
        Some(dir) if dir.starts_with('/') => Some(dir.to_string()),
        Some(_) => Some(UNKNOWN_WORKDIR.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest::proptest! {
        #[test]
        fn a_named_directory_is_used_only_when_absolute_and_otherwise_unknown(dir in ".{0,24}", reported in "/[a-z]{1,8}") {
            let got = declared_cwd(Some(reported.clone()), Some(&dir));
            let expected = if dir.starts_with('/') { dir.clone() } else { UNKNOWN_WORKDIR.to_string() };
            proptest::prop_assert_eq!(got, Some(expected));
            proptest::prop_assert_eq!(declared_cwd(Some(reported.clone()), None), Some(reported));
        }
    }
}
