//! Where this process reads its home from.
//!
//! A release build reads `$HOME` and nothing else. The `test-home` feature stands up a private
//! home for an integration-test binary, so the suite does not read or write the home of whoever
//! ran it. An environment variable cannot select one.

use std::ffi::OsString;
use std::path::PathBuf;

#[cfg(feature = "test-home")]
use std::sync::OnceLock;

#[cfg(feature = "test-home")]
static TEST_HOME: OnceLock<PathBuf> = OnceLock::new();
#[cfg(feature = "test-home")]
static REAL_HOME: OnceLock<()> = OnceLock::new();

/// Use `home` for every home lookup in this process. A second call is ignored.
#[cfg(feature = "test-home")]
#[doc(hidden)]
pub fn install_test_home(home: PathBuf) {
    let _ = TEST_HOME.set(home);
}

/// The home this process classifies against.
#[doc(hidden)]
pub fn process_home_path() -> Option<PathBuf> {
    var_os().map(PathBuf::from)
}

pub(crate) fn var() -> Result<String, std::env::VarError> {
    #[cfg(feature = "test-home")]
    if let Some(home) = overridden() {
        return match home.to_str() {
            Some(text) => Ok(text.to_string()),
            None => Err(std::env::VarError::NotUnicode(home.as_os_str().to_os_string())),
        };
    }
    std::env::var("HOME")
}

pub(crate) fn var_os() -> Option<OsString> {
    #[cfg(feature = "test-home")]
    if let Some(home) = overridden() {
        return Some(home.as_os_str().to_os_string());
    }
    std::env::var_os("HOME")
}

#[cfg(feature = "test-home")]
fn overridden() -> Option<&'static PathBuf> {
    if let Some(home) = TEST_HOME.get() {
        return Some(home);
    }
    if REAL_HOME.get().is_some() {
        return None;
    }
    if integration_test_binary() {
        return Some(TEST_HOME.get_or_init(prepare_home));
    }
    let _ = REAL_HOME.set(());
    None
}

#[cfg(feature = "test-home")]
fn integration_test_binary() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let Some(name) = exe.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let name = name.strip_suffix(".exe").unwrap_or(name);
    if name.starts_with("safe-chains") || name.starts_with("safe_chains") || name.starts_with("gen-fuzz") || name.starts_with("gen_fuzz") {
        return false;
    }
    let Some((_, hash)) = name.rsplit_once('-') else {
        return false;
    };
    hash.len() >= 8 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(feature = "test-home")]
fn prepare_home() -> PathBuf {
    let path = fresh_dir();
    std::fs::create_dir_all(path.join(".ssh/keys")).expect("ssh dir");
    std::fs::write(path.join(".ssh/id_rsa"), "").expect("id_rsa");
    std::fs::write(path.join(".ssh/authorized_keys"), "").expect("authorized_keys");
    std::fs::create_dir_all(path.join(".config")).expect("config dir");
    std::fs::write(path.join(".config/safe-chains.toml"), "").expect("empty config");
    std::fs::create_dir_all(path.join("projects")).expect("projects");
    std::fs::write(path.join(".sc-isolated-home"), "").expect("marker");
    classifier_spelling(path.canonicalize().unwrap_or(path))
}

/// Drop a `/private` firmlink prefix. Classification folds `/private/var` to `/var` before it
/// matches `$HOME`, so the canonical spelling of this directory would miss that match.
#[cfg(feature = "test-home")]
fn classifier_spelling(path: PathBuf) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path;
    };
    let Some(rest) = text.strip_prefix("/private") else {
        return path;
    };
    if rest.starts_with("/var/") || rest.starts_with("/tmp/") || rest.starts_with("/etc/") { PathBuf::from(rest) } else { path }
}

#[cfg(feature = "test-home")]
const STALE_HOME_AGE: std::time::Duration = std::time::Duration::from_secs(6 * 3600);

#[cfg(feature = "test-home")]
fn sweep_stale_homes(base: &std::path::Path, older_than: std::time::Duration) {
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    for entry in entries.flatten() {
        let stale = entry.file_name().to_string_lossy().starts_with("sc-it-home-")
            && entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age > older_than);
        if stale {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

#[cfg(feature = "test-home")]
fn fresh_dir() -> PathBuf {
    sweep_stale_homes(&std::env::temp_dir(), STALE_HOME_AGE);
    let mut n = 0u32;
    loop {
        let path = std::env::temp_dir().join(format!("sc-it-home-{}-{n}", std::process::id()));
        if std::fs::create_dir(&path).is_ok() {
            return path;
        }
        n += 1;
        assert!(n < 1000, "could not create an isolated home");
    }
}

#[cfg(all(test, feature = "test-home"))]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn homes_left_by_earlier_runs_are_swept_and_a_current_one_is_not() {
        let base = tempfile::tempdir().expect("base");
        let old = base.path().join("sc-it-home-1-0");
        let current = base.path().join("sc-it-home-2-0");
        let other = base.path().join("unrelated-old");
        for dir in [&old, &current, &other] {
            std::fs::create_dir_all(dir.join("projects")).expect("dir");
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        super::sweep_stale_homes(base.path(), std::time::Duration::from_millis(10));
        assert!(!old.exists() && !current.exists(), "every sc-it-home-* older than the cutoff goes");
        assert!(other.exists(), "a folder without the prefix is left alone");
        let young = base.path().join("sc-it-home-3-0");
        std::fs::create_dir_all(&young).expect("dir");
        super::sweep_stale_homes(base.path(), std::time::Duration::from_secs(3600));
        assert!(young.exists(), "a recent home stays");
    }

    #[test]
    fn a_private_firmlink_is_stored_as_the_spelling_classification_folds_to() {
        assert_eq!(
            super::classifier_spelling(PathBuf::from("/private/var/folders/ab/cd/T/home")),
            PathBuf::from("/var/folders/ab/cd/T/home")
        );
        assert_eq!(super::classifier_spelling(PathBuf::from("/private/tmp/home")), PathBuf::from("/tmp/home"));
        let users = PathBuf::from("/Users/me");
        assert_eq!(super::classifier_spelling(users.clone()), users);
        let other = PathBuf::from("/private/Users/me");
        assert_eq!(super::classifier_spelling(other.clone()), other);
    }
}
