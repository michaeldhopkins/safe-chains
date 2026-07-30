#![no_main]

//! PATH-ADMIT target: the two rules that bound the package-content read admits.
//!
//! Admitting distributed package content for reading ended a real friction — a man page or a
//! vendored crate README was being refused although its bytes are public by construction. But an
//! admit map is exactly what a previous audit retired, after finding it swallowed the macOS
//! keychain, Homebrew service configs and auth logs underneath broad roots. So the admits are
//! narrow, and two rules keep them narrow. Both are asserted here against a fuzzed path tail,
//! because the failure they guard was structural rather than particular to any one path:
//!
//! 1. THE SHIELD IS ABSOLUTE. Region specificity ranks exact ≫ prefix ≫ segment, so every subtree
//!    admit outranks the credential shield's segment match. The day package content became
//!    readable, `/usr/share/.ssh/id_rsa` was approved. Any admit node added later reopens that
//!    unless the shield wins outright, so it is checked against every root, not one.
//! 2. READS ONLY. The justification for these admits covers DISCLOSURE and nothing else. If a
//!    write ever rides in on the same node, the change has quietly widened what the agent can
//!    alter, which was never argued for.

use libfuzzer_sys::fuzz_target;

const ADMIT_ROOTS: &[&str] = &[
    "/usr/share",
    "/usr/include",
    "/usr/lib",
    "/usr/local/share",
    "/usr/local/lib",
    "/opt/homebrew/share",
    "/opt/homebrew/lib",
    "/Library/Developer/CommandLineTools",
    "/nix/store/abc",
    "~/.cargo/registry",
    "~/.rustup/toolchains",
    "~/go/pkg/mod",
    "~/.local/share/mise/installs",
];

const SHIELDS: &[&str] = &[".ssh", ".aws", ".gnupg"];

/// A tail safe to splice into a command without changing its shape. Anything the shell would read
/// as structure is rejected, so a failure means the ADMIT rule broke rather than the parse.
fn is_transparent(tail: &str) -> bool {
    !tail.is_empty()
        && tail.len() <= 120
        && !tail.starts_with('-')
        && !tail.starts_with('/')
        && !tail.chars().any(|c| {
            c.is_whitespace()
                || c.is_control()
                || matches!(
                    c,
                    '\'' | '"' | '`' | '\\' | '$' | '(' | ')' | ';' | '&' | '|' | '<' | '>' | '*'
                        | '?' | '[' | ']' | '{' | '}' | '!' | '#' | '=' | '~'
                )
        })
}

fuzz_target!(|data: &[u8]| {
    let Ok(tail) = std::str::from_utf8(data) else {
        return;
    };
    if !is_transparent(tail) {
        return;
    }

    for root in ADMIT_ROOTS {
        // 1. No admit root may widen the credential shield, at any depth beneath it.
        for shield in SHIELDS {
            let path = format!("{root}/{shield}/{tail}");
            assert!(
                !safe_chains::is_safe_command(&format!("cat {path}")),
                "an admit prefix widened the credential shield: `cat {path}`"
            );
        }

        // 2. The admit is read-only. Whatever the tail, a write to it must still refuse.
        let path = format!("{root}/{tail}");
        for write in [format!("rm -rf {path}"), format!("echo x > {path}")] {
            assert!(
                !safe_chains::is_safe_command(&write),
                "a read admit granted a WRITE: `{write}`"
            );
        }
    }
});
