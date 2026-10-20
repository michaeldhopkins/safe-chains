//! Comparisons of path text that the region matchers and grants share.

/// Whether `inner` sits at or below `outer` as a path, by whole components.
///
/// `~/.ssh` is at-or-below `~/.ssh` and `~/.ssh/known_hosts` is below it, but `~/.sshfoo` is not —
/// hence the component boundary rather than a bare `starts_with`.
pub(super) fn at_or_below(inner: &str, outer: &str) -> bool {
    let inner = inner.trim_end_matches('/');
    let outer = outer.trim_end_matches('/');
    // An empty `outer` would make every absolute path read as "below" it. No node roots at `/`
    // today, so this is a fail-closed backstop rather than a live case: nothing NAMES a node that
    // claims the whole filesystem.
    if outer.is_empty() {
        return false;
    }
    inner == outer || inner.strip_prefix(outer).is_some_and(|rest| rest.starts_with('/'))
}

/// Whether `remainder` (a path below a grant root) contains a hidden component — a dotfile/
/// dotdir like `.ssh`, `.env`, `.git-credentials`. Credentials and config live in these, so a
/// broad grant must not sweep them up; grant such a directory explicitly to reach inside it.
pub(super) fn has_hidden_component(remainder: &str) -> bool {
    remainder.split('/').any(|seg| seg.len() > 1 && seg.starts_with('.'))
}

/// ASCII-case-insensitive `starts_with`, zero-alloc (for case-folded shield matching).
pub(super) fn ci_starts_with(haystack: &str, prefix: &str) -> bool {
    haystack.len() >= prefix.len() && haystack.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::{at_or_below, ci_starts_with, has_hidden_component};
    use proptest::prelude::*;

    proptest! {
        /// A path is at or below a root exactly when it is the root or continues it past a `/`;
        /// a name that only begins with the root's last component is a neighbour, not inside it.
        #[test]
        fn a_path_is_below_a_root_only_past_a_component_boundary(
            root in "(~|/[a-z]{1,4})(/[.a-z]{1,5}){0,3}",
            rest in "[.a-z]{1,5}(/[.a-z]{1,5}){0,2}",
        ) {
            prop_assert!(at_or_below(&root, &root));
            let (below, neighbour) = (format!("{root}/{rest}"), format!("{root}{rest}"));
            prop_assert!(at_or_below(&below, &root));
            prop_assert!(!at_or_below(&neighbour, &root));
            prop_assert!(!at_or_below(&root, ""));
        }

        /// The case-insensitive prefix agrees with comparing lowercased copies.
        #[test]
        fn the_folded_prefix_agrees_with_lowercasing(haystack in "[a-zA-Z./]{0,10}", prefix in "[a-zA-Z./]{0,6}") {
            let oracle = haystack.to_ascii_lowercase().starts_with(&prefix.to_ascii_lowercase());
            prop_assert_eq!(ci_starts_with(&haystack, &prefix), oracle);
        }

        /// A remainder holds a hidden component exactly when one of its components is a dot name.
        #[test]
        fn a_hidden_component_is_found_wherever_it_is(parts in prop::collection::vec("[a-z]{1,4}", 0..4), at in 0usize..4, name in "[a-z]{1,4}") {
            let plain = parts.join("/");
            prop_assert!(!has_hidden_component(&plain));
            let mut with = parts.clone();
            let hidden = format!(".{name}");
            with.insert(at.min(parts.len()), hidden);
            prop_assert!(has_hidden_component(&with.join("/")));
        }
    }
}
