//! Generate a libFuzzer dictionary and seed corpus for the `parse` fuzz target from the command
//! registry's own TOML.
//!
//! Byte mutation almost never synthesizes a valid command, so a from-scratch corpus barely reaches
//! the per-command grammars (measured: ~26% region coverage of authored source). The cheapest lift
//! is to hand the fuzzer the real material: the `examples_safe` / `examples_denied` invocations as
//! SEED inputs (complete, valid commands that drive coverage straight into each grammar), and the
//! command / subcommand / flag VOCABULARY as a dictionary the mutator splices instead of guessing
//! bytes. Both are derived from `commands/**/*.toml`, so new commands are covered automatically.
//!
//! Run from the repo root:
//!   cargo run --bin gen-fuzz-corpus --features fuzz-gen
//!
//! Feature-gated (`fuzz-gen`) so it never ships in the normal binary. Outputs
//! `fuzz/dict/parse.dict` and `gen-*` seed files under `fuzz/corpus/parse/` (both git-ignored: the
//! dict and `gen-*` seeds are regenerated, never committed — a hand-curated `seed-*` corpus could
//! still be committed alongside).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dict_path = root.join("fuzz/dict/parse.dict");
    let corpus_dir = root.join("fuzz/corpus/parse");

    let mut names = BTreeSet::new(); // command / subcommand / flag names
    let mut flags = BTreeSet::new(); // flag tokens from the allowlist arrays
    let mut seeds = BTreeSet::new(); // full example invocations

    let mut files = Vec::new();
    collect_toml_files(&root.join("commands"), &mut files);
    files.sort();
    for path in &files {
        if path.file_name().and_then(|s| s.to_str()) == Some("SAMPLE.toml") {
            continue;
        }
        let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let value: toml::Value =
            toml::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
        collect(&value, &mut names, &mut flags, &mut seeds);
    }

    write_dict(&dict_path, &names, &flags);
    let written = write_seeds(&corpus_dir, &seeds);

    eprintln!(
        "dict: {} names + {} flags -> {}",
        names.len(),
        flags.len(),
        dict_path.display()
    );
    eprintln!("seeds: {written} unique invocations -> {}", corpus_dir.display());
}

fn collect_toml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_toml_files(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            out.push(path);
        }
    }
}

/// Walk the TOML value tree, pulling every real token out of the keys that carry them — so nested
/// `[[command.sub]]` / `[[command.sub.sub]]` / `[command.wrapper]` etc. are covered without knowing
/// the shape. `name` yields command/subcommand/flag names; the allowlist arrays and a `path_gate`
/// `flags` table yield flag tokens; `examples_*` yield seed invocations.
fn collect(
    v: &toml::Value,
    names: &mut BTreeSet<String>,
    flags: &mut BTreeSet<String>,
    seeds: &mut BTreeSet<String>,
) {
    match v {
        toml::Value::Table(t) => {
            for (k, val) in t {
                match k.as_str() {
                    "name" => {
                        if let Some(s) = val.as_str() {
                            add_token(s, names);
                        }
                    }
                    "standalone" | "valued" | "bare_flags" | "main_valued" | "main_variadic"
                    | "eval_safe_flags" | "eval_safe_required_flags" => add_string_tokens(val, flags),
                    // `[command.path_gate] flags = { "--output-dir" = "write" }` — keys are flags.
                    "flags" => {
                        if let Some(tbl) = val.as_table() {
                            for fk in tbl.keys() {
                                add_token(fk, flags);
                            }
                        }
                    }
                    "examples_safe" | "examples_denied" => add_seeds(val, seeds),
                    _ => {}
                }
                collect(val, names, flags, seeds);
            }
        }
        toml::Value::Array(a) => {
            for el in a {
                collect(el, names, flags, seeds);
            }
        }
        _ => {}
    }
}

/// A dictionary token: a single word (no whitespace), non-empty, and short. Filters out anything
/// that isn't a real command/flag word so the dict stays tight.
fn add_token(s: &str, set: &mut BTreeSet<String>) {
    if !s.is_empty() && s.len() <= 48 && !s.chars().any(char::is_whitespace) {
        set.insert(s.to_string());
    }
}

fn add_string_tokens(v: &toml::Value, set: &mut BTreeSet<String>) {
    if let Some(arr) = v.as_array() {
        for el in arr {
            if let Some(s) = el.as_str() {
                add_token(s, set);
            }
        }
    }
}

fn add_seeds(v: &toml::Value, set: &mut BTreeSet<String>) {
    if let Some(arr) = v.as_array() {
        for el in arr {
            if let Some(s) = el.as_str()
                && !s.is_empty()
            {
                set.insert(s.to_string());
            }
        }
    }
}

fn write_dict(path: &Path, names: &BTreeSet<String>, flags: &BTreeSet<String>) {
    // Shell metacharacters the CST/parser layers need but byte mutation rarely balances on its own
    // (unbalanced quotes/parens are the common stuck state). Seeding them as dict entries lets the
    // mutator build chains, substitutions, and redirects around the real command tokens.
    const META: &[&str] = &[
        "|", "||", "&&", "&", ";", "$(", ")", "${", "}", "`", "\"", "'", ">", ">>", "<", "2>&1",
        "*", "?", "~", "../", "\\", "=", " -- ", "\n", "\t",
    ];
    let mut entries: BTreeSet<String> = BTreeSet::new();
    for tok in names.iter().chain(flags.iter()).map(String::as_str).chain(META.iter().copied()) {
        entries.insert(dict_entry(tok));
    }
    let body: String = entries.into_iter().map(|e| e + "\n").collect();
    let dir = path.parent().expect("dict path has a parent");
    fs::create_dir_all(dir).expect("create dict dir");
    fs::write(path, format!("# Generated by gen-fuzz-corpus from commands/**/*.toml. Do not edit.\n{body}"))
        .expect("write dict");
}

/// A libFuzzer dictionary line: a double-quoted token with `"`, `\`, and non-printables escaped
/// (`\xHH`). See https://llvm.org/docs/LibFuzzer.html#dictionaries.
fn dict_entry(tok: &str) -> String {
    let mut out = String::from("\"");
    for b in tok.bytes() {
        match b {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!("\\x{b:02x}")),
        }
    }
    out.push('"');
    out
}

/// Write each seed as a content-hash-named `gen-*` file (idempotent: same corpus every run, no
/// duplicates). `gen-` keeps them git-ignored, distinct from a committed `seed-*` corpus.
fn write_seeds(dir: &Path, seeds: &BTreeSet<String>) -> usize {
    fs::create_dir_all(dir).expect("create corpus dir");
    for seed in seeds {
        let digest = Sha256::digest(seed.as_bytes());
        let name: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
        fs::write(dir.join(format!("gen-{name}")), seed.as_bytes()).expect("write seed");
    }
    seeds.len()
}
