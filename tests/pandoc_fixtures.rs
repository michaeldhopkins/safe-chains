use std::path::{Path, PathBuf};

const VERSIONS: &[&str] = &["3.12.1"];

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pandoc")
}

fn check_version(version: &str) {
    let dir = fixtures().join(version);
    let input_raw = std::fs::read_to_string(dir.join("input-formats.txt")).expect("input-formats.txt");
    let output_raw = std::fs::read_to_string(dir.join("output-formats.txt")).expect("output-formats.txt");
    let readers: Vec<&str> = input_raw.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let writers: Vec<&str> = output_raw.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    assert!(!readers.is_empty());
    assert!(!writers.is_empty());
    for reader in &readers {
        assert!(safe_chains::is_safe_command(&format!("pandoc -f {reader} README.md")), "reader format `{reader}` rejected");
        assert!(safe_chains::is_safe_command(&format!("pandoc --from={reader}+smart README.md")), "reader `{reader}+smart` rejected");
    }
    for writer in &writers {
        if *writer == "pdf" {
            assert!(!safe_chains::is_safe_command(&format!("pandoc -t {writer} README.md")), "pdf writer must remain refused");
        } else {
            assert!(safe_chains::is_safe_command(&format!("pandoc -t {writer} README.md")), "writer format `{writer}` rejected");
            assert!(safe_chains::is_safe_command(&format!("pandoc --to={writer}+smart README.md")), "writer `{writer}+smart` rejected");
            assert!(safe_chains::is_safe_command(&format!("pandoc -D {writer}")), "template format `{writer}` rejected");
        }
    }
}

#[test]
fn pandoc_3_12_1() {
    check_version("3.12.1");
}

#[test]
fn every_fixture_version_has_a_test() {
    let mut found: Vec<String> = std::fs::read_dir(fixtures())
        .expect("fixtures/pandoc")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    found.sort();
    let mut listed: Vec<String> = VERSIONS.iter().map(|v| (*v).to_string()).collect();
    listed.sort();
    assert_eq!(found, listed, "each tests/fixtures/pandoc/<version> needs a named test and a VERSIONS entry");
}
