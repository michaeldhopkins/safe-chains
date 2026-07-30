#![no_main]

//! EXPLAIN target: the layer that talks to the human and to the model.
//!
//! `parse` calls `is_safe_command`, which never renders, so `cst/explain.rs` sits outside it. That
//! matters because the rendered text is not decoration: `--explain` is what a person reads before
//! approving, and the same render feeds the hook's `additionalContext`, which is injected into the
//! model's context. A command that can forge a line of it can tell the reader something we never
//! said — a `✓ … auto-approves` for a segment that was refused.
//!
//! Three properties, none of which the availability target can see:
//!
//! 1. The two paths AGREE. `is_safe_command` and `explain().is_allowed()` walk the tree separately;
//!    if they ever disagree the explanation is describing a different verdict than the one enforced,
//!    which is worse than no explanation.
//! 2. Exactly one marker line per real segment. Counting is what makes this general: it fails for
//!    any payload that manufactures a line, not just the spellings someone thought to list.
//! 3. No control characters survive into the output. That is the property forgery rests on — a
//!    newline is what lets injected text start its own line.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let command = String::from_utf8_lossy(data);

    let enforced = safe_chains::is_safe_command(&command);
    let explanation = safe_chains::cst::explain(&command);
    let described = explanation.is_allowed();
    assert_eq!(
        enforced, described,
        "the explanation describes a different verdict than the one enforced: \
         is_safe_command={enforced}, explain().is_allowed()={described}, command={command:?}"
    );

    let rendered = explanation.render();

    // A line may only bear a marker if it belongs to a real segment.
    let marker_lines = rendered
        .lines()
        .filter(|l| l.starts_with("  \u{2713}  ") || l.starts_with("  \u{2717}  "))
        .count();
    // `<=`, not `==`. The renderer legitimately emits FEWER lines than there are segments — an
    // unparseable command prints one "could not parse" message and no marker lines at all. What
    // forgery needs is an EXTRA line, so the anti-forgery property is a ceiling: a command may
    // never produce more marker lines than there are real segments to mark.
    assert!(
        marker_lines <= explanation.segments.len(),
        "command forged a segment line: {} markers for {} segments, command={command:?}\n{rendered}",
        marker_lines,
        explanation.segments.len()
    );

    // Only the newlines the renderer itself emits may appear. Anything else is command-derived text
    // that escaped neutralizing, and a `\r` or a bidi override rewrites what the reader sees just as
    // effectively as a `\n` does.
    for c in rendered.chars() {
        assert!(
            c == '\n' || !c.is_control(),
            "control character {:?} reached the rendered output for command={command:?}",
            c
        );
        assert!(
            !matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'),
            "bidi override {:?} reached the rendered output for command={command:?}",
            c
        );
    }
});
