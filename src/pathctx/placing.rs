//! Placing a path argument: joined onto the working directory, followed through links on disk, and
//! written relative to the project root when it lands inside it.

use std::borrow::Cow;
use std::cell::Cell;

use super::home::home_path_inside_root;
use super::lexical::{expand_home, express_relative_to_root, lexical_join};
use super::{CURRENT, anchor, folder, physical};

thread_local! {
    static ON_THE_LINK: Cell<bool> = const { Cell::new(false) };
}

/// Runs `judge` with the last component of every path kept, not followed: the command acts on a link
/// named there (`cp -R link x`, `chmod -h`), not on what it leads to. A path ending in `/` or `/.`
/// is still followed, as the kernel follows it.
pub(crate) fn on_the_link<T>(judge: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            ON_THE_LINK.set(self.0);
        }
    }
    let _restore = Restore(ON_THE_LINK.replace(true));
    judge()
}

pub(super) fn resolve_placed(path: &str, use_: Option<anchor::Use>) -> Cow<'_, str> {
    if path.is_empty() || path.contains('$') {
        return Cow::Borrowed(path);
    }
    let follow_last = use_ != Some(anchor::Use::Rebind) && !ON_THE_LINK.get();
    let root = CURRENT.with(|c| c.borrow().root.clone()).filter(|r| r.starts_with('/') && !folder::is_unknown(r));
    if path.starts_with('~') {
        let real = expand_home(path)
            .and_then(|abs| physical::resolve(&abs, follow_last))
            .map(|real| {
                root.as_deref()
                    .map_or_else(|| real.clone(), |root| express_relative_to_root(&real, &physical::root(root)))
            })
            .filter(|placed| placed.starts_with('/'));
        if let Some(real) = real {
            return Cow::Owned(real);
        }
        return home_path_inside_root(path).map_or(Cow::Borrowed(path), Cow::Owned);
    }
    let resolved = CURRENT.with(|c| {
        let ctx = c.borrow();
        match (ctx.cwd.as_deref(), ctx.root.as_deref()) {
            (Some(cwd), Some(root)) if cwd.starts_with('/') && root.starts_with('/') => {
                if !path.starts_with('/')
                    && !use_.is_some_and(anchor::Use::mutates)
                    && let Some(real) = through_a_link_in_the_reported_folder(cwd, path, follow_last)
                {
                    return Some(real);
                }
                if folder::is_unknown(root)
                    && !path.starts_with('/')
                    && !use_.is_some_and(anchor::Use::mutates)
                    && !super::reported_is_secret()
                    && let Some(placed) = folder::place(cwd, path, use_)
                {
                    return Some(placed);
                }
                let raw = if path.starts_with('/') {
                    path.to_string()
                } else {
                    folder::place(cwd, path, use_).map_or_else(|| format!("{cwd}/{path}"), |placed| format!("{root}/{placed}"))
                };
                Some(match physical::resolve(&raw, follow_last) {
                    Some(real) => express_relative_to_root(&real, &physical::root(root)),
                    None => express_relative_to_root(&lexical_join("/", &raw), root),
                })
            }
            _ if path.starts_with('/') => physical::resolve(path, follow_last),
            _ => None,
        }
    });
    resolved.map_or(Cow::Borrowed(path), Cow::Owned)
}

/// Where a relative `path` read in the unknown folder lands when it passes through a link in the
/// real folder the harness reported, or `None` when it meets no link there.
///
/// The unknown folder names no place on disk, so a file committed as a link to `~/.ssh/id_rsa` would
/// otherwise be judged as a file of the folder. The command still runs in the reported folder, where
/// the kernel follows the link, so the path is judged where it lands, as it is in a known
/// workspace. Only reads come here: the real place is never judged more
/// leniently than a file of the unknown folder for a read, and writes there are judged by name.
fn through_a_link_in_the_reported_folder(cwd: &str, path: &str, follow_last: bool) -> Option<String> {
    let below = cwd.strip_prefix(crate::targets::UNKNOWN_WORKDIR)?;
    if !(below.is_empty() || below.starts_with('/')) {
        return None;
    }
    // A link anywhere on the way counts, the reported folder's own spelling included: a session
    // that ran `cd keys`, where `keys` leads to `~/.ssh`, reports the folder by the link's name.
    let reported = super::reported_folder()?;
    let below = below.trim_start_matches('/');
    let relative = if below.is_empty() { path.to_string() } else { format!("{below}/{path}") };
    physical::resolve(&format!("{}/{relative}", reported.trim_end_matches('/')), follow_last)
}
