//! A scratch directory outside every temporary root and outside `$HOME`.
//!
//! Set `SAFE_CHAINS_TEST_OUTSIDE_DIR` to a folder outside every temporary root, outside
//! `$HOME`, and one that `safe-chains config grant` would accept. [`writable_outside`] creates
//! a directory there. When the variable is unset or the folder does not qualify, it returns
//! `None`. Call [`uncovered`]: it prints [`UNCOVERED`], and it fails the test when
//! `SAFE_CHAINS_REQUIRE_OUTSIDE` is set.

use std::path::{Path, PathBuf};

/// What [`uncovered`] prints when [`writable_outside`] finds nothing.
pub const UNCOVERED: &str = "uncovered: no writable folder outside the temporary root and the home";

/// A writable directory inside `SAFE_CHAINS_TEST_OUTSIDE_DIR`, when that folder qualifies.
#[must_use]
pub fn writable_outside() -> Option<tempfile::TempDir> {
    let parent = env_path("SAFE_CHAINS_TEST_OUTSIDE_DIR")?;
    let home = match read_home() {
        Home::Unreadable => return None,
        Home::Unset => None,
        Home::Set(value) => Some(value),
    };
    dir_in(&parent, home.as_deref())
}

/// Print [`UNCOVERED`]. Fail the test when `SAFE_CHAINS_REQUIRE_OUTSIDE` is set.
#[track_caller]
pub fn uncovered() {
    eprintln!("{UNCOVERED}");
    if std::env::var_os("SAFE_CHAINS_REQUIRE_OUTSIDE").is_some() {
        panic!("{UNCOVERED}");
    }
}

enum Home {
    Unset,
    Unreadable,
    Set(String),
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

fn dir_in(parent: &Path, home: Option<&str>) -> Option<tempfile::TempDir> {
    if parent.as_os_str().is_empty() || parent == Path::new("/") {
        return None;
    }
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
    !path.is_empty() && crate::pathctx::under_temp(path)
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
    fn a_temp_root_the_home_and_a_refused_grant_do_not_qualify() {
        for path in [
            "/tmp/scratch", "/private/tmp/scratch", "/var/tmp/scratch", "/private/var/tmp/scratch", "/var/folders/xx/scratch",
            "/private/var/folders/xx/scratch", "/dev/shm", "/dev/shm/scratch", "/usr/bin/scratch", "relative/scratch",
        ] {
            assert!(!spelling_ok(path, Some("/home/runner")), "{path}");
        }
        assert!(!spelling_ok("/Users/me/scratch", Some("/Users/me")));
        assert!(!spelling_ok("/opt/hostedtoolcache/scratch", Some("/opt/hostedtoolcache")));
        assert!(!spelling_ok("/Users/Shared/scratch", Some("/")));
        if home_user() != "Shared" {
            assert!(!spelling_ok("/Users/Shared/scratch", Some("/Users/me")));
        }
        assert!(spelling_ok("/opt/hostedtoolcache/scratch", Some("/home/runner")));
        assert!(spelling_ok("/opt/hostedtoolcache/safe-chains-outside/x", Some("/home/runner")));
        assert!(spelling_ok("/opt/safe-chains-outside/x", Some("/home/runner")));
        assert!(spelling_ok("/usr/local/safe-chains-outside/x", Some("/Users/runner")));
        for path in [
            "/VAR/folders/xx/yy/T/x",
            "/var/folders/xx/yy/C/cache",
            "/var/folders/xx/yy/0/state",
            "/var/folders/xx/yy/X/extra",
            "/System/Volumes/Data/tmp/x",
            "/System/Volumes/Data/private/var/folders/xx/yy/C/cache",
        ] {
            assert!(!spelling_ok(path, Some("/home/runner")), "{path}");
        }
    }

    fn home_user() -> String {
        crate::pathctx::home_dir().unwrap_or_default().rsplit('/').next().unwrap_or("").to_string()
    }

    #[test]
    fn the_variable_is_the_only_folder() {
        let missing = run_probe("outside_dir::tests::probe_outside", None, false);
        assert!(text_of(&missing).contains("PROBE_OK=none"), "{}", text_of(&missing));
        assert!(missing.status.success(), "{}", text_of(&missing));

        let mut dirs = vec!["/tmp".to_string(), "/var/tmp".to_string(), "/var/folders".to_string(), "relative".to_string()];
        if let Ok(home) = std::env::var("HOME")
            && !home.is_empty()
            && home != "/"
        {
            dirs.push(home);
        }
        for dir in dirs {
            let out = run_probe("outside_dir::tests::probe_outside", Some(&dir), false);
            assert!(out.status.success(), "{dir}: {}", text_of(&out));
            assert!(text_of(&out).contains("PROBE_OK=none"), "{dir}: {}", text_of(&out));
        }
    }

    #[test]
    fn uncovered_fails_only_when_required() {
        let skipped = run_probe("outside_dir::tests::probe_uncovered", None, false);
        let skipped_text = text_of(&skipped);
        assert!(skipped.status.success(), "{skipped_text}");
        assert!(skipped_text.contains("UNCOVERED_RETURNED"), "{skipped_text}");
        assert!(skipped_text.contains(UNCOVERED), "{skipped_text}");

        let required = run_probe("outside_dir::tests::probe_uncovered", None, true);
        let required_text = text_of(&required);
        assert!(!required.status.success(), "{required_text}");
        assert!(!required_text.contains("UNCOVERED_RETURNED"), "{required_text}");
        assert!(required_text.contains(UNCOVERED), "{required_text}");
    }

    #[test]
    fn the_configured_folder_qualifies() {
        let Some(dir) = writable_outside() else {
            uncovered();
            return;
        };
        let home = match read_home() {
            Home::Set(value) => Some(value),
            Home::Unset | Home::Unreadable => None,
        };
        assert!(qualifies(dir.path(), home.as_deref()), "{}", dir.path().display());
        let configured = std::env::var("SAFE_CHAINS_TEST_OUTSIDE_DIR").expect("variable");
        let parent = Path::new(configured.trim_end_matches('/'));
        assert!(dir.path().starts_with(parent), "{} is not under {configured}", dir.path().display());
    }

    #[test]
    fn qualification_checks_both_spellings_and_canonical_home() {
        let target_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
        std::fs::create_dir_all(&target_root).expect("target dir");
        let holder = tempfile::tempdir_in(&target_root).expect("holder");
        let base = holder.path().canonicalize().expect("canonical");
        let outside = base.join("outside");
        let child = outside.join("child");
        std::fs::create_dir_all(&child).expect("mkdir");

        assert!(env_path("PATH").is_some_and(|path| !path.as_os_str().is_empty()));
        assert_eq!(env_path("SAFE_CHAINS_UNSET_OUTSIDE_TEST_VAR"), None);

        assert!(qualifies(&outside, None));
        assert!(!blocked_by_canonical_home(outside.to_str().expect("utf8"), None));
        assert!(!blocked_by_canonical_home(outside.to_str().expect("utf8"), Some(outside.to_str().expect("utf8"))));

        let created = dir_in(&outside, None).expect("dir_in outside");
        assert!(created.path().starts_with(&outside));
        assert!(dir_in(Path::new(""), None).is_none());
        assert!(dir_in(Path::new("/"), None).is_none());

        #[cfg(unix)]
        {
            let link_to_outside = base.join("link_to_outside");
            std::os::unix::fs::symlink(&outside, &link_to_outside).expect("symlink outside");
            assert!(qualifies(&link_to_outside, None));

            let link_to_temp = base.join("link_to_temp");
            std::os::unix::fs::symlink("/tmp", &link_to_temp).expect("symlink temp");
            assert!(!qualifies(&link_to_temp, None));

            let home_link = base.join("home_link");
            std::os::unix::fs::symlink(&outside, &home_link).expect("symlink home");
            assert!(blocked_by_canonical_home(child.to_str().expect("utf8"), home_link.to_str()));
            assert!(!qualifies(&child, home_link.to_str()));
        }

        let cwd = base.join("cwd");
        std::fs::create_dir(&cwd).expect("cwd");
        std::fs::File::open(&cwd)
            .expect("open cwd")
            .set_modified(std::time::SystemTime::UNIX_EPOCH)
            .expect("set mtime");
        let fake_home = format!("/nonexistent/{}", home_user());
        let mut cmd = std::process::Command::new(std::env::current_exe().expect("test exe"));
        cmd.args(["--exact", "outside_dir::tests::probe_outside", "--nocapture", "--test-threads=1"]);
        cmd.current_dir(&cwd);
        cmd.env_remove("SAFE_CHAINS_REQUIRE_OUTSIDE");
        cmd.env("SAFE_CHAINS_OUTSIDE_DIR_PROBE", "1");
        cmd.env("SAFE_CHAINS_TEST_OUTSIDE_DIR", &outside);
        cmd.env("HOME", &fake_home);
        let out = cmd.output().expect("spawn");
        let text = text_of(&out);
        assert!(out.status.success(), "{text}");
        assert!(text.contains("PROBE_OK=some "), "{text}");
        assert_eq!(std::fs::metadata(&cwd).expect("cwd meta").modified().expect("cwd mtime"), std::time::SystemTime::UNIX_EPOCH);
    }

    fn run_probe(test: &str, dir: Option<&str>, require: bool) -> std::process::Output {
        let mut cmd = std::process::Command::new(std::env::current_exe().expect("test exe"));
        cmd.args(["--exact", test, "--nocapture", "--test-threads=1"]);
        cmd.env_remove("SAFE_CHAINS_TEST_OUTSIDE_DIR");
        cmd.env_remove("SAFE_CHAINS_REQUIRE_OUTSIDE");
        cmd.env("SAFE_CHAINS_OUTSIDE_DIR_PROBE", "1");
        if let Some(dir) = dir {
            cmd.env("SAFE_CHAINS_TEST_OUTSIDE_DIR", dir);
        }
        if require {
            cmd.env("SAFE_CHAINS_REQUIRE_OUTSIDE", "1");
        }
        cmd.output().expect("spawn")
    }

    fn text_of(out: &std::process::Output) -> String {
        format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
    }

    #[test]
    fn probe_outside() {
        if std::env::var_os("SAFE_CHAINS_OUTSIDE_DIR_PROBE").is_none() {
            return;
        }
        assert!(dir_in(Path::new(""), None).is_none());
        match writable_outside() {
            Some(dir) => println!("PROBE_OK=some {}", dir.path().display()),
            None => println!("PROBE_OK=none"),
        }
    }

    #[test]
    fn probe_uncovered() {
        if std::env::var_os("SAFE_CHAINS_OUTSIDE_DIR_PROBE").is_none() {
            return;
        }
        uncovered();
        println!("UNCOVERED_RETURNED");
    }
}
