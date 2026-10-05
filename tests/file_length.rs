//! The file-length gate.
//!
//! Adapted from an earlier gate used in other Rust projects, whose `production_lines` rule was
//! arrived at the hard way and is reproduced rather than re-derived — see the comment on it.
//! safe-chains went without one until 2026-09-16, by which time `src/registry/tests.rs` had
//! reached 7,900 lines and `src/pathgate.rs` 2,000. Function-level
//! lints never see a file growing one function at a time, and this crate has `too_many_lines` and
//! `cognitive_complexity` turned on the whole time.
//!
//! Three decisions make it useful rather than annoying:
//!
//! 1. **Tests don't count against production.** Rust convention keeps `#[cfg(test)] mod tests` in
//!    the same file, so counting whole files would mean "adding tests can break the build" — the
//!    opposite of what we want.
//!
//! 2. **A whole-file test module is tests too.** This is the part the earlier gate did not need. safe-chains
//!    keeps its big suites in their own files behind `#[cfg(test)] mod tests;` — `src/registry/tests.rs`,
//!    `src/handler_property_tests.rs`, `src/tests.rs`, `src/composition.rs` and five more. Read as
//!    production those files are enormous and would each land a five-figure pin, which is the same
//!    "adding tests breaks the build" failure wearing a different hat: the code is test code, it just
//!    lives one file over. They are held to the TEST limit instead, and the set is DERIVED from the
//!    `#[cfg(test)] mod …;` declarations rather than listed, so moving a suite in or out is picked up
//!    on its own.
//!
//! 3. **It ratchets.** A file already over its limit is pinned at the size it was when the gate went
//!    in: allowed to shrink, never to grow, and a shrink must lower its pin in the same change (the
//!    test says so), so the file cannot grow back. The gate goes in today and the tree catches up,
//!    instead of the limit being raised to fit whatever was written.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use syn::spanned::Spanned;
use syn::visit::Visit;

/// The ceiling for a production file, in lines that are not test items.
///
/// 400 is the limit the earlier gate used in every project that adopted it, and the point
/// of the number is that it is the same one, not that it is tuned.
const LIMIT: usize = 400;

/// The ceiling for a file that is entirely test code.
///
/// Higher than production because Rust test code here is mostly DATA — corpus tables, fixture
/// invocations, `examples_safe` sweeps — which reads long without being complex, and splitting a
/// table in half to satisfy a line count makes it harder to read, not easier. The margin is the
/// same proportion the TypeScript gate uses, where tests get 300 against production's 250.
const TEST_LIMIT: usize = 500;

/// Files over their limit, each pinned at its size when the gate went in: it may shrink, never
/// grow. A file that outgrows its limit AFTER this point is split, never pinned — a pin is only
/// ever for a file that was over before the gate existed.
///
/// Sizes are as of the commit that introduced the gate. Lowering one is a normal part of shrinking
/// a file; adding one is not.
fn pinned() -> HashMap<&'static str, usize> {
    HashMap::from(PINS)
}

include!("fixtures/file_length_pins.rs");

/// The 1-based line ranges of every test-only item, its attributes and doc comments included.
struct TestItems(Vec<(usize, usize)>);

impl TestItems {
    fn add(&mut self, attrs: &[syn::Attribute], item: &impl Spanned) -> bool {
        if !test_only(attrs) {
            return false;
        }
        let span = item.span();
        self.0.push((span.start().line, span.end().line));
        true
    }
}

impl<'a> Visit<'a> for TestItems {
    fn visit_item(&mut self, i: &'a syn::Item) {
        let attrs = match i {
            syn::Item::Const(x) => &x.attrs,
            syn::Item::Enum(x) => &x.attrs,
            syn::Item::Fn(x) => &x.attrs,
            syn::Item::Impl(x) => &x.attrs,
            syn::Item::Macro(x) => &x.attrs,
            syn::Item::Mod(x) => &x.attrs,
            syn::Item::Static(x) => &x.attrs,
            syn::Item::Struct(x) => &x.attrs,
            syn::Item::Trait(x) => &x.attrs,
            syn::Item::Type(x) => &x.attrs,
            syn::Item::Use(x) => &x.attrs,
            _ => return syn::visit::visit_item(self, i),
        };
        if !self.add(attrs, i) {
            syn::visit::visit_item(self, i);
        }
    }
    fn visit_impl_item_fn(&mut self, f: &'a syn::ImplItemFn) {
        if !self.add(&f.attrs, f) {
            syn::visit::visit_impl_item_fn(self, f);
        }
    }
}

/// Is this item compiled only for tests (`#[test]`, `#[cfg(test)]`)?
fn test_only(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("test") || (a.path().is_ident("cfg") && a.parse_args::<syn::Meta>().is_ok_and(|m| m.path().is_ident("test")))
    })
}

/// The file's lines outside its test-only items (`#[cfg(test)]` modules, helpers and impls,
/// `#[test]` functions), wherever they sit.
///
/// Read from the parsed file, never the text. Every text rule this gate had was fooled by a shape
/// of ordinary code: two early drafts took any `#[cfg(test)]` as the start of the tests and
/// waved through arbitrarily large files, a later one stopped at the first test module (it
/// measured a 7,993-line `main.rs` at 74 lines), and the one this was adapted from ended a module
/// at the first `}` in column 0, which a fixture string holds as often as the module's end does. This crate's tests are
/// made almost entirely of such strings.
///
/// Blank lines TOUCHING a test item are counted as part of it — the one correction this copy makes
/// to the inherited rule. Without it the gate breaks its own first promise: a test item added to a
/// pinned file arrives with a blank line separating it from its neighbour, that blank line sits
/// outside every test span, and the file's PRODUCTION count goes up by one. "Adding tests never
/// breaks the build" was then false for exactly the files the ratchet holds, which is where it
/// matters. Found the day the gate went in — `src/pathgate.rs` grew two production lines from an
/// addition that was entirely `#[cfg(test)]`.
///
/// Both directions, because the blank line can fall on either side: between two test items, or
/// between the production code above and a test item appended below.
fn production_lines(source: &str) -> usize {
    let file = syn::parse_file(source).unwrap_or_else(|e| panic!("does not parse: {e}"));
    let mut tests = TestItems(Vec::new());
    tests.visit_file(&file);
    let lines: Vec<&str> = source.lines().collect();
    let total = lines.len();
    let mut is_test = vec![false; total + 1];
    for (a, b) in &tests.0 {
        is_test[*a..=(*b).min(total)].fill(true);
    }
    let blank = |line: usize| lines.get(line - 1).is_some_and(|l| l.trim().is_empty());
    for start in 1..=total {
        if !is_test[start] {
            continue;
        }
        for line in (1..start).rev().take_while(|l| blank(*l)) {
            is_test[line] = true;
        }
        for line in (start + 1..=total).take_while(|l| blank(*l)) {
            is_test[line] = true;
        }
    }
    (1..=total).filter(|line| !is_test[*line]).count()
}

/// The bodyless `mod NAME;` declarations in `source`, split by whether they are test-only, with any
/// `#[path = "…"]` override attached.
///
/// Bodyless only: `mod tests { … }` is an INLINE module, already handled by `production_lines`, and
/// it names no separate file.
fn mod_decls(source: &str) -> Vec<(String, Option<String>, bool)> {
    let Ok(file) = syn::parse_file(source) else { return Vec::new() };
    file.items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Mod(m) if m.content.is_none() => {
                let path = m.attrs.iter().find(|a| a.path().is_ident("path")).and_then(|a| match &a.meta {
                    syn::Meta::NameValue(nv) => match &nv.value {
                        syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) => Some(s.value()),
                        _ => None,
                    },
                    _ => None,
                });
                Some((m.ident.to_string(), path, test_only(&m.attrs)))
            }
            _ => None,
        })
        .collect()
}

/// Where `mod NAME;` inside `declaring_file` resolves to: `<dir>/NAME.rs` or `<dir>/NAME/mod.rs`.
///
/// `<dir>` is the declaring file's own module directory — the parent for `lib.rs`/`mod.rs`, and the
/// same-named sibling directory otherwise, which is how `mod tests;` in `src/suggest.rs` reaches
/// `src/suggest/tests.rs`.
fn resolve_mod(declaring_file: &Path, name: &str, path_attr: Option<&str>) -> Option<PathBuf> {
    let parent = declaring_file.parent()?;
    let stem = declaring_file.file_stem()?.to_str()?;
    let dir = if matches!(stem, "lib" | "main" | "mod") { parent.to_path_buf() } else { parent.join(stem) };
    if let Some(rel) = path_attr {
        let candidate = dir.join(rel);
        return candidate.is_file().then_some(candidate);
    }
    [dir.join(format!("{name}.rs")), dir.join(name).join("mod.rs")].into_iter().find(|p| p.is_file())
}

/// Every file that exists only for tests: one declared `#[cfg(test)] mod NAME;`, and everything
/// such a file goes on to declare.
///
/// Transitive on purpose. `src/cst/proptests.rs` is test-only, so a `mod resolution;` inside it is
/// test-only too, whether or not that declaration repeats the `#[cfg(test)]` its parent already
/// applies. Reading only the first level would have let a suite escape the set by being one module
/// deeper.
fn test_only_files(files: &[PathBuf]) -> HashSet<PathBuf> {
    let sources: HashMap<&PathBuf, String> = files.iter().map(|p| (p, std::fs::read_to_string(p).unwrap_or_default())).collect();
    let mut found = HashSet::new();
    let mut queue: Vec<PathBuf> = Vec::new();

    for file in files {
        for (name, path_attr, is_test) in mod_decls(&sources[file]) {
            if is_test && let Some(target) = resolve_mod(file, &name, path_attr.as_deref()) {
                queue.push(target);
            }
        }
    }
    while let Some(file) = queue.pop() {
        if !found.insert(file.clone()) {
            continue;
        }
        let Some(source) = sources.get(&file) else { continue };
        for (name, path_attr, _) in mod_decls(source) {
            if let Some(target) = resolve_mod(&file, &name, path_attr.as_deref()) {
                queue.push(target);
            }
        }
    }
    found
}

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn sources(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
}

/// The verdict for one file, or `None` when it is within its allowance. Split out from the walk so
/// the ratchet stays testable: with a tree that satisfies every pin, these branches would go
/// unexercised and rot.
fn verdict(relative: &str, lines: usize, limit: usize, pinned: &HashMap<&'static str, usize>) -> Option<String> {
    match pinned.get(relative) {
        Some(&ceiling) if lines > ceiling => Some(format!(
            "{relative}: {lines} lines, up from its pinned {ceiling}. It is already over the \
             {limit}-line limit; split it rather than growing it further. New code goes in a new \
             module, never into a pinned file."
        )),
        Some(_) if lines <= limit => Some(format!(
            "{relative}: down to {lines} lines — under the {limit} limit, so remove its entry from \
             the pin list and let the real limit hold it there."
        )),
        // The ratchet clicks: a pin left above the file's size would let it grow back.
        Some(&ceiling) if lines < ceiling => Some(format!(
            "{relative}: down to {lines} lines from its pinned {ceiling}. Lower its pin to {lines} \
             so it cannot grow back."
        )),
        Some(_) => None,
        None if lines > limit => Some(format!(
            "{relative}: {lines} lines, over the {limit} limit. Split it into pieces that each do \
             one thing (test items are not counted, so they are not the cause)."
        )),
        None => None,
    }
}

/// Every `.rs` file the gate measures, with the limit it is held to and the lines it spends.
fn measured() -> Vec<(String, usize, usize)> {
    let root = crate_root();
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    let test_only = test_only_files(&files);
    // Integration tests are test code by location, so they need no declaration to be recognized.
    sources(&root.join("tests"), &mut files);
    files.push(root.join("build.rs"));

    files
        .iter()
        .filter(|p| p.is_file())
        .map(|path| {
            let relative = path.strip_prefix(&root).unwrap_or(path).to_string_lossy().replace('\\', "/");
            let source = std::fs::read_to_string(path).unwrap_or_default();
            let is_test = test_only.contains(path) || relative.starts_with("tests/");
            let (limit, lines) = if is_test { (TEST_LIMIT, source.lines().count()) } else { (LIMIT, production_lines(&source)) };
            (relative, lines, limit)
        })
        .collect()
}

#[test]
fn no_file_outgrows_its_limit() {
    let pinned = pinned();
    let files = measured();
    assert!(files.len() > 50, "only {} files measured — the walk is broken, not the tree", files.len());
    let mut failures: Vec<String> = files
        .iter()
        .filter_map(|(relative, lines, limit)| verdict(relative, *lines, *limit, &pinned))
        .collect();
    failures.sort();
    assert!(failures.is_empty(), "\n{}\n", failures.join("\n"));
}

#[test]
fn the_ratchet_holds_a_pinned_file_to_its_size() {
    let pinned = HashMap::from([("a.rs", 500)]);
    assert!(verdict("a.rs", 500, 400, &pinned).is_none(), "at its ceiling is fine");
    assert!(
        verdict("a.rs", 450, 400, &pinned).is_some_and(|m| m.contains("Lower its pin to 450")),
        "a shrink must lower the pin, or the file could grow back"
    );
    assert!(verdict("a.rs", 501, 400, &pinned).is_some_and(|m| m.contains("up from its pinned")), "a pinned file may not grow");
    assert!(verdict("a.rs", 400, 400, &pinned).is_some_and(|m| m.contains("remove its entry")), "once under the limit the pin must go");
    assert!(verdict("b.rs", 400, 400, &pinned).is_none(), "unpinned, at the limit");
    assert!(verdict("b.rs", 401, 400, &pinned).is_some_and(|m| m.contains("over the")), "unpinned, over the limit");
}

#[test]
fn every_pinned_file_still_exists() {
    // A rename leaving a stale entry would exempt nothing, and the gate would quietly stop
    // protecting the file it names.
    let measured: HashSet<String> = measured().into_iter().map(|(r, _, _)| r).collect();
    let missing: Vec<&str> = pinned().keys().copied().filter(|r| !measured.contains(*r)).collect();
    assert!(missing.is_empty(), "pinned files are no longer measured at these paths: {missing:?}");
}

/// The whole-file test modules must be RECOGNIZED as tests, or they get held to the production
/// limit and the gate turns into "adding tests breaks the build" one file over.
///
/// Named explicitly rather than counted: a bug that emptied the set would still satisfy any
/// count-based check that happened to be under the total.
#[test]
fn whole_file_test_modules_are_read_as_tests() {
    let by_path: HashMap<String, usize> = measured().into_iter().map(|(r, _, limit)| (r, limit)).collect();
    for relative in [
        "src/registry/tests.rs", "src/handler_property_tests.rs", "src/tests.rs", "src/composition.rs", "src/cst/proptests.rs",
        "src/engine/testgen.rs", "src/engine/resolve/scenarios.rs", "src/suggest/tests.rs", "src/decisionlog/tests.rs",
    ] {
        assert_eq!(
            by_path.get(relative),
            Some(&TEST_LIMIT),
            "{relative} is declared `#[cfg(test)] mod …;` and must be held to the test limit"
        );
    }
    // …and the other way, or "everything is a test file" would pass the loop above.
    for relative in ["src/pathgate.rs", "src/main.rs", "src/registry/mod.rs"] {
        assert_eq!(by_path.get(relative), Some(&LIMIT), "{relative} is production");
    }
}

#[test]
fn only_test_items_are_left_out_of_the_count() {
    assert_eq!(production_lines("fn a() {}\nfn b() {}\n#[cfg(test)]\nmod tests {\n // lots\n}\n"), 2);
    assert_eq!(production_lines("fn a() {}\n"), 1, "a file with no tests counts whole");
    assert_eq!(production_lines(""), 0);
    let cases: &[(&str, &str, usize)] = &[
        // 3, not the inherited 4: the blank line after the declaration is now absorbed with it.
        (
            "a test-only mod declaration is its own line, not the rest of the file",
            "mod real;\n#[cfg(test)]\nmod test_support;\n\nfn a() {}\nfn b() {}\n",
            3,
        ),
        (
            "a test-only helper is the helper, not the rest of the file",
            "fn a() {}\n#[cfg(test)]\nfn helper() {}\nfn b() {}\nfn c() {}\n",
            3,
        ),
        // Likewise 2, not the inherited 3 — the trailing blank line goes with the declaration.
        (
            "an attribute between the guard and the item goes with the item",
            "#[cfg(test)]\n#[path = \"t.rs\"]\nmod tests;\n\nfn a() {}\nfn b() {}\n",
            2,
        ),
        ("a same-line test module body is still skipped", "fn a() {}\n#[cfg(test)] mod tests {\n    fn t() {}\n}\n", 1),
        ("a one-line test module", "fn a() {}\n#[cfg(test)] mod tests { fn t() {} }\nfn b() {}\n", 2),
        (
            // 3, not the inherited 4: `fn a`, `fn b`, `fn c`. The blank line between the test
            // module and `fn b` is absorbed. The point of the case survives — production code
            // AFTER a test module is still counted, which is what one earlier rule lost.
            "production code after a test module still counts (files here keep them between functions)",
            "fn a() {}\n#[cfg(test)]\nmod a_tests {\n    #[test]\n    fn t() {\n    }\n}\n\nfn b() {}\nfn c() {}\n",
            3,
        ),
        (
            "a `}` in column 0 inside a test's string does not end the module",
            "#[cfg(test)]\nmod tests {\n    const FIX: &str = \"\n}\n\";\n    fn t() {}\n}\nfn b() {}\n",
            1,
        ),
        (
            "`#[cfg(test)]` in a string or comment is not an attribute",
            "// #[cfg(test)]\nconst A: &str = \"\n#[cfg(test)]\nmod x {\";\nfn b() {}\n",
            5,
        ),
        ("a test-only method in a production impl", "impl A {\n    fn a() {}\n    #[cfg(test)]\n    fn t() {}\n}\n", 3),
        ("doc comments go with their item", "/// tests\n#[cfg(test)]\nmod t {}\nfn b() {}\n", 1),
        // The correction. Each of these is what "add a test to a pinned file" actually looks like.
        ("a blank line between two test items belongs to them", "fn a() {}\n#[cfg(test)]\nfn t() {}\n\n#[cfg(test)]\nfn u() {}\n", 1),
        ("a blank line before an appended test item belongs to it", "fn a() {}\nfn b() {}\n\n#[cfg(test)]\nmod t {\n}\n", 2),
        ("a blank line after a test item belongs to it", "#[cfg(test)]\nmod t {\n}\n\nfn a() {}\n", 1),
        ("a run of blank lines is absorbed whole", "fn a() {}\n\n\n#[cfg(test)]\nfn t() {}\n\n\nfn b() {}\n", 2),
        ("blank lines between two PRODUCTION items still count", "fn a() {}\n\nfn b() {}\n", 3),
    ];
    for (why, source, expected) in cases {
        assert_eq!(production_lines(source), *expected, "{why}");
    }
}

#[test]
fn a_test_only_module_declaration_is_recognized_wherever_it_points() {
    let decls = mod_decls("#[cfg(test)]\nmod tests;\nmod real;\n#[cfg(test)]\nmod inline { }\n");
    assert_eq!(
        decls,
        vec![("tests".to_string(), None, true), ("real".to_string(), None, false),],
        "bodyless declarations only, each tagged with whether it is test-only"
    );
    let with_path = mod_decls("#[cfg(test)]\n#[path = \"support/x.rs\"]\nmod x;\n");
    assert_eq!(with_path, vec![("x".to_string(), Some("support/x.rs".to_string()), true)]);
}
