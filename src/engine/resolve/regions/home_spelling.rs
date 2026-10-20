//! The other spellings of a path that a credential shield must also be asked about.

use super::current_os;

/// Every spelling a SHIELD must be asked about: the path itself, plus its macOS firmlink form
/// when the `/private` prefix only matches under case folding.
///
/// `canonicalize` already folds `/private/etc/x` → `/etc/x`, but case-sensitively, because that
/// fold also feeds admits and folding an admit would hand `/PRIVATE/tmp` the scratch rung. So the
/// case variant of a firmlink never reached the shield: `/etc/SHADOW` was refused (nodes fold) and
/// `/PRIVATE/etc/shadow` was not, though on the default macOS volume they are one file.
///
/// Extra spellings are offered to the shield ONLY. Everything else classifies the path as written,
/// which is what keeps a case variant from reaching a permissive node.
///
/// The home folder is offered the same way. On the default macOS volume `/users/me/.s3cfg` and
/// `/System/Volumes/Data/Users/me/.s3cfg` are `~/.s3cfg`, but `canonicalize` folds only the exact
/// `$HOME` to `~`, so both read out as ordinary absolute files.
pub(super) fn shield_spellings(path: &str) -> Vec<String> {
    let mut out = vec![path.to_string()];
    let mut offer = |p: String| {
        if !out.contains(&p) {
            out.push(p);
        }
    };
    let on_data_volume = (current_os() == "macos").then(|| strip_prefix_ci(path, "/System/Volumes/Data")).flatten();
    for spelling in std::iter::once(path).chain(on_data_volume) {
        offer(spelling.to_string());
        if let Some(folded) = crate::engine::resolve::locus::firmlink_fold_case_insensitive(spelling) {
            offer(folded);
        }
        if let Some(home) = home_fold_case_insensitive(spelling) {
            offer(home);
        }
    }
    out
}

/// `path` without `prefix`, matched ASCII-case-insensitively at a component boundary.
pub(super) fn strip_prefix_ci<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    let head = path.get(..prefix.len())?;
    let rest = &path[prefix.len()..];
    (head.eq_ignore_ascii_case(prefix) && rest.starts_with('/')).then_some(rest)
}

/// The `~/…` form of an absolute path under `$HOME` in any case, on macOS only, where the default
/// volume is case-insensitive.
fn home_fold_case_insensitive(path: &str) -> Option<String> {
    if current_os() != "macos" {
        return None;
    }
    let home = std::env::var("HOME").ok().filter(|h| h.starts_with('/') && h.len() > 1)?;
    let rest = strip_prefix_ci(path, home.trim_end_matches('/'))?;
    Some(format!("~{rest}"))
}

#[cfg(test)]
mod tests {
    use super::strip_prefix_ci;
    use proptest::prelude::*;

    proptest! {
        /// A prefix is stripped in any case exactly when it ends at a component boundary, and what
        /// is left is the rest of the path as written.
        #[test]
        fn a_prefix_strips_in_any_case_only_at_a_component_boundary(
            prefix in "(/[a-z]{1,6}){1,3}",
            rest in "(/[a-zA-Z.]{1,6}){1,3}",
            glued in "[a-z]{1,4}",
            flips in prop::collection::vec(any::<bool>(), 24),
        ) {
            let cased: String = prefix
                .chars()
                .zip(flips.iter().cycle())
                .map(|(c, &up)| if up { c.to_ascii_uppercase() } else { c })
                .collect();
            let path = format!("{cased}{rest}");
            prop_assert_eq!(strip_prefix_ci(&path, &prefix), Some(rest.as_str()));
            let longer = format!("{cased}{glued}{rest}");
            prop_assert_eq!(strip_prefix_ci(&longer, &prefix), None);
            prop_assert_eq!(strip_prefix_ci(&cased, &prefix), None);
        }
    }
}
