//! The invocations the crate's own example tests name: every string in a `safe!`, `denied!`,
//! `inert!`, `safe_read!` or `safe_write!` block under `src/`. The Rust handlers (curl, awk, sed,
//! find, tar…) have no TOML examples, so these blocks are where their surface is written down.
//! Parsed with `syn`, so a macro inside a string or a comment is not mistaken for one.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::visit::Visit;

const EXAMPLE_MACROS: &[&str] = &["safe", "denied", "inert", "safe_read", "safe_write"];

struct Entry(String);

impl Parse for Entry {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<syn::Ident>()?;
        input.parse::<syn::Token![:]>()?;
        Ok(Entry(input.parse::<syn::LitStr>()?.value()))
    }
}

#[derive(Default)]
struct Collector {
    found: BTreeSet<String>,
    unreadable: usize,
}

impl<'a> Visit<'a> for Collector {
    fn visit_macro(&mut self, mac: &'a syn::Macro) {
        if mac.path.get_ident().is_some_and(|i| EXAMPLE_MACROS.contains(&i.to_string().as_str())) {
            match mac.parse_body_with(Punctuated::<Entry, syn::Token![,]>::parse_terminated) {
                Ok(entries) => self.found.extend(entries.into_iter().map(|e| e.0)),
                Err(_) => self.unreadable += 1,
            }
        }
        syn::visit::visit_macro(self, mac);
    }
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The example strings of one source text. Panics on a block it cannot read, so a change in the
/// macros' shape fails loudly instead of quietly shrinking the corpus.
pub fn examples_in(source: &str, label: &str) -> BTreeSet<String> {
    let file = syn::parse_file(source).unwrap_or_else(|e| panic!("parse {label}: {e}"));
    let mut collector = Collector::default();
    collector.visit_file(&file);
    assert_eq!(collector.unreadable, 0, "{label}: an example macro block is not `name: \"cmd\", …`");
    collector.found
}

pub fn rust_examples(root: &Path) -> BTreeSet<String> {
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    files.sort();
    files
        .iter()
        .flat_map(|path| {
            let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            examples_in(&text, &path.display().to_string())
        })
        .collect()
}
