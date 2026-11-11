//! Telling a URL from a filesystem path.

/// The LOCAL path a `file:` URL names, or `None` when `path` is not a `file:` URL. Schemes are
/// case-insensitive; handles `file:///p`, `file://host/p`, and `file:/p`.
pub(super) fn file_url_local(path: &str) -> Option<&str> {
    if path.len() < 5 || !path.as_bytes()[..5].eq_ignore_ascii_case(b"file:") {
        return None;
    }
    let rest = &path[5..];
    Some(
        rest.strip_prefix("//")
            .map_or(rest, |authority| authority.find('/').map_or("", |i| &authority[i..])),
    )
}

/// Whether `path` is a network URL: a `scheme://…` whose scheme is well-formed (a letter, then
/// letters / digits / `+` / `-` / `.`). A local path that merely contains `://` is not a URL.
pub(super) fn is_network_url(path: &str) -> bool {
    let Some(idx) = path.find("://") else {
        return false;
    };
    let scheme = &path[..idx];
    let well_formed_scheme = scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'));
    well_formed_scheme && has_host(&path[idx + 3..])
}

/// A URL needs a HOST. `s3://../../.ssh/id_rsa` has none, so it is not a URL at all — it is a
/// local path wearing a scheme, and a generic reader hands it straight to the OS, which walks the
/// `..` and opens the key. Treating it as a URL meant skipping the local-path checks entirely,
/// which was harmless only while every path it could climb to was refused anyway.
///
/// Deliberately narrow. A real `https://example.com/.ssh/id_rsa` still classifies as a URL and is
/// the network handler's business, not the shield's.
fn has_host(after_scheme: &str) -> bool {
    let authority = after_scheme.split('/').next().unwrap_or("");
    !authority.is_empty() && authority != "." && authority != ".."
}

/// Whether `path` is a URL rather than a filesystem path: a `scheme://…` or a `file:` URL.
///
/// Admitting a URL is right for a network OPERAND, where the command's own handler gates the
/// network. It is wrong for an EXECUTOR, which has to be a local file the project owns — see
/// `execute_file_verdict`.
pub(crate) fn is_url(path: &str) -> bool {
    file_url_local(path).is_some() || is_network_url(path)
}

#[cfg(test)]
mod tests {
    use super::{file_url_local, is_url};
    use proptest::prelude::*;

    proptest! {
        /// Every spelling of a `file:` URL names the absolute path inside it, and is a URL.
        #[test]
        fn a_file_url_names_the_local_path_inside_it(
            path in "(/[a-zA-Z0-9_.]{1,6}){1,4}",
            scheme in "[fF][iI][lL][eE]",
            host in "[a-z]{1,6}",
        ) {
            for url in [format!("{scheme}://{path}"), format!("{scheme}:{path}"), format!("{scheme}://{host}{path}")] {
                prop_assert_eq!(file_url_local(&url), Some(path.as_str()), "{}", url);
                prop_assert!(is_url(&url), "{}", url);
            }
        }

        /// A plain path is never a URL, and a scheme with no host is a local path wearing a scheme.
        #[test]
        fn a_path_or_a_hostless_scheme_is_not_a_url(
            path in "(/[a-zA-Z0-9_]{1,6}){1,4}",
            scheme in "[a-z][a-z0-9+.-]{0,5}",
        ) {
            prop_assert!(!is_url(&path));
            let hostless = format!("{scheme}://..{path}");
            prop_assert!(scheme.eq_ignore_ascii_case("file") || !is_url(&hostless), "{}", hostless);
            let hosted = format!("{scheme}://example.com{path}");
            prop_assert!(is_url(&hosted), "{}", hosted);
        }
    }
}
