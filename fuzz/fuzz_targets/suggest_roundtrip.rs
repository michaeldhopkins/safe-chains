#![no_main]

//! SUGGEST target: the config GENERATOR, which `is_safe_command` never calls.
//!
//! `--suggest` writes a `.safe-chains.toml` and prints a `[[trusted]]` pin for the user to paste
//! into the trust root. Everything it emits is derived from a command string, so the command is the
//! input and the generated config is the output — a generator whose output does not parse is a
//! generator that hands someone a file safe-chains can never load, which is exactly the bug found
//! by hand this session (it appended to an unparseable config and reported success).
//!
//! The property is a ROUNDTRIP: whatever we generate must be readable back. It needs no oracle —
//! `toml::from_str` is the judge — and it holds for every command, which is what makes it fuzzable.
//! Note this deliberately exercises only the PURE functions; the file-writing path lives in main.rs
//! and would turn a fuzz run into a disk-thrashing exercise.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let command = String::from_utf8_lossy(data);

    let outcome = safe_chains::suggest::analyze(&command);
    let safe_chains::suggest::Outcome::Generated { entries, .. } = outcome else {
        return; // nothing generated: no config to check
    };
    if entries.is_empty() {
        return;
    }

    // 1. The block we print and tell the user to add must be valid TOML on its own.
    let block = safe_chains::suggest::render_toml(&entries);
    assert!(
        toml::from_str::<toml::Value>(&block).is_ok(),
        "render_toml produced invalid TOML for command={command:?}:\n{block}"
    );

    // 2. The merged file we WRITE must parse. This is the roundtrip that matters: the pin we hand
    //    over is a hash of exactly these bytes, so a file that does not load makes the pin certify
    //    something unusable.
    let merged = safe_chains::suggest::merged_content("", &entries);
    assert!(
        toml::from_str::<toml::Value>(&merged).is_ok(),
        "merged_content produced invalid TOML for command={command:?}:\n{merged}"
    );

    // 3. Merging into an EXISTING valid config must keep it valid — appending is where a generator
    //    usually breaks the file it is extending.
    let existing = "[[command]]\nname = \"already-here\"\nmax_positional = 1\n";
    let appended = safe_chains::suggest::merged_content(existing, &entries);
    assert!(
        toml::from_str::<toml::Value>(&appended).is_ok(),
        "merging into a valid config broke it for command={command:?}:\n{appended}"
    );
    assert!(
        appended.contains("already-here"),
        "merging DROPPED the user's existing entry for command={command:?}:\n{appended}"
    );

    // 4. The pin is what the user pastes into the trust root, so it must be valid TOML too — and it
    //    must survive a directory name containing quotes or newlines, which is attacker-chosen in
    //    any cloned repo.
    let hash = safe_chains::suggest::config_hash(merged.as_bytes());
    let pin = safe_chains::suggest::pin_block(&command, &hash);
    assert!(
        toml::from_str::<toml::Value>(&pin).is_ok(),
        "pin_block produced invalid TOML for dir={command:?}:\n{pin}"
    );
});
