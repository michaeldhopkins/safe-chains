//! A release build reads `$HOME` and nothing else. `test-home` is how the suite stands up a
//! private home, and it is not a default feature, so a release binary cannot grow one.
#[path = "support/isolate_home.rs"]
mod isolate_home;

use std::path::Path;
use std::process::Command;

fn test_home_stays_off_by_default(manifest: &str) -> Result<(), String> {
    let value: toml::Value = toml::from_str(manifest).map_err(|err| err.to_string())?;
    let features = value.get("features").and_then(toml::Value::as_table).ok_or("no features table")?;
    if !features.contains_key("test-home") {
        return Err("test-home is not declared".to_string());
    }
    let defaulted = features
        .get("default")
        .and_then(toml::Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some("test-home")));
    if defaulted {
        return Err("test-home is a default feature".to_string());
    }
    Ok(())
}

#[test]
fn test_home_is_not_a_default_feature() {
    let _home = isolate_home::home();
    let bad = "[features]\ndefault = [\"test-home\"]\ntest-home = []\n";
    let err = test_home_stays_off_by_default(bad).expect_err("a captured manifest that defaults test-home");
    assert!(err.contains("default"), "{err}");
    let manifest = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).expect("Cargo.toml");
    test_home_stays_off_by_default(&manifest).expect("this package declares test-home and does not default it");
}

#[test]
fn a_renamed_release_binary_reads_the_home_it_was_given() {
    let _home = isolate_home::home();
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target = manifest.join("target/home-probe");
    let status = Command::new(env!("CARGO"))
        .args(["build", "--release", "--locked", "--offline", "--bin", "safe-chains"])
        .env("CARGO_TARGET_DIR", &target)
        .current_dir(manifest)
        .status()
        .expect("build the release binary");
    assert!(status.success(), "release build failed");
    let built = target.join("release/safe-chains");
    let home = tempfile::tempdir().expect("fixture home");
    std::fs::create_dir_all(home.path().join(".config")).expect("config dir");
    std::fs::write(home.path().join(".config/safe-chains.toml"), "level = \"probe-marker-deadbeef\"\n").expect("fixture config");
    let dir = tempfile::tempdir().expect("copy dir");
    let renamed = dir.path().join("x-deadbeef");
    std::fs::copy(&built, &renamed).expect("copy the release binary");
    let cwd = tempfile::tempdir().expect("cwd");
    let out = Command::new(&renamed)
        .args(["config", "list"])
        .env_clear()
        .env("HOME", home.path())
        .env("PATH", "/usr/bin:/bin")
        .current_dir(cwd.path())
        .output()
        .expect("run the renamed binary");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "config list failed: {stdout} {stderr}");
    assert!(stdout.contains("probe-marker-deadbeef"), "the renamed release binary did not read HOME: {stdout}");
    assert!(!stdout.contains("sc-it-home-"), "the renamed release binary used a private home: {stdout}");
    let bytes = std::fs::read(&built).expect("read the release binary");
    assert!(
        !bytes.windows(b"sc-it-home-".len()).any(|window| window == b"sc-it-home-"),
        "the release binary contains a private-home prefix"
    );
}
