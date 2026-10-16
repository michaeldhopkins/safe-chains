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

pub mod anchor;
pub mod folder;
pub(crate) use folder::judging;
mod home;
pub mod item_shape;
mod lexical;
mod physical;
mod placing;
mod vars;
pub use item_shape::{Binding, Facts, ItemShape, ItemShapeGuard, binding, enter_loop_shape, enter_stdin_shape, loop_shape, stdin_shape};
use lexical::{expand_home, lexical_join, names_no_known_directory};
pub(crate) use placing::on_the_link;
use placing::resolve_placed;
pub use vars::{LoopGuard, VarGuard, enter_loop_var, enter_var, expand_vars, names_a_loop_var};

/// The working directory and project root for the command under evaluation. Both optional:
/// a harness may supply neither (e.g. opencode), and classification falls back to the
/// relative-is-worktree assumption.
#[derive(Clone, Default)]
pub struct PathCtx {
    pub cwd: Option<String>,
    pub root: Option<String>,
    /// The harness's session id, when it supplies one. Only used to recognize this session's
    /// SCRATCHPAD — see [`in_session_scratchpad`].
    pub session_id: Option<String>,
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
        // `session_id` is carried through unchanged: a `cd` mid-chain must not drop scratchpad
        // recognition (the session is the same session whatever directory it walks into).
        PathCtx { cwd: std::mem::replace(&mut b.cwd, cwd), root: b.root.clone(), session_id: b.session_id.clone() }
    }))
}

/// The ambient working directory, if known.
pub fn cwd() -> Option<String> {
    CURRENT.with(|c| c.borrow().cwd.clone())
}

/// The workspace ROOT (project dir), if known — falling back to `cwd` (the hook defaults root to
/// cwd). Used by the adjacent-sibling classifier to find the workspace's parent.
pub fn root() -> Option<String> {
    CURRENT.with(|c| {
        let b = c.borrow();
        b.root.clone().or_else(|| b.cwd.clone())
    })
}

/// Whether `path` lies inside THIS session's scratchpad — the harness's own per-session working
/// directory (e.g. Claude Code's `/private/tmp/claude-<uid>/<project-slug>/<session-id>/scratchpad`).
///
/// The anchor is the **session id as a whole path component**, not the surrounding layout. That is
/// deliberate and is what makes this safe *and* durable:
///
/// - **Unforgeable.** The session id arrives in the harness's own hook envelope, never from the
///   agent's shell. An attacker cannot pre-plant `/tmp/<this-session-id>/evil.sh` because the id is
///   unknown until the session exists (and it is unique per session). Compare a *layout* pattern
///   ("anything under `/tmp/claude-*`"), which anyone can create.
/// - **Durable.** It survives the harness reorganizing the parts around the id — the uid suffix,
///   the slug, `/tmp` vs `/private/tmp`, the trailing directory name. Those are internal details
///   (Claude Code does not document or expose the scratchpad path, and declined to; see
///   docs/design/agent-scratchpad.md), so matching them exactly would be brittle.
/// - **Fail-closed.** No session id, or a path that does not contain it, simply does not match:
///   the path keeps its ordinary classification (`/tmp` → `temp`, i.e. foreign). A harness that
///   supplies no id, or whose scratchpad omits it, is exactly as restricted as before — never worse.
///
/// Requiring a TEMP-root prefix as well keeps the id from blessing something outside the scratch
/// area on a harness that happens to embed the id elsewhere (a log path under `$HOME`, say).
pub fn in_session_scratchpad(path: &str) -> bool {
    let Some(id) = CURRENT.with(|c| c.borrow().session_id.clone()) else {
        return false;
    };
    // A short or trivial id could collide with an ordinary directory name; require something
    // id-shaped before trusting it as an anchor.
    if id.len() < 8 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return false;
    }
    if !under_temp_root(path) {
        return false;
    }
    path.split('/').any(|seg| seg == id)
}

/// Whether `path` is under a temporary-filesystem root. macOS's `/tmp` is a symlink to
/// `/private/tmp`, and harnesses report either spelling, so both are accepted (as is `$TMPDIR`).
pub fn under_temp_root(path: &str) -> bool {
    const ROOTS: &[&str] = &["/tmp/", "/private/tmp/", "/var/tmp/", "/private/var/tmp/"];
    if ROOTS.iter().any(|r| path.starts_with(r)) {
        return true;
    }
    std::env::var("TMPDIR").ok().is_some_and(|t| {
        let t = t.trim_end_matches('/');
        !t.is_empty() && t.starts_with('/') && path.starts_with(&format!("{t}/"))
    })
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

/// Resolve a path argument for classification against the ambient `cwd`/`root`. Returns a
/// path the *existing* classifiers (`classify_locus`, `is_safe_write_target`) can score
/// unchanged. When `cwd` and `root` are both known and absolute, a **relative** path is lexically
/// joined onto `cwd` (no filesystem access) and an **absolute** path is normalized in place; then
/// either way, if the result is inside `root` it comes back as a **root-relative** path (so the
/// classifiers see "worktree"), and if it escaped `root` (e.g. `cwd` is `/etc`, or an absolute
/// `/etc/hosts`) it comes back **absolute** (so they see `machine`/etc.). This makes the absolute
/// and relative spellings of the SAME in-root file classify identically — safety on the OPERATION,
/// not the SYNTAX. A `~/…` path that expands into `root` comes back root-relative too, as its
/// `$HOME/…` spelling does; any other `~` path, a `$`-unpinnable one, or no context, is returned
/// as-is for the home classifiers. In an unknown folder (`folder`), a placed path joins the root.
pub fn resolve(path: &str) -> Cow<'_, str> {
    resolve_placed(path, None)
}

/// As [`resolve`], for a path put to `use_`. A write or a rebind is recorded as one the command leaf
/// being judged NAMED (`folder`).
pub fn resolve_for(path: &str, use_: anchor::Use) -> Cow<'_, str> {
    if use_.mutates() {
        folder::note_named_write();
    }
    resolve_placed(path, Some(use_))
}

/// The cwd after a `cd` whose target cannot be pinned. It looks like a path AND is unpinnable, so
/// every later relative path resolved against it worst-cases in both gate layers.
pub(crate) const UNRESOLVED_CWD: &str = "/__SAFE_CHAINS_CMDSUB__";

/// Resolve a `cd` target to a new working directory. `cur` is the current cwd, needed for a
/// *relative* target. Used by intra-line `cd` tracking (HP-19 #2).
///
/// A target that cannot be pinned yields `UNRESOLVED_CWD`, NOT `None`. This used to return `None`
/// for `~…` and `$VAR`, and the caller reads `None` as "no cd happened" and keeps the previous cwd
/// — so the `cd` was silently ignored and every later relative path was judged against a workspace
/// the shell had already left. `cd ~/.aws && cat credentials` and `cd ~/.claude && echo … >
/// settings.json` both auto-approved that way, the second writing the very file the allowlist
/// bridge trusts. `None` now means only what the caller can act on: `cd_target` already returned
/// `None` for "not a cd" (bare `cd`, `cd -`), so reaching here means the shell definitely moved.
///
/// `~`/`~/…` are EXPANDED rather than blanket-refused, because they are pinnable — that keeps
/// `cd ~/.aws && cat credentials` gated as the credential read it is instead of coarsely denying
/// everything downstream.
pub fn join_cwd(cur: Option<&str>, target: &str) -> Option<String> {
    join_cwd_mode(cur, target, false)
}

pub fn join_cwd_mode(cur: Option<&str>, target: &str, physical: bool) -> Option<String> {
    let expanded = match expand_home(target) {
        Some(t) => t,
        None => return Some(UNRESOLVED_CWD.to_string()), // `~` with no HOME
    };
    // Another user's home, a variable, an UNDECLARED substitution: the shell moved somewhere we
    // cannot name. Fail closed rather than pretend it stayed put.
    //
    // A DECLARED substitution (`cd $(pwd)`) is deliberately not in that set. Its sentinel carries a
    // locus, so letting it through the joins below keeps the cwd at that locus — `cd $(pwd) && cat
    // f` stays a worktree read, while `cd $(fd d /etc) && cat f` is a machine one. Refusing both
    // would have been sound but needlessly coarse.
    if names_no_known_directory(&expanded) {
        return Some(UNRESOLVED_CWD.to_string());
    }
    let raw = if expanded.starts_with('/') {
        expanded.into_owned()
    } else {
        let c = cur.filter(|c| c.starts_with('/'))?;
        format!("{c}/{expanded}")
    };
    Some(if physical { physical::resolve(&raw, true).unwrap_or_else(|| lexical_join("/", &raw)) } else { lexical_join("/", &raw) })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SID: &str = "7676dbc5-a265-43b3-a0f8-49666792bd9b";

    fn with_session<T>(id: Option<&str>, f: impl FnOnce() -> T) -> T {
        let _g = enter(PathCtx { cwd: Some("/home/u/proj".into()), root: Some("/home/u/proj".into()), session_id: id.map(str::to_string) });
        f()
    }

    /// The recognition rule is a SECURITY boundary: matching a path that is not really this
    /// session's scratchpad would hand `sandbox-scope` (and therefore EXECUTE) to foreign code. So
    /// enumerate the spoofing class rather than one example — every way a hostile or unrelated path
    /// could try to look like the scratchpad must fail, and every legitimate spelling must match.
    #[test]
    fn only_this_sessions_scratchpad_is_recognized() {
        let scratch = format!("/private/tmp/claude-501/-Users-u-proj/{SID}/scratchpad");
        let matching: &[String] = &[
            format!("{scratch}/build.sh"),
            format!("{scratch}/nested/deep/gen.py"),
            scratch.clone(),
            // layout around the id varies by harness/platform — the id is the anchor, not the shape
            format!("/tmp/{SID}/x.sh"),
            format!("/tmp/some-other-harness/{SID}/work/x.sh"),
            format!("/var/tmp/{SID}/x.sh"),
        ];
        let rejected: &[String] = &[
            // a DIFFERENT session's scratchpad — same layout, wrong id
            "/private/tmp/claude-501/-Users-u-proj/00000000-1111-2222-3333-444444444444/scratchpad/x.sh".into(),
            // the id as a SUBSTRING of a component, not a component (the prefix/suffix attack)
            format!("/tmp/{SID}-evil/x.sh"),
            format!("/tmp/evil-{SID}/x.sh"),
            format!("/tmp/a{SID}/x.sh"),
            // right id, but OUTSIDE any temp root — the id must not bless arbitrary locations
            format!("/home/u/{SID}/x.sh"),
            format!("~/.ssh/{SID}/id_rsa"),
            format!("/etc/{SID}/passwd"),
            // anonymous temp files carry no id at all
            "/tmp/evil.sh".into(),
            "/private/tmp/downloaded.sh".into(),
        ];
        with_session(Some(SID), || {
            for p in matching {
                assert!(in_session_scratchpad(p), "should be recognized: {p}");
            }
            for p in rejected {
                assert!(!in_session_scratchpad(p), "must NOT be recognized: {p}");
            }
        });
    }

    /// Fail closed on every degenerate session id: no harness id, or one too short/odd to be a
    /// trustworthy anchor, must recognize NOTHING — never widen a path.
    #[test]
    fn a_missing_or_unusable_session_id_recognizes_nothing() {
        let path = format!("/tmp/{SID}/x.sh");
        with_session(None, || {
            assert!(!in_session_scratchpad(&path), "no session id → no recognition");
        });
        for weak in ["", "abc", "1234567", "..", "/", "a/b", "id with space", "x*y"] {
            with_session(Some(weak), || {
                assert!(!in_session_scratchpad(&format!("/tmp/{weak}/x.sh")), "weak id {weak:?} must not anchor recognition",);
            });
        }
    }

    #[test]
    fn no_context_leaves_paths_unchanged() {
        assert_eq!(resolve("./x"), "./x");
        assert_eq!(resolve("config"), "config");
        assert_eq!(resolve("/etc/x"), "/etc/x");
    }

    #[test]
    fn relative_inside_the_project_stays_worktree_relative() {
        let _g = enter(PathCtx { cwd: Some("/home/u/proj/sub".into()), root: Some("/home/u/proj".into()), ..Default::default() });
        assert_eq!(resolve("x"), "sub/x", "cwd under root → root-relative");
        assert_eq!(resolve("./y"), "sub/y");
        assert_eq!(resolve("../z"), "z", ".. that stays inside root");
    }

    #[test]
    fn relative_outside_the_project_becomes_absolute() {
        let _g = enter(PathCtx { cwd: Some("/etc".into()), root: Some("/home/u/proj".into()), ..Default::default() });
        assert_eq!(resolve("x"), "/etc/x", "cd /etc → the real target");
        assert_eq!(resolve("passwd"), "/etc/passwd");
        assert_eq!(resolve("no-such-dir/*"), "/etc/no-such-dir/*");
    }

    #[test]
    fn dotdot_escaping_the_project_becomes_absolute() {
        let _g = enter(PathCtx { cwd: Some("/home/u/proj".into()), root: Some("/home/u/proj".into()), ..Default::default() });
        assert_eq!(resolve("../../../etc/x"), "/etc/x");
    }

    #[test]
    fn absolute_in_root_becomes_root_relative_outside_stays_absolute() {
        let _g = enter(PathCtx { cwd: Some("/home/u/proj/sub".into()), root: Some("/home/u/proj".into()), ..Default::default() });
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
            resolve("/home/u/proj-evil/secret"),
            "/home/u/proj-evil/secret",
            "a sibling dir is not confused for inside by bare string prefix",
        );
        // home / unpinnable → returned as-is (the classifiers handle these)
        assert_eq!(resolve("$HOME/x"), "$HOME/x");
        assert_eq!(resolve("~/x"), "~/x");
    }

    #[test]
    fn a_home_spelled_path_inside_root_becomes_root_relative() {
        let Some(home) = std::env::var("HOME").ok().filter(|h| h.starts_with('/') && h.len() > 1) else {
            return;
        };
        let root = format!("{home}/projects/app");
        let _g = enter(PathCtx { cwd: Some(format!("{root}/sub")), root: Some(root), ..Default::default() });
        assert_eq!(resolve("~/projects/app/src/main.rs"), "src/main.rs");
        assert_eq!(resolve("~/projects/app"), ".");
        assert_eq!(resolve("~/projects/app/"), ".");
        let under = |rest: &str| format!("~/projects/{rest}");
        assert_eq!(resolve(&under("app/a/../b")), "b");
        assert_eq!(resolve(&under("app/.git/hooks/pre-commit")), ".git/hooks/pre-commit");
        for (outside, why) in [("peer/x", "a sibling"), ("app-evil/x", "no bare string prefix"), ("app/../../.ssh/id_rsa", "an escape")] {
            assert_eq!(resolve(&under(outside)), under(outside), "{why} stays home-spelled");
        }
        assert_eq!(resolve("~/.ssh/id_rsa"), "~/.ssh/id_rsa");
        assert_eq!(resolve("~"), "~");
        assert_eq!(resolve("~bob/projects/app/x"), "~bob/projects/app/x", "another user's home is not ours");
    }

    #[test]
    fn a_home_spelled_path_needs_an_absolute_root() {
        let _none = enter(PathCtx { cwd: Some("/w".into()), root: None, ..Default::default() });
        assert_eq!(resolve("~/x"), "~/x");
        drop(_none);
        let _rel = enter(PathCtx { cwd: Some("/w".into()), root: Some("w".into()), ..Default::default() });
        assert_eq!(resolve("~/x"), "~/x");
    }

    #[test]
    fn a_home_rooted_workspace_leaves_home_paths_to_the_home_classifiers() {
        let Some(home) = std::env::var("HOME").ok().filter(|h| h.starts_with('/') && h.len() > 1) else {
            return;
        };
        let _g = enter(PathCtx { cwd: Some(home.clone()), root: Some(home.clone()), ..Default::default() });
        assert_eq!(resolve("~"), "~");
        assert_eq!(resolve("~/notes.txt"), "~/notes.txt");
        assert_eq!(resolve("notes.txt"), format!("{home}/notes.txt"));
        assert_eq!(resolve("."), home);
    }

    #[test]
    fn a_home_spelled_path_is_not_resolved_from_outside_the_root_or_through_a_glob() {
        let Some(home) = std::env::var("HOME").ok().filter(|h| h.starts_with('/') && h.len() > 1) else {
            return;
        };
        let root = format!("{home}/projects/app");
        let outside = enter(PathCtx { cwd: Some("/etc".into()), root: Some(root.clone()), ..Default::default() });
        assert_eq!(resolve("~/projects/app/x"), "~/projects/app/x", "a quoted `~` would name a directory under the cwd");
        drop(outside);
        let _g = enter(PathCtx { cwd: Some(root.clone()), root: Some(root), ..Default::default() });
        for glob in ["app/.ss?/id_rsa", "app/*"].map(|rest| format!("~/projects/{rest}")) {
            assert_eq!(resolve(&glob), glob);
        }
    }

    #[test]
    fn a_home_spelled_path_is_not_resolved_into_a_protected_root() {
        let Some(home) = std::env::var("HOME").ok().filter(|h| h.starts_with('/') && h.len() > 1) else {
            return;
        };
        let root = format!("{home}/.ssh");
        let _g = enter(PathCtx { cwd: Some(root.clone()), root: Some(root), ..Default::default() });
        assert_eq!(resolve("~/.ssh/id_rsa"), "~/.ssh/id_rsa");
    }

    #[test]
    fn a_root_holding_a_protected_place_keeps_every_path_absolute() {
        let _g = enter(PathCtx { cwd: Some("/".into()), root: Some("/".into()), ..Default::default() });
        assert_eq!(resolve("/etc/shadow"), "/etc/shadow");
        assert_eq!(resolve("etc/sudoers"), "/etc/sudoers");
        assert_eq!(resolve("etc"), "/etc");
        assert_eq!(resolve("/srv/app/x"), "/srv/app/x");
        assert_eq!(resolve("."), "/");
        drop(_g);
        let _etc = enter(PathCtx { cwd: Some("/etc".into()), root: Some("/etc".into()), ..Default::default() });
        assert_eq!(resolve("hosts"), "hosts", "an unprotected file in a root below every home is the worktree");
        assert_eq!(resolve("sudoers"), "/etc/sudoers");
        assert_eq!(resolve("."), "/etc", "the root itself is above a protected place");
        drop(_etc);
        let _inside = enter(PathCtx { cwd: Some("/root/app".into()), root: Some("/root/app".into()), ..Default::default() });
        assert_eq!(resolve("/root/app/x"), "x", "a root inside the protected place is where the user works");
    }

    #[test]
    fn loop_var_expands_to_its_representative_per_face() {
        let _g = enter_loop_var("f".into(), "read_item".into(), "write_item".into());
        assert_eq!(expand_vars("$f", false), "read_item");
        assert_eq!(expand_vars("$f", true), "write_item");
        assert_eq!(expand_vars("${f}", false), "read_item");
        assert_eq!(expand_vars("$f.bak", false), "read_item.bak", "compound suffix");
        assert_eq!(expand_vars("pre/$f", false), "pre/read_item");
        assert_eq!(expand_vars("$foo", false), "$foo", "$foo is not $f");
        assert_eq!(expand_vars("$g", false), "$g", "unbound var untouched");
        assert_eq!(expand_vars("plain", false), "plain");
    }

    #[test]
    fn loop_var_binding_is_scoped_and_nests() {
        assert_eq!(expand_vars("$f", false), "$f", "no binding");
        {
            let _outer = enter_loop_var("f".into(), "outer".into(), "outer".into());
            {
                let _inner = enter_loop_var("f".into(), "inner".into(), "inner".into());
                assert_eq!(expand_vars("$f", false), "inner", "innermost wins");
            }
            assert_eq!(expand_vars("$f", false), "outer", "inner popped on drop");
        }
        assert_eq!(expand_vars("$f", false), "$f", "all popped");
    }

    #[test]
    fn the_guard_restores_on_drop() {
        {
            let _g = enter(PathCtx { cwd: Some("/etc".into()), root: Some("/r".into()), ..Default::default() });
            assert_eq!(resolve("x"), "/etc/x");
        }
        assert_eq!(resolve("x"), "x", "context cleared after the guard drops");
    }

    const PROBE_VAR: &str = "SAFE_CHAINS_TMPDIR_PROBE_PATH";

    fn tmpdir_verdict(tmpdir: &str, path: &str) -> bool {
        let out = std::process::Command::new(std::env::current_exe().expect("test exe"))
            .args(["--exact", "pathctx::tests::tmpdir_probe", "--nocapture", "--test-threads=1"])
            .env("TMPDIR", tmpdir)
            .env(PROBE_VAR, path)
            .output()
            .expect("spawn probe");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "probe failed for TMPDIR={tmpdir:?} path={path:?}: {stdout}");
        match (stdout.contains("PROBE_VERDICT=true"), stdout.contains("PROBE_VERDICT=false")) {
            (true, false) => true,
            (false, true) => false,
            _ => panic!("probe did not run for TMPDIR={tmpdir:?} path={path:?}: {stdout}"),
        }
    }

    #[test]
    fn tmpdir_probe() {
        let Ok(path) = std::env::var(PROBE_VAR) else {
            return;
        };
        println!("PROBE_VERDICT={}", under_temp_root(&path));
    }

    #[test]
    fn a_tmpdir_root_counts_only_when_absolute_and_non_empty() {
        for (tmpdir, path, expected) in [
            ("/work/scratch", "/work/scratch/x.sh", true),
            ("/work/scratch/", "/work/scratch/x.sh", true),
            ("/work/scratch//", "/work/scratch/nested/x.sh", true),
            ("/work/scratch", "/work/scratch", false),
            ("/work/scratch", "/work/scratchy/x.sh", false),
            ("/work/scratch", "/work/other/x.sh", false),
            ("/work/scratch", "work/scratch/x.sh", false),
            ("", "/x.sh", false),
            ("", "/", false),
            ("/", "/x.sh", false),
            ("//", "/x.sh", false),
            ("scratch", "scratch/x.sh", false),
            ("scratch/", "scratch/x.sh", false),
            ("./scratch", "./scratch/x.sh", false),
            ("$SCRATCH", "$SCRATCH/x.sh", false),
            ("relative/dir", "relative/dir/x.sh", false),
        ] {
            assert_eq!(tmpdir_verdict(tmpdir, path), expected, "TMPDIR={tmpdir:?} path={path:?}");
        }
    }

    #[test]
    fn the_fixed_temp_roots_hold_whatever_tmpdir_says() {
        for tmpdir in ["", "scratch", "/work/scratch"] {
            for path in ["/tmp/x", "/private/tmp/x", "/var/tmp/x", "/private/var/tmp/x"] {
                assert!(tmpdir_verdict(tmpdir, path), "TMPDIR={tmpdir:?} path={path:?}");
            }
            assert!(!tmpdir_verdict(tmpdir, "/tmp"), "the bare root is not under itself");
        }
    }
}
