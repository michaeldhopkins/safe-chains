//! Extra workspace roots named in the user config (`workspace_roots` in
//! `~/.config/safe-chains.toml`). A listed path is a root the way `~/projects` is: the path itself
//! when it has no dot component, and every plain descendant. A dot component that is part of the
//! listed path does not disqualify a descendant, which is how `~/.worktrees/<name>` works.

#[cfg(test)]
use std::cell::RefCell;
#[cfg(not(test))]
use std::sync::LazyLock;

#[derive(serde::Deserialize)]
struct File {
    #[serde(default)]
    workspace_roots: Vec<String>,
}

#[cfg(test)]
thread_local! {
    static TEST_ROOTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// The longest listed root that covers `path`, and the remainder below it.
/// An empty remainder means `path` is the root itself.
fn best_cover(path: &str, home: Option<&str>, roots: &[String]) -> Option<(String, String)> {
    let path = fold_private(path.trim_end_matches('/'));
    let mut best: Option<(String, String)> = None;
    for root in roots {
        let Some(root) = expand(root, home) else { continue };
        let rest = if path == root {
            String::new()
        } else {
            let Some(rest) = path.strip_prefix(&root).filter(|r| r.starts_with('/')) else { continue };
            rest[1..].to_string()
        };
        if best.as_ref().is_none_or(|(kept, _)| root.len() > kept.len()) {
            best = Some((root, rest));
        }
    }
    best
}

/// The remainder of `path` below the longest listed root that covers it, or `None` when none does.
pub(super) fn covers(path: &str, home: Option<&str>, roots: &[String]) -> Option<String> {
    best_cover(path, home, roots).map(|(_, rest)| rest)
}

/// Whether a listed root covers `path`, and whether a dot component then keeps it from being one.
/// `None` when no listed root covers `path`, so the caller keeps its own rule.
pub(super) fn dot_disqualifies(path: &str, home: Option<&str>) -> Option<bool> {
    disqualified(path, home, &current())
}

fn disqualified(path: &str, home: Option<&str>, roots: &[String]) -> Option<bool> {
    best_cover(path, home, roots).map(|(root, rest)| if rest.is_empty() { root_has_dot(&root, home) } else { has_dot(&rest) })
}

/// A dot in the listed root disqualifies the root itself. A dot above it, such as a temporary
/// directory the home sits in, does not: that dot is not part of the root.
fn root_has_dot(root_abs: &str, home: Option<&str>) -> bool {
    let rel = match home.map(|h| fold_private(h.trim_end_matches('/'))) {
        Some(home) if root_abs == home || root_abs.starts_with(&format!("{home}/")) => {
            root_abs.strip_prefix(&home).and_then(|r| r.strip_prefix('/')).unwrap_or("")
        }
        _ => root_abs.trim_start_matches('/'),
    };
    has_dot(rel)
}

pub(super) fn listed(path: &str, home: Option<&str>) -> bool {
    covers(path, home, &current()).is_some()
}

fn has_dot(path: &str) -> bool {
    path.split('/').any(|c| c.starts_with('.'))
}

fn current() -> Vec<String> {
    #[cfg(test)]
    {
        TEST_ROOTS.with(|roots| roots.borrow().clone())
    }
    #[cfg(not(test))]
    {
        USER_ROOTS.clone()
    }
}

/// Run `f` with `roots` as the configured workspace roots. Gone when `f` returns, including by panic.
#[cfg(test)]
pub(super) fn with_workspace_roots<T>(roots: &[&str], f: impl FnOnce() -> T) -> T {
    struct Clear;
    impl Drop for Clear {
        fn drop(&mut self) {
            TEST_ROOTS.with(|roots| roots.borrow_mut().clear());
        }
    }
    TEST_ROOTS.with(|slot| *slot.borrow_mut() = roots.iter().map(|s| (*s).to_string()).collect());
    let _clear = Clear;
    f()
}

fn expand(root: &str, home: Option<&str>) -> Option<String> {
    let root = root.trim().trim_end_matches('/');
    if root.is_empty() || root.contains(['*', '?', '$']) || root.split('/').any(|c| c == "." || c == "..") {
        return None;
    }
    let abs = if let Some(rest) = root.strip_prefix("~/") {
        if rest.is_empty() {
            return None;
        }
        let home = home.filter(|h| h.starts_with('/') && h.len() > 1)?.trim_end_matches('/');
        format!("{home}/{rest}")
    } else if root.starts_with('/') {
        root.to_string()
    } else {
        return None;
    };
    let abs = fold_private(abs.trim_end_matches('/'));
    (!too_broad(&abs, home)).then_some(abs)
}

fn too_broad(abs: &str, home: Option<&str>) -> bool {
    if matches!(abs, "/" | "/Users" | "/home" | "/private" | "") {
        return true;
    }
    home.map(|h| fold_private(h.trim_end_matches('/'))).is_some_and(|h| abs == h)
}

fn fold_private(path: &str) -> String {
    for firm in ["/etc", "/var", "/tmp"] {
        let private = format!("/private{firm}");
        if path == private || path.starts_with(&format!("{private}/")) {
            return path["/private".len()..].to_string();
        }
    }
    path.to_string()
}

#[cfg(not(test))]
fn load() -> Vec<String> {
    if std::env::var_os("SAFE_CHAINS_NO_LOCAL").is_some() {
        return Vec::new();
    }
    let Some(home) = std::env::var_os("HOME") else {
        return Vec::new();
    };
    let Ok(source) = std::fs::read_to_string(std::path::PathBuf::from(home).join(".config/safe-chains.toml")) else {
        return Vec::new();
    };
    parse(&source)
}

pub(super) fn parse(source: &str) -> Vec<String> {
    toml::from_str::<File>(source).map(|file| file.workspace_roots).unwrap_or_default()
}

#[cfg(not(test))]
static USER_ROOTS: LazyLock<Vec<String>> = LazyLock::new(load);

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn a_dot_above_the_home_does_not_disqualify_the_listed_root() {
        let home = "/var/folders/.tmp/home";
        let roots = ["~/agent-workspaces".to_string(), "~/work/.agents".to_string()];
        assert_eq!(super::disqualified(&format!("{home}/agent-workspaces"), Some(home), &roots), Some(false));
        assert_eq!(super::disqualified(&format!("{home}/work/.agents"), Some(home), &roots), Some(true));
        assert_eq!(super::disqualified(&format!("{home}/work/.agents/agent-1"), Some(home), &roots), Some(false));
    }

    #[test]
    fn the_file_lists_roots_and_a_broken_value_lists_none() {
        assert!(parse("").is_empty());
        assert!(parse("[[grant]]\npath = \"~/projects\"\nread = true\n").is_empty());
        assert_eq!(
            parse("workspace_roots = [\"~/agent-workspaces\", \"~/scripts\"]\n[[grant]]\npath = \"/opt/one\"\nread = true\n"),
            vec!["~/agent-workspaces".to_string(), "~/scripts".to_string()]
        );
        assert!(parse("workspace_roots = \"~/scripts\"\n").is_empty());
        assert!(parse("not toml {{{").is_empty());
    }

    #[test]
    fn a_root_of_the_home_or_the_filesystem_covers_nothing() {
        let home = "/srv/accounts/dev";
        let path = format!("{home}/agent-workspaces/agent-1");
        for root in ["/", "~", "~/", "~/..", "/home", "/Users", "/private", home, "~/../.ssh", "agent-workspaces", "$HOME/scripts"] {
            assert!(covers(&path, Some(home), &[root.to_string()]).is_none(), "{root}");
        }
    }

    #[test]
    fn an_absolute_root_covers_its_descendants_on_either_spelling_of_a_firmlink() {
        let roots = vec!["/tmp/agent-workspaces".to_string()];
        assert_eq!(covers("/tmp/agent-workspaces/agent-1", None, &roots).as_deref(), Some("agent-1"));
        assert_eq!(covers("/private/tmp/agent-workspaces/agent-1", None, &roots).as_deref(), Some("agent-1"));
        assert!(covers("/tmp/agent-workspaces-evil/agent-1", None, &roots).is_none());
    }

    #[test]
    fn the_longest_listed_root_decides_a_dot_inside_it() {
        let home = "/srv/accounts/dev";
        let roots = vec!["~/work".to_string(), "~/work/.agents".to_string()];
        let child = format!("{home}/work/.agents/agent-1");
        let other = format!("{home}/work/.other/agent-1");
        assert_eq!(disqualified(&child, Some(home), &roots), Some(false), "the longer root contains the dot");
        assert_eq!(disqualified(&other, Some(home), &roots), Some(true), "only ~/work covers this, and .other is below it");
        assert_eq!(disqualified(&format!("{home}/work/.agents"), Some(home), &roots), Some(true));
    }

    #[test]
    fn a_folder_is_a_workspace_root_only_while_the_config_names_it() {
        let home = std::env::var("HOME").expect("HOME");
        let agent = format!("{home}/agent-workspaces/agent-1");
        let scripts = format!("{home}/scripts/job");
        let dotted = format!("{home}/work/.agents/agent-1");
        let parent = format!("{home}/work/.agents");
        let hidden = format!("{home}/agent-workspaces/agent-1/.git");
        assert!(!super::super::is_workspace_root(&agent));
        assert!(!super::super::is_workspace_root(&scripts));
        assert!(!super::super::is_workspace_root(&dotted));
        assert!(super::super::is_workspace_root(&format!("{home}/projects/app")));
        assert!(super::super::is_workspace_root(&format!("{home}/.worktrees/wt-1")));
        assert!(!super::super::is_workspace_root(&format!("{home}/.worktrees")));
        with_workspace_roots(&["~/agent-workspaces", "~/scripts", "~/work/.agents"], || {
            assert!(super::super::is_workspace_root(&agent));
            assert!(super::super::is_workspace_root(&format!("{home}/agent-workspaces")));
            assert!(super::super::is_workspace_root(&scripts));
            assert!(super::super::is_workspace_root(&format!("{home}/scripts")));
            assert!(super::super::is_workspace_root(&dotted));
            assert!(!super::super::is_workspace_root(&parent), "a listed root with a dot component is not itself a root");
            assert!(!super::super::is_workspace_root(&hidden));
            assert!(!super::super::is_workspace_root(&format!("{home}/scripts/job/.hidden")));
        });
        assert!(!super::super::is_workspace_root(&agent), "the roots outlived the question");
        assert!(!super::super::is_workspace_root(&scripts));
        assert!(!super::super::is_workspace_root(&dotted));
    }

    proptest! {
        #[test]
        fn a_configured_root_covers_itself_and_its_plain_descendants_only(
            root in prop::collection::vec("[a-z]{1,6}", 1..3),
            child in prop::collection::vec("[a-z]{1,6}", 1..3),
            extra in "[a-z]{1,6}",
        ) {
            let home = "/srv/accounts/dev";
            let root_rel = root.join("/");
            let child_rel = child.join("/");
            let roots = vec![format!("~/{root_rel}")];
            let at = format!("{home}/{root_rel}");
            let below = format!("{at}/{child_rel}");
            let at_rest = covers(&at, Some(home), &roots);
            let below_rest = covers(&below, Some(home), &roots);
            prop_assert_eq!(at_rest.as_deref(), Some(""));
            prop_assert_eq!(below_rest.as_deref(), Some(child_rel.as_str()));
            let neighbour = format!("{at}{extra}");
            prop_assert!(covers(&neighbour, Some(home), &roots).is_none());
            prop_assert_eq!(disqualified(&at, Some(home), &roots), Some(false));
            prop_assert_eq!(disqualified(&below, Some(home), &roots), Some(false));
            let hidden = format!("{below}/.git");
            prop_assert_eq!(disqualified(&hidden, Some(home), &roots), Some(true));
            prop_assert!(covers(&below, Some(home), &[]).is_none());
        }

        #[test]
        fn a_dot_in_the_listed_root_disqualifies_the_root_itself_and_not_a_plain_child(
            head in "[a-z]{1,6}",
            dot in "[a-z]{1,6}",
            child in "[a-z]{1,6}",
        ) {
            let home = "/srv/accounts/dev";
            let roots = vec![format!("~/{head}/.{dot}")];
            let parent = format!("{home}/{head}/.{dot}");
            let below = format!("{parent}/{child}");
            prop_assert_eq!(disqualified(&parent, Some(home), &roots), Some(true));
            prop_assert_eq!(disqualified(&below, Some(home), &roots), Some(false));
            let hidden = format!("{below}/.git");
            prop_assert_eq!(disqualified(&hidden, Some(home), &roots), Some(true));
            prop_assert!(covers(&below, Some(home), &[]).is_none());
        }
    }
}
