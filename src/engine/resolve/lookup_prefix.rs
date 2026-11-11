//! macOS lookup prefixes, which name the same file as the path after them.

/// macOS resolves `/.nofollow/p` and `/.resolve/<flags>/p` as `/p`, with lookup flags, so both
/// prefixes name the same file as the path after them.
pub(crate) fn strip_lookup_prefixes(path: &str) -> Option<String> {
    if crate::engine::resolve::regions::current_os() != "macos" {
        return None;
    }
    strip_on_macos(path)
}

fn strip_on_macos(path: &str) -> Option<String> {
    let mut rest = path;
    loop {
        let trimmed = rest.trim_start_matches('/');
        if !rest.starts_with('/') {
            break;
        }
        let (first, after) = trimmed.split_once('/').unwrap_or((trimmed, ""));
        if first.eq_ignore_ascii_case(".nofollow") {
            rest = &trimmed[first.len()..];
        } else if first.eq_ignore_ascii_case(".resolve") {
            let flags_end = after.find('/').unwrap_or(after.len());
            rest = &after[flags_end..];
        } else {
            break;
        }
        if rest.is_empty() {
            rest = "/";
            break;
        }
    }
    (rest.len() != path.len()).then(|| rest.to_string())
}

#[cfg(test)]
mod tests {
    use super::strip_on_macos;
    use proptest::prelude::*;

    fn prefix() -> impl Strategy<Value = String> {
        prop_oneof![
            "/[.][nN][oO][fF][oO][lL][lL][oO][wW]".prop_map(String::from),
            "[0-9]{1,3}".prop_map(|flags| format!("/.resolve/{flags}")),
            "/[.][rR][eE][sS][oO][lL][vV][eE]/[a-z0-9]{1,3}".prop_map(String::from),
        ]
    }

    proptest! {
        /// Any run of lookup prefixes in front of an ordinary absolute path strips back to that path.
        #[test]
        fn a_run_of_lookup_prefixes_strips_to_the_path_after_it(
            prefixes in prop::collection::vec(prefix(), 1..4),
            path in "(/[a-zA-Z_][a-zA-Z0-9_.]{0,6}){1,4}",
        ) {
            let spelled = format!("{}{path}", prefixes.concat());
            prop_assert_eq!(strip_on_macos(&spelled), Some(path));
        }

        /// A path whose first component is not a lookup prefix is left alone.
        #[test]
        fn a_path_without_a_lookup_prefix_is_left_alone(path in "(/[a-zA-Z_][a-zA-Z0-9_]{0,6}){1,4}") {
            prop_assert_eq!(strip_on_macos(&path), None);
        }
    }
}
