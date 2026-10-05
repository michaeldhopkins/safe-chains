//! The property-test manifest check: every pure module states its invariants as property tests.
//!
//! `tests/properties.toml` classifies every file under `src/` as `pure`, `effectful` or
//! `test_only`. This fails when a file is missing from it (so a new module forces the call), when a
//! named property is not a test inside a `proptest!` block, or when a pure file names none and is
//! not in `owed`. `owed` may only shrink: a file that gains a property must leave it.
//!
//! Two details follow from how this crate keeps its tests:
//! - Most of its properties live in whole-file suites (`src/handler_property_tests.rs`,
//!   `src/cst/proptests.rs`, `src/engine/testgen.rs`, …), not beside the code they state an
//!   invariant of. A pure file names those as `"src/suite.rs::property_name"`; a bare name is
//!   looked up in the file itself.
//! - Those suites are neither pure nor effectful code, so they go in `[test_only]` with a reason.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use proc_macro2::{TokenStream, TokenTree};

const ROOTS: &[&str] = &["src"];

fn crate_root() -> PathBuf {
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

/// Names of the functions declared inside `proptest!` invocations, at any depth.
fn property_names(stream: TokenStream, found: &mut BTreeSet<String>) {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    for (i, token) in tokens.iter().enumerate() {
        let TokenTree::Group(group) = token else {
            continue;
        };
        let invoked = matches!(
            (i.checked_sub(2).and_then(|j| tokens.get(j)), i.checked_sub(1).and_then(|j| tokens.get(j))),
            (Some(TokenTree::Ident(name)), Some(TokenTree::Punct(bang)))
                if name == "proptest" && bang.as_char() == '!'
        );
        if invoked {
            let body: Vec<TokenTree> = group.stream().into_iter().collect();
            for pair in body.windows(2) {
                if let [TokenTree::Ident(kw), TokenTree::Ident(name)] = pair
                    && kw == "fn"
                {
                    found.insert(name.to_string());
                }
            }
        } else {
            property_names(group.stream(), found);
        }
    }
}

fn properties_in(source: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    if let Ok(stream) = source.parse::<TokenStream>() {
        property_names(stream, &mut found);
    }
    found
}

fn string_list(value: Option<&toml::Value>) -> Vec<String> {
    value
        .and_then(toml::Value::as_array)
        .map(|items| items.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

/// Every failure for the tree at `root`, measured against `manifest`.
fn check(root: &Path, roots: &[&str], manifest: &str) -> Vec<String> {
    let manifest: toml::Table = match manifest.parse() {
        Ok(table) => table,
        Err(e) => return vec![format!("properties.toml does not parse: {e}")],
    };
    let table = |key: &str| manifest.get(key).and_then(toml::Value::as_table).cloned().unwrap_or_default();
    let pure: BTreeMap<String, Vec<String>> = table("pure").iter().map(|(file, names)| (file.clone(), string_list(Some(names)))).collect();
    let effectful = table("effectful");
    let test_only = table("test_only");
    let owed: BTreeSet<String> = string_list(manifest.get("owed")).into_iter().collect();

    let mut files = Vec::new();
    for dir in roots {
        sources(&root.join(dir), &mut files);
    }
    let on_disk: BTreeSet<String> = files
        .iter()
        .map(|p| p.strip_prefix(root).unwrap_or(p).to_string_lossy().replace('\\', "/"))
        .collect();

    let mut failures = Vec::new();
    for file in &on_disk {
        let kinds = [pure.contains_key(file), effectful.contains_key(file), test_only.contains_key(file)];
        match kinds.iter().filter(|k| **k).count() {
            0 => failures.push(format!(
                "{file} is not in tests/properties.toml. Classify it as pure (naming its property tests, or owed), \
                 effectful (with a reason) or test_only (with a reason)."
            )),
            1 => {}
            _ => failures.push(format!("{file} is listed in more than one of pure, effectful and test_only.")),
        }
    }
    for file in pure.keys().chain(effectful.keys()).chain(test_only.keys()).chain(owed.iter()) {
        if !on_disk.contains(file) {
            failures.push(format!("{file} is in tests/properties.toml but no longer exists."));
        }
    }
    for (kind, entries) in [("effectful", &effectful), ("test_only", &test_only)] {
        for (file, reason) in entries {
            if reason.as_str().is_none_or(|r| r.trim().is_empty()) {
                failures.push(format!("{file} is {kind} without a reason."));
            }
        }
    }
    let mut defined: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut defined_in = |file: &str| {
        defined
            .entry(file.to_string())
            .or_insert_with(|| properties_in(&std::fs::read_to_string(root.join(file)).unwrap_or_default()))
            .clone()
    };
    for (file, names) in &pure {
        for name in names {
            let (home, property) = name.split_once("::").unwrap_or((file.as_str(), name.as_str()));
            if !defined_in(home).contains(property) {
                failures.push(format!("{file} names property `{name}`, which is not a test inside a `proptest!` block in {home}."));
            }
        }
        match (names.is_empty(), owed.contains(file)) {
            (true, false) => failures.push(format!("{file} is pure but names no property tests. Write one; `owed` only shrinks.")),
            (false, true) => failures.push(format!("{file} has property tests now; remove it from `owed`.")),
            _ => {}
        }
    }
    for file in owed.iter().filter(|f| !pure.contains_key(*f) && on_disk.contains(*f)) {
        failures.push(format!("{file} is in `owed` but is not listed as pure."));
    }
    failures.sort();
    failures
}

#[test]
fn every_source_file_is_classified_and_pure_ones_have_properties() {
    let root = crate_root();
    let manifest =
        std::fs::read_to_string(root.join("tests/properties.toml")).unwrap_or_else(|e| panic!("reading tests/properties.toml: {e}"));
    let mut files = Vec::new();
    for dir in ROOTS {
        let before = files.len();
        sources(&root.join(dir), &mut files);
        assert!(files.len() > before, "no `.rs` files under `{dir}`; the walk is broken");
    }
    let failures = check(&root, ROOTS, &manifest);
    assert!(failures.is_empty(), "\n{}\n", failures.join("\n"));
}

#[test]
fn the_check_catches_each_kind_of_gap_in_a_fixture_tree() {
    let fixture = crate_root().join("tests/fixtures/properties");
    let manifest = std::fs::read_to_string(fixture.join("properties.toml")).expect("fixture manifest");
    let failures = check(&fixture, &["src"], &manifest);
    let expect = [
        "src/unlisted.rs is not in tests/properties.toml",
        "src/gone.rs is in tests/properties.toml but no longer exists",
        "src/misnamed.rs names property `not_there`",
        "src/not_a_property.rs names property `plain_test`",
        "src/remote_bad.rs names property `src/suite.rs::not_there`",
        "src/bare.rs is pure but names no property tests",
        "src/paid_off.rs has property tests now; remove it from `owed`",
        "src/effect.rs is in `owed` but is not listed as pure",
        "src/silent.rs is effectful without a reason",
        "src/suite_silent.rs is test_only without a reason",
    ];
    for want in expect {
        assert!(failures.iter().any(|f| f.starts_with(want)), "missing {want:?} in {failures:#?}");
    }
    assert_eq!(failures.len(), expect.len(), "unexpected failures: {failures:#?}");
}

#[test]
fn properties_are_found_inside_proptest_blocks_only() {
    let source = r#"
        #[test] fn plain() {}
        proptest! { #[test] fn direct(x in 0..1) {} }
        mod tests { proptest::proptest! { #[test] fn qualified(x in 0..1) {} } }
        const S: &str = "proptest! { fn in_a_string() {} }";
    "#;
    let names: Vec<String> = properties_in(source).into_iter().collect();
    assert_eq!(names, ["direct", "qualified"]);
}
