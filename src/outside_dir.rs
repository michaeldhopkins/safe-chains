//! A scratch directory outside every temporary root and outside `$HOME`.
//!
//! Tests that need one ask [`writable_outside`]. macOS tries the parent of `$TMPDIR`, then
//! `/Users/Shared`. Linux tries `$RUNNER_TOOL_CACHE`, then a directory under `/var/tmp`. A
//! candidate is kept only when [`crate::pathctx::under_temp_root`] does not claim it, it is
//! outside `$HOME`, and `safe-chains config grant` would accept it. Anything else, including a
//! device folder such as `/dev/shm`, is skipped. When nothing qualifies the function returns
//! `None` and the caller prints [`UNCOVERED`] and returns.

use std::path::{Path, PathBuf};

/// What a caller prints, then returns, when [`writable_outside`] finds nothing.
pub const UNCOVERED: &str = "uncovered: no writable folder outside the temporary root and the home";

/// A writable directory outside every temporary root, outside `$HOME`, and not a path
/// `safe-chains config grant` refuses.
#[must_use]
pub fn writable_outside() -> Option<tempfile::TempDir> {
    let home = match read_home() {
        Home::Unreadable => return None,
        Home::Unset => None,
        Home::Set(value) => Some(value),
    };
    let tmpdir = env_path("TMPDIR");
    let cache = env_path("RUNNER_TOOL_CACHE");
    for parent in candidate_parents(host(), tmpdir.as_deref(), cache.as_deref()) {
        if let Some(dir) = dir_in(&parent, home.as_deref()) {
            return Some(dir);
        }
    }
    None
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Host {
    Macos,
    Linux,
    Other,
}

enum Home {
    Unset,
    Unreadable,
    Set(String),
}

fn host() -> Host {
    if cfg!(target_os = "macos") {
        Host::Macos
    } else if cfg!(target_os = "linux") {
        Host::Linux
    } else {
        Host::Other
    }
}

fn read_home() -> Home {
    match std::env::var("HOME") {
        Err(std::env::VarError::NotPresent) => Home::Unset,
        Err(std::env::VarError::NotUnicode(_)) => Home::Unreadable,
        Ok(value) => {
            let value = value.trim_end_matches('/');
            if value.is_empty() { Home::Unset } else { Home::Set(value.to_string()) }
        }
    }
}

fn env_path(key: &str) -> Option<PathBuf> {
    let raw = std::env::var_os(key)?;
    if raw.is_empty() { None } else { Some(PathBuf::from(raw)) }
}

fn candidate_parents(host: Host, tmpdir: Option<&Path>, tool_cache: Option<&Path>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    match host {
        Host::Macos => {
            if let Some(parent) = tmpdir_parent(tmpdir) {
                out.push(parent);
            }
            out.push(PathBuf::from("/Users/Shared"));
        }
        Host::Linux => {
            if let Some(cache) = tool_cache.filter(|path| !path.as_os_str().is_empty()) {
                out.push(cache.to_path_buf());
            }
            out.push(PathBuf::from("/var/tmp"));
        }
        Host::Other => {}
    }
    out
}

fn tmpdir_parent(tmpdir: Option<&Path>) -> Option<PathBuf> {
    let parent = tmpdir?.parent()?;
    if parent.as_os_str().is_empty() || parent == Path::new("/") { None } else { Some(parent.to_path_buf()) }
}

fn dir_in(parent: &Path, home: Option<&str>) -> Option<tempfile::TempDir> {
    if parent.as_os_str().is_empty() || parent == Path::new("/") {
        return None;
    }
    std::fs::create_dir_all(parent).ok()?;
    let dir = tempfile::Builder::new().prefix("sc-outside-").tempdir_in(parent).ok()?;
    qualifies(dir.path(), home).then_some(dir)
}

fn qualifies(path: &Path, home: Option<&str>) -> bool {
    let Some(spelled) = path.to_str() else {
        return false;
    };
    if !spelling_ok(spelled, home) {
        return false;
    }
    let Ok(canon_path) = path.canonicalize() else {
        return false;
    };
    let Some(canon) = canon_path.to_str() else {
        return false;
    };
    (canon == spelled || spelling_ok(canon, home)) && !blocked_by_canonical_home(canon, home)
}

fn spelling_ok(path: &str, home: Option<&str>) -> bool {
    path.starts_with('/') && !path_under_home(path, home) && !inside_temp(path) && !crate::userconfig::grant_refused(path)
}

fn inside_temp(path: &str) -> bool {
    let path = path.trim_end_matches('/');
    !path.is_empty() && (crate::pathctx::under_temp_root(path) || crate::pathctx::under_temp_root(&format!("{path}/x")))
}

fn path_under_home(path: &str, home: Option<&str>) -> bool {
    let Some(home) = home.map(|value| value.trim_end_matches('/')).filter(|value| !value.is_empty()) else {
        return false;
    };
    if home == "/" {
        return path.starts_with('/');
    }
    path == home || path.starts_with(&format!("{home}/"))
}

fn blocked_by_canonical_home(path: &str, home: Option<&str>) -> bool {
    let Some(home) = home else {
        return false;
    };
    let Ok(real) = Path::new(home).canonicalize() else {
        return false;
    };
    let Some(real) = real.to_str() else {
        return true;
    };
    let real = real.trim_end_matches('/');
    if real == home.trim_end_matches('/') {
        return false;
    }
    path_under_home(path, Some(real))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_follow_the_host() {
        let tmp = Path::new("/var/folders/xx/yy/T");
        let cache = Path::new("/opt/hostedtoolcache");
        assert_eq!(
            candidate_parents(Host::Macos, Some(tmp), Some(cache)),
            vec![PathBuf::from("/var/folders/xx/yy"), PathBuf::from("/Users/Shared")]
        );
        assert_eq!(
            candidate_parents(Host::Macos, Some(Path::new("/var/folders/xx/yy/T/")), None),
            vec![PathBuf::from("/var/folders/xx/yy"), PathBuf::from("/Users/Shared")]
        );
        assert_eq!(candidate_parents(Host::Macos, None, Some(cache)), vec![PathBuf::from("/Users/Shared")]);
        assert_eq!(candidate_parents(Host::Macos, Some(Path::new("/tmp")), None), vec![PathBuf::from("/Users/Shared")]);
        assert_eq!(
            candidate_parents(Host::Linux, Some(Path::new("/tmp")), Some(cache)),
            vec![PathBuf::from("/opt/hostedtoolcache"), PathBuf::from("/var/tmp")]
        );
        assert_eq!(candidate_parents(Host::Linux, None, None), vec![PathBuf::from("/var/tmp")]);
        assert_eq!(candidate_parents(Host::Linux, None, Some(Path::new(""))), vec![PathBuf::from("/var/tmp")]);
        assert!(candidate_parents(Host::Other, Some(tmp), Some(cache)).is_empty());
    }

    #[test]
    fn a_device_folder_a_temp_root_and_home_do_not_qualify() {
        assert!(!spelling_ok("/dev/shm", None));
        assert!(!spelling_ok("/dev/shm/scratch", Some("/home/runner")));
        assert!(!spelling_ok("/var/tmp/scratch", Some("/home/runner")));
        assert!(!spelling_ok("/private/var/tmp/scratch", None));
        assert!(!spelling_ok("/tmp/scratch", None));
        assert!(!spelling_ok("/private/tmp/scratch", None));
        assert!(!spelling_ok("/Users/me/scratch", Some("/Users/me")));
        assert!(!spelling_ok("/Users/Shared/scratch", Some("/Users/Shared")));
        assert!(!spelling_ok("/Users/Shared/scratch", Some("/")));
        assert!(!spelling_ok("relative/scratch", Some("/home/runner")));
        // `Shared` is another user unless this process is that user, so a grant refuses the folder.
        if home_user() != "Shared" {
            assert!(!spelling_ok("/Users/Shared/scratch", Some("/Users/me")));
            assert!(!spelling_ok("/Users/Shared/scratch", None));
        }
        assert!(spelling_ok("/opt/hostedtoolcache/scratch", Some("/home/runner")));
    }

    fn home_user() -> String {
        crate::pathctx::home_dir().unwrap_or_default().rsplit('/').next().unwrap_or("").to_string()
    }

    #[test]
    fn a_folder_under_var_tmp_does_not_qualify() {
        let Ok(dir) = tempfile::Builder::new().prefix("sc-outside-").tempdir_in("/var/tmp") else {
            eprintln!("{UNCOVERED}");
            return;
        };
        assert!(!qualifies(dir.path(), None), "{}", dir.path().display());
    }

    #[test]
    fn a_writable_shared_users_folder_follows_the_gate() {
        if !Path::new("/Users/Shared").is_dir() {
            return;
        }
        let Ok(dir) = tempfile::Builder::new().prefix("sc-outside-").tempdir_in("/Users/Shared") else {
            eprintln!("{UNCOVERED}");
            return;
        };
        let home = Some("/Users/me");
        let spelled_ok = dir.path().to_str().is_some_and(|path| spelling_ok(path, home));
        assert_eq!(qualifies(dir.path(), home), spelled_ok, "{}", dir.path().display());
        if home_user() != "Shared" {
            assert!(!spelled_ok, "a grant accepted {}", dir.path().display());
        }
    }

    #[test]
    fn the_parent_of_tmpdir_qualifies_when_it_is_writable() {
        let Ok(raw) = std::env::var("TMPDIR") else {
            eprintln!("{UNCOVERED}");
            return;
        };
        let Some(parent) = tmpdir_parent(Some(Path::new(&raw))) else {
            eprintln!("{UNCOVERED}");
            return;
        };
        let Ok(dir) = tempfile::Builder::new().prefix("sc-outside-").tempdir_in(&parent) else {
            eprintln!("{UNCOVERED}");
            return;
        };
        let home = std::env::var("HOME").ok().map(|value| value.trim_end_matches('/').to_string());
        assert!(qualifies(dir.path(), home.as_deref()), "{}", dir.path().display());
    }

    #[test]
    fn writable_outside_with_tmpdir_and_home_unset() {
        let exe = std::env::current_exe().expect("test exe");
        let out = std::process::Command::new(exe)
            .args(["--exact", "outside_dir::tests::probe_with_tmpdir_and_home_unset", "--nocapture", "--test-threads=1"])
            .env_remove("TMPDIR")
            .env_remove("HOME")
            .env("SAFE_CHAINS_OUTSIDE_DIR_PROBE", "1")
            .output()
            .expect("spawn");
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "probe failed: {stdout}{stderr}");
        assert!(stdout.contains("PROBE_OK="), "probe did not run: {stdout}{stderr}");
    }

    #[test]
    fn probe_with_tmpdir_and_home_unset() {
        if std::env::var_os("SAFE_CHAINS_OUTSIDE_DIR_PROBE").is_none() {
            return;
        }
        assert!(std::env::var_os("TMPDIR").is_none(), "TMPDIR is set");
        assert!(std::env::var_os("HOME").is_none(), "HOME is set");
        match writable_outside() {
            Some(dir) => {
                assert!(qualifies(dir.path(), None), "{}", dir.path().display());
                println!("PROBE_OK=some {}", dir.path().display());
            }
            None => println!("PROBE_OK=none"),
        }
    }
}
