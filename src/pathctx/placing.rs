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
    let root = CURRENT.with(|c| c.borrow().root.clone()).filter(|r| r.starts_with('/'));
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
                // Relative → join onto cwd; absolute → normalize in place. Then express relative
                // to root if inside (worktree), else absolute.
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
