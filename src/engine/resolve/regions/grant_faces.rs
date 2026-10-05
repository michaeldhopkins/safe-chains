//! How the grants matching a path combine into the faces it is granted.

/// Folds `(specificity, read, write)` matches into the most specific, as `regions::best_grant`
/// does with `fold(None, merge)`. A more specific grant decides for what it names; equally
/// specific grants add their faces together, since a grant only ever widens.
///
/// Picking one of the equal ones instead let a read-only grant borrowed from
/// `~/.claude/settings.json` (a `Read(...)` rule, appended after the user's own grants) shadow
/// the user's `write = true` grant for the same directory: the tie went to whichever came last,
/// and every write there still asked.
pub(super) fn merge<S: Ord>(best: Option<(S, bool, bool)>, next: (S, bool, bool)) -> Option<(S, bool, bool)> {
    match best {
        Some((s, r, w)) if s == next.0 => Some((s, r || next.1, w || next.2)),
        Some(best) if best.0 > next.0 => Some(best),
        _ => Some(next),
    }
}

#[cfg(test)]
mod tests {
    use super::merge;
    use crate::engine::resolve::regions::with_user_and_derived_grants;
    use crate::pathctx::{PathCtx, enter};

    #[test]
    fn equally_specific_grants_add_their_faces_and_the_more_specific_decides() {
        let fold = |m: &[(u8, bool, bool)]| m.iter().copied().fold(None, merge).map(|(_, r, w)| (r, w));
        assert_eq!(fold(&[(2, true, true), (2, true, false)]), Some((true, true)), "order 1");
        assert_eq!(fold(&[(2, true, false), (2, true, true)]), Some((true, true)), "order 2");
        assert_eq!(fold(&[(1, true, true), (3, true, false)]), Some((true, false)), "specific wins");
        assert_eq!(fold(&[(3, true, false), (1, true, true)]), Some((true, false)), "either order");
        assert_eq!(fold(&[]), None);
    }

    /// The case that was broken, end to end: the user's own write grant beside a borrowed read
    /// grant for the same directory.
    #[test]
    fn a_borrowed_read_grant_never_shadows_a_write_grant_for_the_same_directory() {
        let Ok(home) = std::env::var("HOME") else { return };
        let ws = format!("{home}/projects/scproj");
        let _g = enter(PathCtx { cwd: Some(ws.clone()), root: Some(ws), ..Default::default() });
        let write = "touch ~/scripts/x";
        let check = crate::is_safe_command;
        assert!(
            with_user_and_derived_grants(&[("~/scripts", true, true)], &[("~/scripts", true, false)], || check(write)),
            "the user's write grant must hold beside a borrowed read grant for the same directory"
        );
        assert!(
            !with_user_and_derived_grants(&[], &[("~/scripts", true, false)], || check(write)),
            "a borrowed read grant alone must still write nothing"
        );
        assert!(
            !with_user_and_derived_grants(&[("~/scripts", true, true), ("~/scripts/keep", true, false)], &[], || check(
                "touch ~/scripts/keep/x"
            ),),
            "a more specific read-only grant still decides for what it names"
        );
    }
}
