//! The lint-suppression ratchet.
//!
//! Two things let a lint stop biting without anyone deciding it should: an `#[allow(…)]` or
//! `#[expect(…)]` on an item, and a `clippy.toml` threshold raised above clippy's default. Both
//! are pinned at their level when this went in (2026-10-04). Either may only move toward clean,
//! and the pin moves with it in the same change, so nothing regrows. When you touch an item
//! carrying a suppression, remove it (extract the options struct, split the function) rather
//! than work around it. A third way is a baseline lint dropped from `Cargo.toml`'s `[lints]`,
//! which `the_lint_baseline_holds` refuses.
//!
//! Suppressions are counted on tokens, not text, so a string or comment mentioning one is not
//! a suppression, while one inside a macro invocation (a `proptest!` block) still is.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use proc_macro2::{Delimiter, TokenStream, TokenTree};

/// The trees scanned, relative to the crate root. `build.rs` is read on its own.
const ROOTS: &[&str] = &["src", "tests", "fuzz/fuzz_targets"];

/// `#[allow]`/`#[expect]` attributes in the tree. May only fall.
const SUPPRESSIONS: usize = 25;

/// The lints `Cargo.toml` must keep at least this strict. `forbid` outranks `deny`, which
/// outranks `warn`.
const BASELINE: &[(&str, &str, &str)] = &[
    ("rust", "unsafe_code", "forbid"),
    ("rust", "unexpected_cfgs", "warn"),
    ("clippy", "struct_excessive_bools", "deny"),
    ("clippy", "fn_params_excessive_bools", "deny"),
    ("clippy", "collapsible_if", "deny"),
    ("clippy", "too_many_arguments", "deny"),
    ("clippy", "needless_range_loop", "deny"),
    ("clippy", "extend_with_drain", "deny"),
    ("clippy", "unwrap_used", "deny"),
    ("clippy", "dbg_macro", "deny"),
    ("clippy", "match_bool", "warn"),
    ("clippy", "bool_to_int_with_if", "warn"),
    ("clippy", "cognitive_complexity", "warn"),
    ("clippy", "too_many_lines", "warn"),
];

/// Clippy's defaults for the thresholds a `clippy.toml` can raise. A key missing from here
/// fails the check, so a newly loosened threshold cannot slip past it unmeasured.
const CLIPPY_DEFAULTS: &[(&str, i64)] = &[
    ("too-many-arguments-threshold", 7),
    ("cognitive-complexity-threshold", 25),
    ("too-many-lines-threshold", 100),
    ("type-complexity-threshold", 250),
    ("max-struct-bools", 3),
    ("max-fn-params-bools", 3),
];

/// Thresholds above their default, pinned at today's value.
const PINNED_THRESHOLDS: &[(&str, i64)] = &[("cognitive-complexity-threshold", 30), ("too-many-arguments-threshold", 8)];

/// Settings in `clippy.toml` that are switches rather than thresholds.
const SWITCHES: &[&str] = &["allow-unwrap-in-tests"];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn sources(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
}

/// Whether an attribute's bracketed body suppresses a lint: `allow(…)`, `expect(…)`, or a
/// `cfg_attr(…)` carrying either.
fn suppresses(body: &TokenStream) -> bool {
    let mut tokens = body.clone().into_iter();
    match tokens.next() {
        Some(TokenTree::Ident(name)) if name == "allow" || name == "expect" => true,
        Some(TokenTree::Ident(name)) if name == "cfg_attr" => tokens.any(|t| match t {
            TokenTree::Group(g) => g
                .stream()
                .into_iter()
                .any(|inner| matches!(&inner, TokenTree::Ident(i) if i == "allow" || i == "expect")),
            _ => false,
        }),
        _ => false,
    }
}

/// Suppression attributes in a token stream, descending into every group.
fn count_suppressions(stream: TokenStream) -> usize {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    let mut count = 0;
    for (i, token) in tokens.iter().enumerate() {
        match token {
            TokenTree::Punct(p) if p.as_char() == '#' => {
                let body = match (tokens.get(i + 1), tokens.get(i + 2)) {
                    (Some(TokenTree::Group(g)), _) if g.delimiter() == Delimiter::Bracket => Some(g),
                    (Some(TokenTree::Punct(bang)), Some(TokenTree::Group(g)))
                        if bang.as_char() == '!' && g.delimiter() == Delimiter::Bracket =>
                    {
                        Some(g)
                    }
                    _ => None,
                };
                if body.is_some_and(|g| suppresses(&g.stream())) {
                    count += 1;
                }
            }
            TokenTree::Group(g) => count += count_suppressions(g.stream()),
            _ => {}
        }
    }
    count
}

fn suppressions_in(source: &str) -> usize {
    let stream: TokenStream = source.parse().unwrap_or_else(|e| panic!("does not lex: {e:?}"));
    count_suppressions(stream)
}

/// `key = integer` settings from a `clippy.toml`; switches and comments are skipped.
fn thresholds(clippy_toml: &str) -> BTreeMap<String, i64> {
    clippy_toml
        .lines()
        .map(|line| line.split('#').next().unwrap_or_default().trim())
        .filter_map(|line| line.split_once('='))
        .filter_map(|(key, value)| Some((key.trim().to_string(), value.trim().parse().ok()?)))
        .collect()
}

fn suppression_verdict(count: usize, pin: usize) -> Option<String> {
    if count > pin {
        Some(format!("{count} lint suppressions, up from {pin}. Fix the cause instead of silencing the lint."))
    } else if count < pin {
        Some(format!("{count} lint suppressions, down from {pin}. Lower SUPPRESSIONS to {count}."))
    } else {
        None
    }
}

/// Failures for one `clippy.toml`, given clippy's defaults and today's pins.
fn threshold_verdicts(set: &BTreeMap<String, i64>, defaults: &[(&str, i64)], pins: &[(&str, i64)]) -> Vec<String> {
    let mut failures = Vec::new();
    for (key, &value) in set {
        let Some(&(_, default)) = defaults.iter().find(|(k, _)| k == key) else {
            failures.push(format!(
                "clippy.toml sets `{key}`, which has no default in CLIPPY_DEFAULTS. Add clippy's \
                 default so the check can tell whether it is loosened."
            ));
            continue;
        };
        let pin = pins.iter().find(|(k, _)| k == key).map(|&(_, p)| p);
        match pin {
            _ if value <= default && pin.is_some() => {
                failures.push(format!("`{key}` is back to {value}, within clippy's default of {default}; remove its pin."))
            }
            _ if value <= default => {}
            None => failures.push(format!("`{key}` = {value} loosens clippy's default of {default}. Fix the code instead.")),
            Some(p) if value > p => {
                failures.push(format!("`{key}` = {value}, up from its pinned {p}. A limit is never raised to fit the code."))
            }
            Some(p) if value < p => {
                failures.push(format!("`{key}` down to {value} from {p}; lower its pin to {value}."));
            }
            Some(_) => {}
        }
    }
    for (key, _) in pins {
        if !set.contains_key(*key) {
            failures.push(format!("`{key}` is pinned but no longer set; remove its pin."));
        }
    }
    failures
}

#[test]
fn lint_suppressions_only_fall() {
    let root = workspace_root();
    let mut files = Vec::new();
    for dir in ROOTS {
        let before = files.len();
        sources(&root.join(dir), &mut files);
        assert!(files.len() > before, "no `.rs` files under `{dir}`; the walk is broken");
    }
    files.push(root.join("build.rs"));
    let count: usize = files.iter().map(|path| suppressions_in(&std::fs::read_to_string(path).unwrap_or_default())).sum();
    let verdict = suppression_verdict(count, SUPPRESSIONS);
    assert!(verdict.is_none(), "{}", verdict.unwrap_or_default());
}

#[test]
fn clippy_thresholds_only_move_toward_the_default() {
    let toml = std::fs::read_to_string(workspace_root().join("clippy.toml")).unwrap_or_else(|e| panic!("reading clippy.toml: {e}"));
    let mut set = thresholds(&toml);
    set.retain(|key, _| !SWITCHES.contains(&key.as_str()));
    assert!(!set.is_empty(), "no thresholds read from clippy.toml; the parser is broken");
    let failures = threshold_verdicts(&set, CLIPPY_DEFAULTS, PINNED_THRESHOLDS);
    assert!(failures.is_empty(), "\n{}\n", failures.join("\n"));
}

#[test]
fn suppressions_are_counted_on_tokens() {
    assert_eq!(suppressions_in("#[allow(dead_code)] fn a() {}"), 1);
    assert_eq!(suppressions_in("#![expect(clippy::too_many_lines)]"), 1);
    assert_eq!(suppressions_in("#[cfg_attr(test, allow(unused))] fn a() {}"), 1);
    assert_eq!(suppressions_in("mod m { fn a() { #[allow(unused)] let x = 1; } }"), 1);
    assert_eq!(suppressions_in("proptest! { #[allow(unused)] fn p() {} }"), 1, "inside a macro");
    assert_eq!(suppressions_in(r##"const S: &str = "#[allow(x)]"; // #[allow(y)]"##), 0);
    assert_eq!(suppressions_in("#[cfg(test)] #[derive(Debug)] /// doc\n struct S;"), 0);
}

#[test]
fn the_suppression_count_ratchets() {
    assert!(suppression_verdict(2, 2).is_none());
    assert!(suppression_verdict(3, 2).is_some_and(|m| m.contains("up from 2")));
    assert!(suppression_verdict(1, 2).is_some_and(|m| m.contains("Lower SUPPRESSIONS to 1")));
}

#[test]
fn thresholds_ratchet_toward_the_default() {
    let defaults = [("t", 10)];
    let pins = [("t", 20)];
    let with = |v: i64| BTreeMap::from([("t".to_string(), v)]);
    assert!(threshold_verdicts(&with(20), &defaults, &pins).is_empty(), "at its pin");
    assert!(threshold_verdicts(&with(21), &defaults, &pins)[0].contains("up from its pinned"));
    assert!(threshold_verdicts(&with(15), &defaults, &pins)[0].contains("lower its pin to 15"));
    assert!(threshold_verdicts(&with(10), &defaults, &pins)[0].contains("remove its pin"));
    assert!(threshold_verdicts(&with(11), &defaults, &[])[0].contains("loosens"));
    assert!(threshold_verdicts(&with(5), &defaults, &[]).is_empty(), "stricter is fine");
    assert!(threshold_verdicts(&BTreeMap::new(), &defaults, &pins)[0].contains("no longer set"));
    let unknown = BTreeMap::from([("mystery".to_string(), 1)]);
    assert!(threshold_verdicts(&unknown, &defaults, &[])[0].contains("no default"));
}

#[test]
fn thresholds_are_read_from_clippy_toml() {
    let read = thresholds("# note\nmax-struct-bools = 2 # why\nallow-unwrap-in-tests = true\n");
    assert_eq!(read, BTreeMap::from([("max-struct-bools".to_string(), 2)]));
}

fn strength(level: &str) -> u8 {
    match level {
        "forbid" => 3,
        "deny" => 2,
        "warn" => 1,
        _ => 0,
    }
}

/// Where `[lints]` is weaker than the baseline. A lint is set either as `"level"` or as a table
/// with a `level` key; `-` and `_` spell the same clippy lint.
fn baseline_gaps(cargo_toml: &str, baseline: &[(&str, &str, &str)]) -> Vec<String> {
    let manifest: toml::Table = toml::from_str(cargo_toml).expect("Cargo.toml parses");
    let lints = manifest.get("lints").and_then(|l| l.as_table());
    let mut gaps = Vec::new();
    for (group, lint, want) in baseline {
        let table = lints.and_then(|l| l.get(*group)).and_then(|g| g.as_table());
        let set = table.and_then(|t| t.iter().find(|(k, _)| k.replace('-', "_") == *lint)).map(|(_, v)| v);
        let level = set.and_then(|v| v.as_str().or_else(|| v.get("level").and_then(|l| l.as_str()))).unwrap_or("absent");
        if strength(level) < strength(want) {
            gaps.push(format!("[lints.{group}] {lint} is {level}; the baseline is {want}"));
        }
    }
    gaps
}

#[test]
fn the_lint_baseline_holds() {
    let text = std::fs::read_to_string(workspace_root().join("Cargo.toml")).expect("Cargo.toml");
    let gaps = baseline_gaps(&text, BASELINE);
    assert!(gaps.is_empty(), "\n{}\n", gaps.join("\n"));
}

#[test]
fn a_weakened_or_missing_baseline_lint_is_refused() {
    let baseline = [("clippy", "unwrap_used", "deny"), ("rust", "unsafe_code", "forbid")];
    let ok = "[lints.clippy]\nunwrap-used = \"deny\"\n[lints.rust]\nunsafe_code = { level = \"forbid\" }\n";
    assert!(baseline_gaps(ok, &baseline).is_empty());
    let weak = "[lints.clippy]\nunwrap_used = \"warn\"\n[lints.rust]\nunsafe_code = \"deny\"\n";
    assert_eq!(baseline_gaps(weak, &baseline).len(), 2);
    assert_eq!(baseline_gaps("[package]\nname = \"x\"\n", &baseline).len(), 2);
}
