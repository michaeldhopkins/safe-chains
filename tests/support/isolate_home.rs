//! A private home for this integration-test binary.
//!
//! The library stands one up on the first lookup, because this binary is a test and not the hook.
//! The hook binary does not include this file.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static HOME: OnceLock<PathBuf> = OnceLock::new();

pub fn home() -> &'static Path {
    HOME.get_or_init(|| safe_chains::process_home_path().expect("isolated home"))
}

#[test]
fn the_library_reads_the_installed_home() {
    let path = safe_chains::userconfig::user_config_path().expect("installed home");
    assert!(path.starts_with(home()), "the library read {path:?}, not {}", home().display());
    assert!(home().join(".sc-isolated-home").is_file(), "exe {:?}", std::env::current_exe());
    let process = std::env::var_os("HOME");
    assert_ne!(process.as_deref(), Some(home().as_os_str()), "the isolated home is the process home");
}

#[test]
fn every_integration_test_installs_an_isolated_home() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut missing = Vec::new();
    for entry in std::fs::read_dir(&root).expect("tests") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read test");
        if !text.contains("mod isolate_home") {
            missing.push(path.file_name().expect("name").to_string_lossy().into_owned());
        }
    }
    assert!(missing.is_empty(), "integration tests without an isolated home: {missing:?}");
}
