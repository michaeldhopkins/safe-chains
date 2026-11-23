//! Path arithmetic for the directory context: joining, home expansion and expressing a path
//! against the project root, all lexical (no filesystem access).

use std::borrow::Cow;

#[cfg(test)]
use std::cell::RefCell;

#[cfg(test)]
thread_local! {
    static HOME_OVERRIDE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// `$HOME` when it is absolute. A unit test can stand a fixture in for the process home, so a
/// listing of `~/Library` or `/Users` does not open the real one. Production reads the environment
/// only.
pub(crate) fn home_dir() -> Option<String> {
    #[cfg(test)]
    if let Some(home) = HOME_OVERRIDE.with(|slot| slot.borrow().clone()).filter(|home| home.starts_with('/')) {
        return Some(home);
    }
    std::env::var("HOME").ok().filter(|home| home.starts_with('/'))
}

/// Install `home` as [`home_dir`] until the guard drops. The process `$HOME` stays what a listing
/// guard compares against, so the fixture is not mistaken for the real one.
#[cfg(test)]
#[must_use]
pub(crate) fn enter_home(home: String) -> HomeGuard {
    HomeGuard(HOME_OVERRIDE.with(|slot| slot.replace(Some(home))))
}

#[cfg(test)]
pub(crate) struct HomeGuard(Option<String>);

#[cfg(test)]
impl Drop for HomeGuard {
    fn drop(&mut self) {
        let previous = self.0.take();
        HOME_OVERRIDE.with(|slot| *slot.borrow_mut() = previous);
    }
}

/// Whether listing `path` would open the process home, or a directory under `/Users` or `/home`.
///
/// `open` of a TCC-protected folder (`~/Library/Keychains`, `~/Desktop`, another user's home)
/// waits for a consent prompt. A launchd job has no prompt, so the open never returns. The
/// process home is the one in the environment, not a fixture installed with [`enter_home`].
#[cfg(test)]
pub(crate) fn walks_real_home(path: &str, real_home: Option<&str>) -> bool {
    let path = normalize_listing(path);
    if let Some(home) = real_home.map(normalize_listing).filter(|home| home.len() > 1)
        && (path == home || path.starts_with(&format!("{home}/")))
    {
        return true;
    }
    const ROOTS: &[&str] = &["/Users", "/home", "/System/Volumes/Data/Users", "/System/Volumes/Data/home"];
    ROOTS.iter().any(|root| listing_under(&path, root))
}

#[cfg(test)]
fn listing_under(path: &str, root: &str) -> bool {
    let (path, root) = if cfg!(target_os = "macos") {
        (path.to_ascii_lowercase(), root.to_ascii_lowercase())
    } else {
        (path.to_string(), root.to_string())
    };
    path == root || path.starts_with(&format!("{root}/"))
}

#[cfg(test)]
fn normalize_listing(path: &str) -> String {
    let mut out = String::new();
    for seg in path.split('/').filter(|seg| !seg.is_empty() && *seg != ".") {
        out.push('/');
        out.push_str(seg);
    }
    if out.is_empty() { "/".into() } else { out }
}

/// Fail the current test when listing `path` would open a directory under the real home, `/Users`
/// or `/home`. A path that is not there is not such a directory: `open` returns at once. A path
/// that is there, or that cannot be stated, is refused before `open`, which is what blocks.
/// Production calls nothing.
#[cfg(test)]
pub(crate) fn guard_listing(path: &str) {
    let real = std::env::var("HOME").ok();
    if !walks_real_home(path, real.as_deref()) {
        return;
    }
    if std::fs::symlink_metadata(path).is_err_and(|err| err.kind() == std::io::ErrorKind::NotFound) {
        return;
    }
    panic!("a unit test listed {path}, under the real home or /Users or /home");
}

/// `~` / `~/rest` expanded against `$HOME`. `~user` is left alone (we cannot resolve another user's
/// home); `None` only when `~` is used and `$HOME` is unusable.
pub(super) fn expand_home(target: &str) -> Option<Cow<'_, str>> {
    let rest = match target {
        "~" => "",
        t => match t.strip_prefix("~/") {
            Some(r) => r,
            None => return Some(Cow::Borrowed(target)),
        },
    };
    let home = home_dir()?;
    Some(Cow::Owned(if rest.is_empty() { home } else { format!("{home}/{rest}") }))
}

/// Express an absolute `abs` path relative to `root`: `.` if it IS the root, a root-relative path
/// if it's inside (classified as worktree), or the absolute path unchanged if it escaped (classified
/// as machine/etc.). The `inside.starts_with('/')` guard prevents a sibling like `/proj-evil` from
/// matching root `/proj` by bare string prefix. A path at, inside or above a protected
/// place the root contains stays absolute (`regions::keeps_absolute`), as from any workspace.
pub(super) fn express_relative_to_root(abs: &str, root: &str) -> String {
    let root = root.trim_end_matches('/');
    if crate::engine::resolve::regions::keeps_absolute(abs, root) {
        return abs.to_string();
    }
    if abs == root {
        return ".".to_string(); // the project root itself
    }
    match abs.strip_prefix(root) {
        Some(inside) if inside.starts_with('/') => inside.trim_start_matches('/').to_string(),
        _ => abs.to_string(),
    }
}

/// Join a relative path onto an absolute base, resolving `.` and `..` purely lexically. A
/// `..` that would climb above `/` is clamped there. A `..` that would climb out of a substitution
/// placeholder is unresolved, since nobody knows what the placeholder stood for.
pub(super) fn lexical_join(base: &str, rel: &str) -> String {
    let mut parts: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
    for seg in rel.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if parts
                    .pop()
                    .is_some_and(|part| part.contains(crate::cst::eval::TAGGED_PREFIX) || part == "safe-chains-unknown-workdir")
                {
                    return super::UNRESOLVED_CWD.to_string();
                }
            }
            s => parts.push(s),
        }
    }
    format!("/{}", parts.join("/"))
}

/// Whether a `cd` target names a place we cannot pin: another user's home, a variable, an
/// undeclared substitution, or a climb out of a declared one, whose locus says nothing about what
/// lies above it.
pub(super) fn names_no_known_directory(expanded: &str) -> bool {
    let climbs_out_of_a_substitution = expanded.contains(crate::cst::eval::TAGGED_PREFIX) && expanded.split('/').any(|part| part == "..");
    expanded.starts_with('~') || expanded.contains('$') || crate::cst::check::is_opaque_value(expanded) || climbs_out_of_a_substitution
}
