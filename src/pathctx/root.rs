//! The project root a command is judged against.

use super::{CURRENT, UNRESOLVED_CWD, folder};

/// The project root a command is judged against.
///
/// An unknown root is a value, not an absence, so no consumer can read it as "no boundary": each
/// has to match on [`Root::Unknown`] and say what it does then, and every place that grants
/// something on the strength of the root fails closed there. The ratchet test
/// `root_consumers_tests` lists every consumer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Root {
    Known(String),
    Unknown,
}

/// The workspace ROOT (project dir), falling back to `cwd` (the hook defaults root to cwd), or
/// [`Root::Unknown`] when the context names a folder that is not a usable one: the unknown-folder
/// sentinel, or a `cd` that could not be resolved.
///
/// An evaluation that was handed no context at all (a library caller, not a harness) has no
/// workspace boundary to be wrong about, so its root is the empty `Known("")`: not an absolute
/// path, so nothing is placed in it, and not unknown, so relative paths keep classifying as they
/// always did without one. A harness always enters a context.
pub fn root() -> Root {
    CURRENT.with(|c| {
        let b = c.borrow();
        if b.cwd.is_none() && b.root.is_none() {
            return Root::Known(String::new());
        }
        match b.root.as_ref().or(b.cwd.as_ref()).filter(|r| !folder::is_unknown(r) && *r != UNRESOLVED_CWD) {
            Some(root) => Root::Known(root.clone()),
            None => Root::Unknown,
        }
    })
}
