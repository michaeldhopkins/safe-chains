//! Ratchet: a test that makes a folder under `$TMPDIR` must remove it when it ends.
//!
//! Tests here once named folders by process id (`sc-cargo-run-<pid>-...`, `safe-chains-log-<tag>-<pid>`)
//! and never removed them, so every run left about 400 behind: thousands by the time the disk reaper
//! looked. A `tempfile::TempDir` removes its folder on drop, pass or fail, so the rule is to use one.

use std::path::{Path, PathBuf};

#[path = "support/isolate_home.rs"]
mod isolate_home;

/// Files that build a path under the temp directory by hand: one with a drop guard of its own, one whose
/// folder lives as long as the process and is swept by the next run.
const OWN_GUARD: &[&str] = &["tests/foreign_code_operands.rs", "src/process_home.rs"];

fn rust_files(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn tests_make_their_temp_folders_with_tempfile() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for sub in ["src", "tests"] {
        rust_files(&root.join(sub), &mut files);
    }
    let needle = ["temp_dir", "()"].concat();
    let mut offenders = Vec::new();
    for file in files {
        let relative = file.strip_prefix(root).expect("under the crate").to_string_lossy().replace('\\', "/");
        if relative == "tests/temp_litter.rs" || OWN_GUARD.contains(&relative.as_str()) {
            continue;
        }
        let text = std::fs::read_to_string(&file).expect("read source");
        for (n, line) in text.lines().enumerate() {
            if line.contains(&needle) {
                offenders.push(format!("{relative}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "use tempfile::TempDir (or Builder) so the folder is removed when the test ends:\n{}",
        offenders.join("\n")
    );
}
