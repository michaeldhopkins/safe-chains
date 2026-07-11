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

/// A bound `for` loop variable: `$name` in the body inherits the loop's `in`-list locus (the
/// `find … {}`→path binding, one layer up). Read and write representatives can differ — a list
/// like `/etc/hosts ~/notes` reads worst at `~/notes` but writes worst at `/etc/hosts`.
struct LoopVar {
    name: String,
    read_repr: String,
    write_repr: String,
}

thread_local! {
    static LOOP_VARS: RefCell<Vec<LoopVar>> = const { RefCell::new(Vec::new()) };
}

/// Bind loop variable `name` to its list's representative items for the duration of the guard
/// (the loop body's classification). Nested loops stack; the innermost binding of a name wins.
#[must_use]
pub fn enter_loop_var(name: String, read_repr: String, write_repr: String) -> LoopGuard {
    LOOP_VARS.with(|v| v.borrow_mut().push(LoopVar { name, read_repr, write_repr }));
    LoopGuard
}

/// Pops the loop binding when dropped.
pub struct LoopGuard;

impl Drop for LoopGuard {
    fn drop(&mut self) {
        LOOP_VARS.with(|v| {
            v.borrow_mut().pop();
        });
    }
}

thread_local! {
    static STDIN_REPR: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Bind the representative PATH of the items arriving on stdin, for the duration of the guard —
/// set by the pipeline walker to the previous stage's output-path locus. An operand-injecting
/// consumer (`xargs`) reads it so `find / | xargs cat` gates the injected operand at `/`, while
/// `find ./src | xargs cat` gates it at the workspace (mirrors `find -exec`'s `{}` binding).
#[must_use]
pub fn enter_stdin_repr(repr: String) -> StdinReprGuard {
    STDIN_REPR.with(|v| v.borrow_mut().push(repr));
    StdinReprGuard
}

/// The current stdin-item representative, or `None` when the source is unknown (no pipe / an
/// unmodeled producer) — in which case the consumer worst-cases the injected operand.
pub fn stdin_item_repr() -> Option<String> {
    STDIN_REPR.with(|v| v.borrow().last().cloned())
}

pub struct StdinReprGuard;

impl Drop for StdinReprGuard {
    fn drop(&mut self) {
        STDIN_REPR.with(|v| {
            v.borrow_mut().pop();
        });
    }
}

/// Expand any bound loop variable (`$name` / `${name}`) in `path` to its representative list
/// item — the read representative when `want_write` is false, the write representative when
/// true. Unbound `$…` is left untouched (so it still fail-closes to machine). Returns `path`
/// unchanged when nothing is bound.
pub fn expand_loop(path: &str, want_write: bool) -> Cow<'_, str> {
    if !path.contains('$') {
        return Cow::Borrowed(path);
    }
    let replaced = LOOP_VARS.with(|v| {
        let vars = v.borrow();
        if vars.is_empty() {
            None
        } else {
            expand_with(path, &vars, want_write)
        }
    });
    replaced.map_or(Cow::Borrowed(path), Cow::Owned)
}

fn expand_with(path: &str, vars: &[LoopVar], want_write: bool) -> Option<String> {
    let mut out = String::with_capacity(path.len());
    let mut rest = path;
    let mut replaced = false;
    while let Some(dollar) = rest.find('$') {
        out.push_str(&rest[..dollar]);
        let after = &rest[dollar + 1..];
        match parse_var(after) {
            Some((name, consumed)) => {
                if let Some(lv) = vars.iter().rev().find(|v| v.name == name) {
                    out.push_str(if want_write { &lv.write_repr } else { &lv.read_repr });
                    replaced = true;
                } else {
                    out.push('$');
                    out.push_str(&after[..consumed]);
                }
                rest = &after[consumed..];
            }
            None => {
                out.push('$');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    replaced.then_some(out)
}

/// Parse a shell variable name immediately after a `$`: `name` or `{name}`. Returns the name
/// and how many bytes of `after` it consumed, or `None` if it isn't a plain variable reference.
fn parse_var(after: &str) -> Option<(&str, usize)> {
    if let Some(braced) = after.strip_prefix('{') {
        let close = braced.find('}')?;
        let name = &braced[..close];
        is_var_name(name).then_some((name, close + 2)) // '{' + name + '}'
    } else {
        let len = after.bytes().take_while(|&b| b.is_ascii_alphanumeric() || b == b'_').count();
        let name = &after[..len];
        is_var_name(name).then_some((name, len))
    }
}

fn is_var_name(s: &str) -> bool {
    let mut bytes = s.bytes();
    matches!(bytes.next(), Some(b) if b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Resolve a path argument for classification against the ambient `cwd`/`root`. Returns a
/// path the *existing* classifiers (`classify_locus`, `is_safe_write_target`) can score
/// unchanged. When `cwd` and `root` are both known and absolute, a **relative** path is lexically
/// joined onto `cwd` (no filesystem access) and an **absolute** path is normalized in place; then
/// either way, if the result is inside `root` it comes back as a **root-relative** path (so the
/// classifiers see "worktree"), and if it escaped `root` (e.g. `cwd` is `/etc`, or an absolute
/// `/etc/hosts`) it comes back **absolute** (so they see `machine`/etc.). This makes the absolute
/// and relative spellings of the SAME in-root file classify identically — safety on the OPERATION,
/// not the SYNTAX. A `~` (home) or `$`-unpinnable path, or no context, is returned as-is.
pub fn resolve(path: &str) -> Cow<'_, str> {
    // Home (`~`) is handled by the classifiers directly; a `$` path can't be joined at all.
    if path.is_empty() || path.starts_with('~') || path.contains('$') {
        return Cow::Borrowed(path);
    }
    let resolved = CURRENT.with(|c| {
        let ctx = c.borrow();
        match (ctx.cwd.as_deref(), ctx.root.as_deref()) {
            (Some(cwd), Some(root)) if cwd.starts_with('/') && root.starts_with('/') => {
                // Relative → join onto cwd; absolute → normalize in place. Then express relative
                // to root if inside (worktree), else absolute.
                let abs = if path.starts_with('/') {
                    lexical_join("/", path)
                } else {
                    lexical_join(cwd, path)
                };
                Some(express_relative_to_root(&abs, root))
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

/// Express an absolute `abs` path relative to `root`: `.` if it IS the root, a root-relative path
/// if it's inside (classified as worktree), or the absolute path unchanged if it escaped (classified
/// as machine/etc.). The `inside.starts_with('/')` guard prevents a sibling like `/proj-evil` from
/// matching root `/proj` by bare string prefix.
fn express_relative_to_root(abs: &str, root: &str) -> String {
    let root = root.trim_end_matches('/');
    if abs == root {
        return ".".to_string(); // the project root itself
    }
    match abs.strip_prefix(root) {
        Some(inside) if inside.starts_with('/') => inside.trim_start_matches('/').to_string(),
        _ => abs.to_string(),
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
    fn absolute_in_root_becomes_root_relative_outside_stays_absolute() {
        let _g = enter(PathCtx { cwd: Some("/home/u/proj/sub".into()), root: Some("/home/u/proj".into()) });
        // absolute INSIDE root → root-relative (worktree), matching the relative spelling
        assert_eq!(resolve("/home/u/proj/main.rs"), "main.rs");
        assert_eq!(resolve("/home/u/proj/sub/x"), "sub/x");
        assert_eq!(resolve("/home/u/proj/a/../b"), "b", "normalized in place");
        assert_eq!(resolve("/home/u/proj"), ".", "the project root itself");
        // absolute OUTSIDE root → unchanged (classified as machine)
        assert_eq!(resolve("/usr/bin/x"), "/usr/bin/x");
        assert_eq!(resolve("/home/u/proj/../../etc/x"), "/home/etc/x", "climbs to /home, still outside root");
        assert_eq!(resolve("/home/u/proj/../../../etc/x"), "/etc/x", "escapes to /etc via ..");
        assert_eq!(
            resolve("/home/u/proj-evil/secret"), "/home/u/proj-evil/secret",
            "a sibling dir is not confused for inside by bare string prefix",
        );
        // home / unpinnable → returned as-is (the classifiers handle these)
        assert_eq!(resolve("$HOME/x"), "$HOME/x");
        assert_eq!(resolve("~/x"), "~/x");
    }

    #[test]
    fn loop_var_expands_to_its_representative_per_face() {
        let _g = enter_loop_var("f".into(), "read_item".into(), "write_item".into());
        assert_eq!(expand_loop("$f", false), "read_item");
        assert_eq!(expand_loop("$f", true), "write_item");
        assert_eq!(expand_loop("${f}", false), "read_item");
        assert_eq!(expand_loop("$f.bak", false), "read_item.bak", "compound suffix");
        assert_eq!(expand_loop("pre/$f", false), "pre/read_item");
        assert_eq!(expand_loop("$foo", false), "$foo", "$foo is not $f");
        assert_eq!(expand_loop("$g", false), "$g", "unbound var untouched");
        assert_eq!(expand_loop("plain", false), "plain");
    }

    #[test]
    fn loop_var_binding_is_scoped_and_nests() {
        assert_eq!(expand_loop("$f", false), "$f", "no binding");
        {
            let _outer = enter_loop_var("f".into(), "outer".into(), "outer".into());
            {
                let _inner = enter_loop_var("f".into(), "inner".into(), "inner".into());
                assert_eq!(expand_loop("$f", false), "inner", "innermost wins");
            }
            assert_eq!(expand_loop("$f", false), "outer", "inner popped on drop");
        }
        assert_eq!(expand_loop("$f", false), "$f", "all popped");
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
