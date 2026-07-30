#![no_main]

//! CONFIG target: the one input an attacker can place in a repository.
//!
//! A repo `.safe-chains.toml` is read out of a checked-out tree. Everything else this suite fuzzes
//! is a command the agent composed or an envelope the harness sent; this is a FILE that arrives
//! with the code. It has already aborted the process once — `load_toml` panicked on a wrong-typed
//! key — and because this runs as a PreToolUse hook, an abort means the harness proceeds. An
//! ordinary typo in a config file silently disabled safe-chains altogether.
//!
//! Three properties:
//!
//! 1. No panic, on any bytes. This is the one with history.
//! 2. FAIL-SAFE on malformed input: a source that is not valid TOML must yield NOTHING. Loading
//!    "some" of a broken config is how a half-parsed definition would widen the allowlist.
//! 3. Deterministic: the same source twice gives the same answer. A config whose meaning depends on
//!    when it was read cannot be reasoned about, and the hook reads it on every invocation.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let source = String::from_utf8_lossy(data);

    for repo_scope in [false, true] {
        let loaded = safe_chains::registry::fuzz_load_config(&source, repo_scope);

        // Malformed TOML must load nothing at all. `toml::from_str::<Value>` is the same syntax
        // check the loader itself performs first, so this asserts the skip actually happened rather
        // than re-deriving what "malformed" means.
        if toml::from_str::<toml::Value>(&source).is_err() {
            assert_eq!(
                loaded, 0,
                "a config that is not valid TOML loaded {loaded} definitions \
                 (repo_scope={repo_scope}); skipping must be all-or-nothing"
            );
        }

        let again = safe_chains::registry::fuzz_load_config(&source, repo_scope);
        assert_eq!(
            loaded, again,
            "loading the same config twice gave {loaded} then {again} \
             (repo_scope={repo_scope}); the hook reads it on every invocation"
        );
    }
});
