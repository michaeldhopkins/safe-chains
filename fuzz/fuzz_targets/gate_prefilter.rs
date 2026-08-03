#![no_main]

//! GATE-PREFILTER target: a declared path gate must never skip a value its own judge would refuse.
//!
//! `pathgate::should_deny` runs the role's judge only on values that clear a pre-filter — a
//! POSITIVE shape test (`looks_like_path`, plus whitespace, plus substitutions). A value whose
//! shape it does not recognize is skipped, unjudged, and therefore approved. In a program that is
//! otherwise allowlist-only that is fail-OPEN by construction, and it has bitten three times, each
//! time as a shape nobody had thought of:
//!
//!   - `borg --rsh 'sh -c evil'`   whitespace: a command line, not a path
//!   - `borg --rsh file:~`         a colon, with no `/` or `.` to make it look like a path
//!   - substitution / `$VAR` values, patched earlier for the same reason
//!
//! A corpus test covers the shapes its author listed, which is exactly the wrong shape of guard for
//! a bug whose defining property is that the shape was unforeseen. So this fuzzes the VALUE and
//! asserts the invariant that has no corpus: whatever the judge says about a value, the gate must
//! say the same. The pre-filter is the only thing between them.
//!
//! Only the fail-OPEN direction is asserted. A gate that denies MORE than the judge means some
//! other flag's rule fired on the same tokens, which is a different question and not a hole.

use libfuzzer_sys::fuzz_target;
use safe_chains::verdict::Verdict;

/// Representative declared gates, one per role, so a single fuzzed value exercises all three
/// judges. Kept small on purpose: the interesting axis is the VALUE, not the command, and the
/// exhaustive command sweep already runs as a unit test.
const GATES: &[(&str, &str)] = &[
    ("borg", "--rsh"),          // Exec
    ("cargo", "--target-dir"),  // Write
    ("asciidoctor", "-o"),      // Write
    ("rsync", "-e"),            // Exec
];

/// Commands whose BARE POSITIONALS carry a declared role, with the leading words needed to reach
/// that slot. The positional invariant is weaker than the flag one and deliberately so: for a
/// declared flag the token IS an operand by declaration, while for a positional whether the token
/// is an operand at all is a real question — which is why the pre-filter still guards this slot and
/// legitimately skips flag-shaped tokens.
const POSITIONALS: &[(&str, &[&str])] = &[
    ("curl", &[]),          // Read
    ("scp", &[]),           // Read
    ("rsync", &[]),         // Read
    ("karma", &["start"]),  // Exec
    ("tilt", &["up"]),      // Exec
];

fuzz_target!(|data: &[u8]| {
    let Ok(value) = std::str::from_utf8(data) else {
        return;
    };
    // A value carrying a NUL or a newline is not a single argv token, so splicing it into a token
    // list would be modelling something the shell cannot produce.
    if value.is_empty() || value.len() > 200 || value.contains(['\0', '\n', '\r']) {
        return;
    }

    for (cmd, flag) in GATES {
        let Some(judged) = safe_chains::pathgate::judge_for_flag(cmd, flag, value) else {
            continue;
        };
        if judged != Verdict::Denied {
            continue;
        }
        let tokens: Vec<_> = [*cmd, *flag, value]
            .iter()
            .map(|s| safe_chains::parse::Token::from_raw(s.to_string()))
            .collect();
        assert!(
            safe_chains::pathgate::should_deny(cmd, &tokens),
            "the pre-filter skipped a value its own gate would refuse: {cmd} {flag} {value:?}",
        );
    }

    // A FLAG-shaped token is not an operand, and the pre-filter is right to skip it — that is the
    // job it still does for this slot. Judging it as a path would be the bug.
    if value.starts_with('-') {
        return;
    }
    for (cmd, lead) in POSITIONALS {
        let Some(judged) = safe_chains::pathgate::judge_for_positional(cmd, value) else {
            continue;
        };
        if judged != Verdict::Denied {
            continue;
        }
        let mut words = vec![(*cmd).to_string()];
        words.extend(lead.iter().map(|s| (*s).to_string()));
        words.push(value.to_string());
        let tokens: Vec<_> = words
            .into_iter()
            .map(safe_chains::parse::Token::from_raw)
            .collect();
        assert!(
            safe_chains::pathgate::should_deny(cmd, &tokens),
            "the pre-filter skipped a POSITIONAL its own gate would refuse: {cmd} {lead:?} {value:?}",
        );
    }
});
