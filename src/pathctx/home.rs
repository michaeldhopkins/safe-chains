//! Resolving a `~/…` spelling against the workspace root.

use super::CURRENT;
use super::lexical::{expand_home, express_relative_to_root, lexical_join};

/// The root-relative form of a `~/…` path whose expansion lies inside an absolute `root`, or
/// `None`, which leaves it to the home classifiers.
///
/// Only while `cwd` is inside the root too: by this point quoting is gone, and a quoted `"~/x"` is
/// a literal `~` directory under `cwd`, which is then inside the root as well. A glob, another
/// user's `~name`, a root inside a protected place (`~/.ssh`) and a root holding one (`$HOME`) are
/// left alone, so a `~/` spelling never reaches further than the home classifiers allow.
pub(super) fn home_path_inside_root(path: &str) -> Option<String> {
    if !path.starts_with("~/") || path.contains(['*', '?', '[']) {
        return None;
    }
    let (cwd, root) = CURRENT.with(|c| {
        let ctx = c.borrow();
        (ctx.cwd.clone(), ctx.root.clone())
    });
    let (cwd, root) = (cwd.filter(|c| c.starts_with('/'))?, root.filter(|r| r.starts_with('/'))?);
    if express_relative_to_root(&lexical_join("/", &cwd), &root).starts_with('/')
        || crate::engine::resolve::regions::protection_covers(&root)
    {
        return None;
    }
    let relative = express_relative_to_root(&lexical_join("/", &expand_home(path)?), &root);
    (!relative.starts_with('/')).then_some(relative)
}
