//! Path arithmetic for the directory context: joining, home expansion and expressing a path
//! against the project root, all lexical (no filesystem access).

use std::borrow::Cow;

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
    let home = std::env::var("HOME").ok().filter(|h| h.starts_with('/'))?;
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
/// `..` that would climb above `/` is clamped there.
pub(super) fn lexical_join(base: &str, rel: &str) -> String {
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
