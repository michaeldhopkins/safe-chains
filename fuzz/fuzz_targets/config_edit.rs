#![no_main]

//! CONFIG EDIT target: `safe-chains config` editing a user file it did not write.
//!
//! The first byte chooses a path and the faces to grant; the rest is the existing file.
//! Properties:
//!
//! 1. No panic, on any file.
//! 2. An edit of a file that loads writes a file that loads. Writing back a broken file would
//!    switch off every rule in it. (A file that does not load is refused before any edit, by
//!    `check_loads`, which this target also runs.)
//! 3. An edit touches only what it is about: on a file with no grants, adding one and removing
//!    it again gives back the original text exactly.

use libfuzzer_sys::fuzz_target;
use safe_chains::userconfig::{Access, fuzzing};

const PATHS: &[&str] = &["~/notes", "/opt/fuzz", "~/projects/app", "/srv/data"];

fuzz_target!(|data: &[u8]| {
    let Some((&selector, rest)) = data.split_first() else { return };
    let existing = String::from_utf8_lossy(rest);
    if fuzzing::check_loads(&existing).is_err() {
        return;
    }
    let path = PATHS[usize::from(selector) % PATHS.len()];
    let access = [Access::Read, Access::Write, Access::ReadWrite][usize::from(selector >> 4) % 3];

    if let Ok(granted) = fuzzing::grant(&existing, path, access) {
        assert!(fuzzing::check_loads(&granted).is_ok(), "a grant wrote a file that does not load:\n{granted}");
        let has_grants = toml::from_str::<toml::Value>(&existing).ok().is_some_and(|v| v.get("grant").is_some());
        if !has_grants {
            let back = fuzzing::remove_grant(&granted, path).expect("the grant just added can be removed");
            // Byte for byte, except that a last line with no newline gains one: the table needed it.
            let nl = if existing.contains("\r\n") { "\r\n" } else { "\n" };
            let expected = if existing.is_empty() || existing.ends_with('\n') { existing.to_string() } else { format!("{existing}{nl}") };
            assert_eq!(back, expected, "grant then remove changed the file");
        }
    }
    // The same for an entry: add one for a command nobody has, remove it again.
    if let Ok(added) = fuzzing::add_entry(&existing, "fuzzedittool sync --fast") {
        assert!(fuzzing::check_loads(&added).is_ok(), "an entry wrote a file that does not load:\n{added}");
        let back = fuzzing::remove_command(&added, "fuzzedittool").expect("the entry just added can be removed");
        let nl = if existing.contains("\r\n") { "\r\n" } else { "\n" };
        let expected = if existing.is_empty() || existing.ends_with('\n') { existing.to_string() } else { format!("{existing}{nl}") };
        assert_eq!(back, expected, "add then remove changed the file");
    }
    for removed in [fuzzing::remove_grant(&existing, path), fuzzing::remove_command(&existing, "fuzztool")]
        .into_iter()
        .flatten()
    {
        assert!(fuzzing::check_loads(&removed).is_ok(), "a removal wrote a file that does not load:\n{removed}");
    }
});
