#![no_main]

//! SETUP target: `--setup` merging into a settings file it did not write.
//!
//! `install` edits a file the user owns, and the shapes it meets are whatever a hand-edit or an
//! unknown schema version left behind. A wrong-typed key panicked cursor's installer, and under
//! `--auto-detect` that aborted the whole run — the user saw "Claude Code: installed" and every
//! tool after cursor was silently left UNPROTECTED, on a machine they now believed was covered.
//!
//! This generalizes `no_target_install_panics_or_clobbers_an_unreadable_config` from a table of
//! hand-written shapes to a search. Three properties:
//!
//! 1. No panic, whatever the existing file contains.
//! 2. On REFUSAL, the file is byte-identical. An unreadable file is usually a hand-edit or a schema
//!    we do not know, and rewriting config we did not understand is not ours to do.
//! 3. On SUCCESS, the result still parses as JSON. A merge that leaves the file unloadable is the
//!    same silent disabling by another route — the harness reads it, not us.
//!
//! Filesystem-driven because the property is about `install`, not about a parser. The homes and the
//! config path per target are discovered ONCE and reused, so an iteration is a write plus a call
//! rather than a directory tree.

use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;
use std::sync::OnceLock;

struct Bed {
    name: &'static str,
    home: PathBuf,
    config: PathBuf,
}

/// One home per target, plus the config path that target actually writes — learned by letting it
/// install once into a clean tree rather than hardcoding filenames that would drift.
fn beds() -> &'static Vec<Bed> {
    static BEDS: OnceLock<Vec<Bed>> = OnceLock::new();
    BEDS.get_or_init(|| {
        let root = std::env::temp_dir().join(format!("sc-fuzz-setup-{}", std::process::id()));
        let mut out = Vec::new();
        for target in safe_chains::targets::registry() {
            let home = root.join(target.name());
            let _ = std::fs::create_dir_all(&home);
            for dir in target.detect_paths(&home) {
                let _ = std::fs::create_dir_all(&dir);
            }
            if let Ok(safe_chains::targets::InstallOutcome::Installed { path }) =
                target.install(&home)
            {
                out.push(Bed { name: target.name(), home, config: path });
            }
        }
        out
    })
}

fuzz_target!(|data: &[u8]| {
    // Self-guard against vacuity: if discovery found no target that installs, every iteration below
    // would be a no-op and the run would report clean while asserting nothing.
    assert!(
        !beds().is_empty(),
        "no target produced an installed config; setup_merge would be testing nothing"
    );
    for bed in beds() {
        let Some(target) = safe_chains::targets::find(bed.name) else {
            continue;
        };
        if std::fs::write(&bed.config, data).is_err() {
            continue;
        }
        let before = std::fs::read(&bed.config).unwrap_or_default();

        // Property 1 is that this line does not abort the process.
        let outcome = target.install(&bed.home);

        let after = std::fs::read(&bed.config).unwrap_or_default();
        match outcome {
            Err(_) => assert!(
                before == after,
                "{}: refused the merge but still rewrote the file",
                bed.name
            ),
            Ok(_) => assert!(
                serde_json::from_slice::<serde_json::Value>(&after).is_ok(),
                "{}: merge produced a file the harness cannot parse:\n{}",
                bed.name,
                String::from_utf8_lossy(&after)
            ),
        }
    }
});
