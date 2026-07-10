//! The directory context the harness supplies (HP-19): the working directory a command
//! runs in, and the project root. It exists to make relative-path classification honest —
//! `cd /etc && echo > ./x` must be seen as writing `/etc/x`, not a worktree file.
//!
//! The context is **ambient** for one command evaluation: a single `cwd`/`root` pair threads
//! logically through the whole recursive verdict tree (script → pipeline → cmd → redirect →
//! leaf). Rather than add a pass-through parameter to ~15 recursive functions across `cst`,
//! `handlers`, and `engine`, it lives in a scoped thread-local, installed by
//! [`enter`] at the top of an evaluation and read at exactly two leaves: legacy
//! `is_safe_write_target` and engine `classify_locus`, both via [`resolve`].
//!
//! Everything is fail-open to *today's* behavior: with no `cwd`/`root` (or an unresolvable
//! path) [`resolve`] returns the path unchanged, so the classifiers behave exactly as before
//! — the signal tightens when present, never a regression when absent.

use std::borrow::Cow;
use std::cell::RefCell;

/// The working directory and project root for the command under evaluation. Both optional:
/// a harness may supply neither (e.g. opencode), and classification falls back to the
/// relative-is-worktree assumption.
#[derive(Clone, Default)]
pub struct PathCtx {
    pub cwd: Option<String>,
    pub root: Option<String>,
}

thread_local! {
    static CURRENT: RefCell<PathCtx> = RefCell::new(PathCtx::default());
}

/// Install `ctx` as the ambient context for the duration of the returned guard; the previous
/// context is restored on drop (panic-safe, so a failing test can't leak into the next).
#[must_use]
pub fn enter(ctx: PathCtx) -> Guard {
    Guard(CURRENT.with(|c| c.replace(ctx)))
}

/// Restores the previous [`PathCtx`] when dropped.
pub struct Guard(PathCtx);

impl Drop for Guard {
    fn drop(&mut self) {
        CURRENT.with(|c| *c.borrow_mut() = std::mem::take(&mut self.0));
    }
}

/// Run `f` with the ambient `cwd` temporarily replaced (root unchanged) — used by intra-line
/// `cd` tracking as it walks a chain's statements. Restored on drop.
#[must_use]
pub fn enter_cwd(cwd: Option<String>) -> Guard {
    Guard(CURRENT.with(|c| {
        let mut b = c.borrow_mut();
        PathCtx { cwd: std::mem::replace(&mut b.cwd, cwd), root: b.root.clone() }
    }))
}

/// The ambient working directory, if known.
pub fn cwd() -> Option<String> {
    CURRENT.with(|c| c.borrow().cwd.clone())
}

/// Resolve a path argument for classification against the ambient `cwd`/`root`. Returns a
/// path the *existing* classifiers (`classify_locus`, `is_safe_write_target`) can score
/// unchanged:
/// - an **absolute**, `~`, or `$`-unpinnable path → returned as-is (they already handle it);
/// - a **relative** path, when `cwd` and `root` are both known and absolute → lexically
///   joined onto `cwd` (no filesystem access). If the result is inside `root` it comes back
///   as a **root-relative** path (so the classifiers see "worktree"); if it escaped `root`
///   (e.g. `cwd` is `/etc`) it comes back **absolute** (so they see `machine`/etc.);
/// - anything else (no context) → returned as-is (today's behavior).
pub fn resolve(path: &str) -> Cow<'_, str> {
    // Absolute / home / unpinnable: the classifiers already resolve these correctly, and a
    // `$` path can't be joined at all. Leave them alone.
    if path.is_empty() || path.starts_with('/') || path.starts_with('~') || path.contains('$') {
        return Cow::Borrowed(path);
    }
    let resolved = CURRENT.with(|c| {
        let ctx = c.borrow();
        match (ctx.cwd.as_deref(), ctx.root.as_deref()) {
            (Some(cwd), Some(root)) if cwd.starts_with('/') && root.starts_with('/') => {
                Some(project_relative_or_absolute(cwd, root, path))
            }
            _ => None,
        }
    });
    resolved.map_or(Cow::Borrowed(path), Cow::Owned)
}

/// Resolve a `cd` target to a new absolute working directory, or `None` if it can't be
/// pinned statically (`cd`, `cd -`, `cd ~…`, `cd $VAR`) — the caller then leaves the running
/// cwd unchanged. `cur` is the current cwd, needed to resolve a *relative* target. Used by
/// intra-line `cd` tracking (HP-19 #2).
pub fn join_cwd(cur: Option<&str>, target: &str) -> Option<String> {
    if target.starts_with('~') || target.contains('$') {
        return None; // home / unpinnable
    }
    if target.starts_with('/') {
        return Some(lexical_join("/", target)); // absolute — normalize
    }
    cur.filter(|c| c.starts_with('/')).map(|c| lexical_join(c, target)) // relative
}

/// Lexically join `rel` onto absolute `cwd`, resolving `.`/`..` without touching the disk,
/// then express the result relative to `root` if it's inside, or absolute if it escaped.
fn project_relative_or_absolute(cwd: &str, root: &str, rel: &str) -> String {
    let abs = lexical_join(cwd, rel);
    let root = root.trim_end_matches('/');
    if abs == root {
        return ".".to_string(); // the project root itself
    }
    match abs.strip_prefix(root) {
        // inside the project → a root-relative path (classified as worktree)
        Some(inside) if inside.starts_with('/') => inside.trim_start_matches('/').to_string(),
        // escaped the project (e.g. cwd is /etc) → the real absolute path
        _ => abs,
    }
}

/// Join a relative path onto an absolute base, resolving `.` and `..` purely lexically. A
/// `..` that would climb above `/` is clamped there.
fn lexical_join(base: &str, rel: &str) -> String {
    let mut parts: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
    for seg in rel.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    format!("/{}", parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_context_leaves_paths_unchanged() {
        assert_eq!(resolve("./x"), "./x");
        assert_eq!(resolve("config"), "config");
        assert_eq!(resolve("/etc/x"), "/etc/x");
    }

    #[test]
    fn relative_inside_the_project_stays_worktree_relative() {
        let _g = enter(PathCtx { cwd: Some("/home/u/proj/sub".into()), root: Some("/home/u/proj".into()) });
        assert_eq!(resolve("x"), "sub/x", "cwd under root → root-relative");
        assert_eq!(resolve("./y"), "sub/y");
        assert_eq!(resolve("../z"), "z", ".. that stays inside root");
    }

    #[test]
    fn relative_outside_the_project_becomes_absolute() {
        let _g = enter(PathCtx { cwd: Some("/etc".into()), root: Some("/home/u/proj".into()) });
        assert_eq!(resolve("x"), "/etc/x", "cd /etc → the real target");
        assert_eq!(resolve("passwd"), "/etc/passwd");
        assert_eq!(resolve("*"), "/etc/*");
    }

    #[test]
    fn dotdot_escaping_the_project_becomes_absolute() {
        let _g = enter(PathCtx { cwd: Some("/home/u/proj".into()), root: Some("/home/u/proj".into()) });
        assert_eq!(resolve("../../../etc/x"), "/etc/x");
    }

    #[test]
    fn absolute_and_unpinnable_are_never_touched_even_with_context() {
        let _g = enter(PathCtx { cwd: Some("/etc".into()), root: Some("/home/u/proj".into()) });
        assert_eq!(resolve("/usr/bin/x"), "/usr/bin/x");
        assert_eq!(resolve("$HOME/x"), "$HOME/x");
        assert_eq!(resolve("~/x"), "~/x");
    }

    #[test]
    fn the_guard_restores_on_drop() {
        {
            let _g = enter(PathCtx { cwd: Some("/etc".into()), root: Some("/r".into()) });
            assert_eq!(resolve("x"), "/etc/x");
        }
        assert_eq!(resolve("x"), "x", "context cleared after the guard drops");
    }
}
