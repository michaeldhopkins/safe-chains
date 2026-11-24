/// `/var/folders/<segment>/<segment>/T/<child>`, or the `/private` spelling of that directory.
pub(crate) fn macos_user_temporary(path: &str) -> bool {
    user_temporary(path, "/var/folders/") || user_temporary(path, "/private/var/folders/")
}

fn user_temporary(path: &str, prefix: &str) -> bool {
    let Some(rest) = path.strip_prefix(prefix) else {
        return false;
    };
    let mut parts = rest.split('/');
    let seg = |part: Option<&str>| part.is_some_and(|part| !part.is_empty() && part != "." && part != "..");
    seg(parts.next()) && seg(parts.next()) && parts.next() == Some("T") && parts.next().is_some()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn a_user_temporary_directory_is_exactly_two_segments_then_t(
            first in "[A-Za-z0-9_-]{1,8}",
            second in "[A-Za-z0-9_-]{1,12}",
            rest in "[A-Za-z0-9._-]{1,16}",
            kind in prop::sample::select(vec!["T", "C", "0", "X", "t", "TT", "tmp"]),
            root in prop::sample::select(vec!["/var/folders", "/private/var/folders"]),
        ) {
            let path = format!("{root}/{first}/{second}/{kind}/{rest}");
            let short = format!("{root}/{first}/{kind}/{rest}");
            let deep = format!("{root}/{first}/{second}/zz/{kind}/{rest}");
            let bare = format!("{root}/{first}/{second}/T");
            let dot = format!("{root}/./{second}/T/{rest}");
            let dotdot = format!("{root}/{first}/../T/{rest}");
            let nested = format!("{root}/{first}/{second}/T/{rest}/nested");
            prop_assert_eq!(super::macos_user_temporary(&path), kind == "T");
            prop_assert!(!super::macos_user_temporary(&short));
            prop_assert!(!super::macos_user_temporary(&deep));
            prop_assert!(!super::macos_user_temporary(&bare));
            prop_assert!(!super::macos_user_temporary(&dot));
            prop_assert!(!super::macos_user_temporary(&dotdot));
            prop_assert!(super::macos_user_temporary(&nested));
        }
    }
}
