//! The config an in-process test reads.
//!
//! Classification looks up `~/.config/safe-chains.toml` and a project `.safe-chains.toml`. Under
//! `cfg(test)` those lookups come here, so a suite does not measure the config of whoever ran it.
//! The default root is an empty temporary directory. A test that is about config passes its own
//! files through [`with_user_config`] or [`with_project_config`].

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

thread_local! {
    static FIXTURE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static FORCE_EMPTY: Cell<usize> = const { Cell::new(0) };
}

static EMPTY: OnceLock<tempfile::TempDir> = OnceLock::new();

fn empty_root() -> PathBuf {
    EMPTY.get_or_init(|| tempfile::tempdir().expect("empty config root")).path().to_path_buf()
}

/// The directory the loader treats as `$HOME` when it looks for a config file.
pub(crate) fn config_home() -> PathBuf {
    if FORCE_EMPTY.with(Cell::get) > 0 {
        return empty_root();
    }
    FIXTURE.with(|slot| slot.borrow().clone()).unwrap_or_else(empty_root)
}

/// Where a project `.safe-chains.toml` search starts. Never the process's current directory.
pub(crate) fn repo_search_root() -> PathBuf {
    config_home()
}

/// Holds the one-time registry load on the empty root, even when the initializing thread has a
/// fixture installed.
pub(crate) struct ForceEmpty;

impl ForceEmpty {
    pub(crate) fn install() -> Self {
        FORCE_EMPTY.with(|flag| flag.set(flag.get().saturating_add(1)));
        Self
    }
}

impl Drop for ForceEmpty {
    fn drop(&mut self) {
        FORCE_EMPTY.with(|flag| flag.set(flag.get().saturating_sub(1)));
    }
}

struct FixtureGuard {
    previous: Option<PathBuf>,
    _dir: Option<tempfile::TempDir>,
}

impl FixtureGuard {
    fn install(root: PathBuf, dir: Option<tempfile::TempDir>) -> Self {
        let previous = FIXTURE.with(|slot| slot.replace(Some(root)));
        Self { previous, _dir: dir }
    }
}

impl Drop for FixtureGuard {
    fn drop(&mut self) {
        let previous = self.previous.take();
        FIXTURE.with(|slot| *slot.borrow_mut() = previous);
    }
}

/// Run `body` with `root` as the config home. The caller keeps `root` alive. No command overlay:
/// this is how a search is pointed at a directory the test already filled in.
pub(crate) fn with_root<R>(root: &Path, body: impl FnOnce() -> R) -> R {
    let guard = FixtureGuard::install(root.to_path_buf(), None);
    let result = body();
    drop(guard);
    result
}

/// Run `body` with `user_toml` as the user config. Command specs reach the classifier through the
/// test overlay, which is dropped when `body` returns. A source that does not load contributes no
/// commands and still stands as the file the level and folder readers open.
pub(crate) fn with_user_config<R>(user_toml: &str, body: impl FnOnce() -> R) -> R {
    let dir = tempfile::tempdir().expect("config fixture");
    write_user(dir.path(), user_toml);
    run_with(dir, user_toml, None, body)
}

/// Run `body` with a project `.safe-chains.toml` of `project_toml`. `user_toml` receives that
/// file's canonical directory and its sha256, and its return value is the user config. The
/// project's commands apply only when that user config's `[[trusted]]` pin matches the file.
pub(crate) fn with_project_config<R>(project_toml: &str, user_toml: impl FnOnce(&Path, &str) -> String, body: impl FnOnce() -> R) -> R {
    let dir = tempfile::tempdir().expect("config fixture");
    let root = dir.path().to_path_buf();
    let project_path = root.join(super::REPO_FILENAME);
    std::fs::write(&project_path, project_toml).expect("write project config");
    let canon = std::fs::canonicalize(&root).unwrap_or(root);
    let hash = super::sha256_hex(project_toml.as_bytes());
    let user = user_toml(&canon, &hash);
    write_user(dir.path(), &user);
    run_with(dir, &user, Some(project_path), body)
}

fn write_user(root: &Path, user_toml: &str) {
    let config_dir = root.join(".config");
    std::fs::create_dir_all(&config_dir).expect("config dir");
    std::fs::write(config_dir.join(super::USER_FILENAME), user_toml).expect("write user config");
}

fn run_with<R>(dir: tempfile::TempDir, user: &str, project: Option<PathBuf>, body: impl FnOnce() -> R) -> R {
    let root = dir.path().to_path_buf();
    let guard = FixtureGuard::install(root, Some(dir));
    let result = match overlay_source(user, project.as_deref()) {
        Some(source) => crate::registry::with_test_commands(&source, body),
        None => body(),
    };
    drop(guard);
    result
}

/// Command text for the test overlay. A project file is included only when the user text pins it.
/// A source that does not load is left out, the way the real loader skips a file it cannot read.
/// Project text is concatenated as written: `[command.output]` is not stripped here. The loader's
/// `repo_scoped` does that, and the test of the restriction calls `repo_scoped` itself.
fn overlay_source(user: &str, project: Option<&Path>) -> Option<String> {
    let mut parts = Vec::new();
    if has_commands(user) {
        parts.push(user.to_string());
    }
    if let Some(path) = project
        && let Ok(bytes) = std::fs::read(path)
        && super::repo_is_trusted(path, &bytes, &super::parse_trusted(user))
        && let Ok(text) = std::str::from_utf8(&bytes)
        && has_commands(text)
    {
        parts.push(text.to_string());
    }
    if parts.is_empty() { None } else { Some(parts.join("\n")) }
}

fn has_commands(source: &str) -> bool {
    super::load_toml(source, "fixture").is_ok_and(|specs| !specs.is_empty())
}
