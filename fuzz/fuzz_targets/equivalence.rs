#![no_main]

//! METAMORPHIC target: a semantics-preserving respelling must not change the verdict.
//!
//! The `parse` target discards the verdict, so it can only find availability bugs. This one asserts
//! a property OF the verdict, which is what makes it able to find a FAIL-OPEN. It needs no oracle:
//! the two sides of each pair are the same command written two ways, so any disagreement is a bug
//! in us regardless of which side is right.
//!
//! Every gap found in the 2026-07-29 review was exactly this shape — one spelling gated and another
//! not. `borg --rsh /tmp/evil` denied while `BORG_RSH=/tmp/evil borg` was approved; a flag gated in
//! the space form had to be checked separately in the `=` form. Those were all found by hand.
//!
//! The fuzzer mutates the VALUE, which is where the classification actually happens — paths,
//! traversals, `~` forms, encodings, unicode. The templates are fixed and hand-verified equivalent,
//! so the transform is sound by construction rather than by rewriting arbitrary text (a textual
//! rewrite would break inside quotes or a heredoc body and report false crashes, which is worse
//! than no target at all: a nightly that cries wolf gets ignored).

use libfuzzer_sys::fuzz_target;

/// Pairs that denote the SAME operation. Both sides must always agree.
const EQUIVALENT: &[(&str, &str)] = &[
    // Long vs short vs `=`: the same flag, three spellings.
    ("rsync -e {v} ./src/ ./dst/", "rsync --rsh {v} ./src/ ./dst/"),
    ("rsync --rsh {v} ./src/ ./dst/", "rsync --rsh={v} ./src/ ./dst/"),
    ("webpack -c {v}", "webpack --config {v}"),
    ("webpack --config {v}", "webpack --config={v}"),
    ("eslint -c {v} ./src", "eslint --config {v} ./src"),
    ("eslint --config {v} ./src", "eslint --config={v} ./src"),
    ("nox -f {v}", "nox --noxfile {v}"),
    ("nox --noxfile {v}", "nox --noxfile={v}"),
    ("mkdocs build -f {v}", "mkdocs build --config-file {v}"),
    ("marp -c {v} ./s.md", "marp --config-file {v} ./s.md"),
    ("sphinx-build -c {v} ./d ./o", "sphinx-build --conf-dir {v} ./d ./o"),
    ("mypy --python-executable {v} ./src", "mypy --python-executable={v} ./src"),
    // Environment twin vs flag: the same thing said two ways, and the pairing the
    // `twin_flag`/`twin_base` tag exists to hold.
    ("borg --rsh {v} check repo", "BORG_RSH={v} borg check repo"),
    ("borg --remote-path {v} list repo", "BORG_REMOTE_PATH={v} borg list repo"),
    ("restic --password-command {v} snapshots", "RESTIC_PASSWORD_COMMAND={v} restic snapshots"),
    ("rsync --rsh {v} ./src/ ./dst/", "RSYNC_RSH={v} rsync ./src/ ./dst/"),
    // Quoting that changes nothing: a value with no metacharacters means the same bare, single- or
    // double-quoted. This is where a classifier that inspects the raw token rather than the parsed
    // word gets caught.
    ("cat {v}", "cat '{v}'"),
    ("cat {v}", "cat \"{v}\""),
    ("cp {v} ./dst", "cp '{v}' ./dst"),
    ("echo hi > {v}", "echo hi > '{v}'"),
];

/// A value safe to splice into every template above without changing what the shell does.
///
/// Anything the shell would treat as structure — whitespace, quotes, redirects, expansion — is
/// rejected, because with it the two sides stop denoting the same command and a mismatch would say
/// nothing. Keeping the filter strict is what keeps a failure meaningful.
/// A leading `-` is excluded deliberately. Whether `--rsh -x` passes `-x` as the VALUE or starts a
/// new flag is a getopt convention, not a shell one, and tools differ — real rsync takes it as the
/// value while we read it as an unknown flag and refuse. That divergence is fail-CLOSED and worth
/// its own look, but inside this target it would report a modelling nuance as a fail-open on every
/// run and drown the signal the target exists for.
fn is_transparent(v: &str) -> bool {
    !v.is_empty()
        && !v.starts_with('-')
        && v.len() <= 200
        && !v.chars().any(|c| {
            c.is_whitespace()
                || c.is_control()
                || matches!(
                    c,
                    '\'' | '"' | '`' | '\\' | '$' | '(' | ')' | ';' | '&' | '|' | '<' | '>' | '*'
                        | '?' | '[' | ']' | '{' | '}' | '!' | '#' | '='
                )
        })
}

fuzz_target!(|data: &[u8]| {
    let Ok(value) = std::str::from_utf8(data) else {
        return;
    };
    if !is_transparent(value) {
        return;
    }
    for (left, right) in EQUIVALENT {
        let a = left.replace("{v}", value);
        let b = right.replace("{v}", value);
        let va = safe_chains::is_safe_command(&a);
        let vb = safe_chains::is_safe_command(&b);
        assert_eq!(
            va, vb,
            "same operation, different verdict:\n  `{a}` -> {va}\n  `{b}` -> {vb}"
        );
    }
});
