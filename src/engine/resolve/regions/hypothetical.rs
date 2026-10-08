//! Questions about grants that have not been written yet.
//!
//! Two callers need them, and neither may change a verdict. `--explain` offers a grant only when
//! the grant would change the answer, which it learns by asking with the grant in place.
//! `safe-chains config grant` refuses a path a built-in rule protects, which it learns from the
//! path's role before any grant applies.

use std::cell::RefCell;

use super::{Grant, GrantSource, Role, base_region, grant_matchers};

thread_local! {
    static EXTRA: RefCell<Vec<Grant>> = const { RefCell::new(Vec::new()) };
}

/// Hand `best_grant` the hypothetical grants alongside the real ones. Empty outside
/// [`with_hypothetical_grant`], which is every classification the hook and the CLI make.
pub(super) fn with_extra<T>(f: impl FnOnce(&[Grant]) -> T) -> T {
    EXTRA.with(|extra| f(&extra.borrow()))
}

/// Run `f` as though `~/.config/safe-chains.toml` also held `[[grant]] path, read, write`.
///
/// The grant is gone when `f` returns or unwinds, so it cannot outlive the question it answers.
pub(crate) fn with_hypothetical_grant<T>(path: &str, read: bool, write: bool, f: impl FnOnce() -> T) -> T {
    struct Clear;
    impl Drop for Clear {
        fn drop(&mut self) {
            EXTRA.with(|extra| extra.borrow_mut().clear());
        }
    }
    let grants = grant_matchers(path)
        .into_iter()
        .map(|matcher| Grant { matcher, read, write, source: GrantSource::UserConfig });
    EXTRA.with(|extra| *extra.borrow_mut() = grants.collect());
    let _clear = Clear;
    f()
}

/// The role `path` has before any grant is applied: what the built-in rules alone say about it.
pub(crate) fn ungranted_role(path: &str) -> Role {
    base_region(&super::super::locus::canonicalize(path))
}

/// The read grants safe-chains borrows from a Claude Code `settings.json` body, as paths. For
/// `safe-chains config list`, which shows where each rule came from.
pub(crate) fn claude_read_grants(settings_json: &str) -> Vec<String> {
    super::claude_read_grant_paths(settings_json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::resolve::write_target_verdict;

    #[test]
    fn a_hypothetical_grant_widens_only_while_it_is_asked_about() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/tester".to_string());
        let path = format!("{home}/hypothetical-probe/notes.txt");
        assert!(!write_target_verdict(&path).is_allowed(), "the probe must start refused");
        let inside =
            with_hypothetical_grant(&format!("{home}/hypothetical-probe"), false, true, || write_target_verdict(&path).is_allowed());
        assert!(inside, "the grant must apply while it is asked about");
        assert!(!write_target_verdict(&path).is_allowed(), "the grant outlived its question");
    }

    #[test]
    fn a_hypothetical_grant_is_cleared_after_a_panic() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/tester".to_string());
        let path = format!("{home}/hypothetical-probe/notes.txt");
        let unwound =
            std::panic::catch_unwind(|| with_hypothetical_grant(&format!("{home}/hypothetical-probe"), true, true, || panic!("boom")));
        assert!(unwound.is_err());
        assert!(!write_target_verdict(&path).is_allowed(), "a panic left the grant in place");
    }

    #[test]
    fn the_ungranted_role_ignores_grants() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/tester".to_string());
        let ssh = format!("{home}/.ssh/id_rsa");
        assert!(ungranted_role(&ssh).reads_secret);
        let still = with_hypothetical_grant(&format!("{home}/.ssh"), true, true, || ungranted_role(&ssh).reads_secret);
        assert!(still, "a grant changed the role it is measured against");
    }
}
