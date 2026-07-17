#![no_main]

//! Coverage-guided extension of the `arbitrary_command_strings_never_panic` proptest: the static
//! classifier must NEVER panic or hang on any input. libFuzzer instruments `safe_chains` and steers
//! byte mutations toward unexplored shell-parser / CST / resolver branches that random proptest
//! generation rarely reaches. A crash here is a real availability bug (a hostile command string that
//! aborts the hook); reproduce it with `cargo fuzz run parse fuzz/artifacts/parse/crash-<hash>`.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Commands are text; lossy-decode so EVERY input exercises the classifier (strict from_utf8
    // would skip most byte mutations and starve coverage). The only contract under test is "does
    // not panic / hang" — the returned verdict is intentionally ignored.
    let command = String::from_utf8_lossy(data);
    let _ = safe_chains::is_safe_command(&command);
});
