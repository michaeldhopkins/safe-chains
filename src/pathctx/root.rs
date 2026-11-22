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

/// The real folder the harness reported, when the command is judged in the unknown one instead:
/// absolute, and not one of our own stand-ins.
pub(super) fn reported_folder() -> Option<String> {
    super::REPORTED_RAW.with(|r| r.borrow().clone()).filter(|c| {
        c.starts_with('/') && !folder::is_unknown(c) && c != UNRESOLVED_CWD && !c.contains('$') && !c.contains("__SAFE_CHAINS_")
    })
}

/// The folder the command runs in on disk: the working directory, or, in the unknown folder, the
/// one the harness reported with any `cd` below it applied. `None` when neither names a real place.
pub(crate) fn folder_on_disk() -> Option<String> {
    let cwd = super::cwd()?;
    let Some(below) = cwd.strip_prefix(crate::targets::UNKNOWN_WORKDIR) else {
        return (cwd != UNRESOLVED_CWD).then_some(cwd);
    };
    if !(below.is_empty() || below.starts_with('/')) {
        return None;
    }
    Some(format!("{}{below}", reported_folder()?.trim_end_matches('/')))
}
