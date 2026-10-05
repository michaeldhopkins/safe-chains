//! The invocations the verdict snapshot pins, derived from the registry rather than written by hand.
//!
//! Every `commands/**/*.toml` node (command, sub, nested sub, matrix action, alias) contributes its
//! bare form, one positional, each flag it declares alone, a valued flag with a placeholder value,
//! and a couple of combinations. A `candidate` node is generated like any other: its rows pin that
//! it stays refused. The TOML is read as untyped values so that a new field is ignored rather than
//! breaking the walk, at the cost of naming here every key that carries a flag.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;
use toml::value::Table;

/// The value given to every valued flag and the positional appended to every node. A plain
/// relative word, so a path-gated flag sees a worktree path and a typed flag a non-number.
pub const PLACEHOLDER: &str = "x";

const LOOPBACK: &str = "http://localhost:8000";

const STANDALONE_KEYS: &[&str] = &["standalone", "bare_flags", "optional_valued", "first_arg_standalone", "write_flags"];
const VALUED_KEYS: &[&str] = &["valued", "first_arg_valued", "output_path_flags"];
const LOOPBACK_KEYS: &[&str] = &["loopback_valued", "first_arg_loopback_valued"];
const NESTED_GRAMMARS: &[&str] = &["behavior", "fallback", "wrapper"];

/// The flags one dispatch node declares, gathered from every key that carries them.
#[derive(Default)]
struct Flags {
    standalone: BTreeSet<String>,
    valued: BTreeSet<String>,
    loopback: BTreeSet<String>,
    require_any: Vec<String>,
    first_arg: Vec<String>,
}

impl Flags {
    fn of(node: &Table, policies: &Table) -> Self {
        let mut flags = Flags::default();
        flags.absorb(node);
        for key in NESTED_GRAMMARS {
            if let Some(grammar) = node.get(*key).and_then(Value::as_table) {
                flags.absorb(grammar);
            }
        }
        if let Some(policy) = node.get("policy").and_then(Value::as_str).and_then(|p| policies.get(p)).and_then(Value::as_table) {
            flags.absorb(policy);
        }
        if let Some(chain) = node.get("verb_chain").and_then(Value::as_table) {
            flags.standalone.extend(strings(chain, "main_standalone"));
            flags.valued.extend(strings(chain, "main_valued"));
        }
        if let Some(gate) = node.get("path_gate").and_then(|g| g.get("flags")).and_then(Value::as_table) {
            flags.valued.extend(gate.keys().cloned());
        }
        if let Some(dest) = node.get("destination_flag").and_then(Value::as_str) {
            flags.valued.insert(dest.to_string());
        }
        flags.require_any = strings(node, "require_any");
        flags.first_arg = strings(node, "first_arg");
        flags
    }

    fn absorb(&mut self, table: &Table) {
        for key in STANDALONE_KEYS {
            self.standalone.extend(strings(table, key));
        }
        for key in VALUED_KEYS {
            self.valued.extend(strings(table, key));
        }
        for key in LOOPBACK_KEYS {
            self.loopback.extend(strings(table, key));
        }
    }
}

fn strings(table: &Table, key: &str) -> Vec<String> {
    table
        .get(key)
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string).collect())
        .unwrap_or_default()
}

/// A first-argument glob made concrete: every `*` becomes the placeholder.
fn concrete(pattern: &str) -> String {
    pattern.replace('*', PLACEHOLDER)
}

/// Every invocation one node contributes, `prefix` being the words that reach it (`git log`).
fn node_invocations(prefix: &str, node: &Table, policies: &Table, out: &mut BTreeSet<String>) {
    let flags = Flags::of(node, policies);
    let at = |rest: &str| format!("{prefix} {rest}");
    out.insert(prefix.to_string());
    out.insert(at(PLACEHOLDER));

    let lead: Vec<String> = match flags.require_any.first() {
        Some(r) => vec![String::new(), format!("{r} ")],
        None => vec![String::new()],
    };
    for r in &flags.require_any {
        out.insert(at(r));
    }
    for lead in &lead {
        for f in &flags.standalone {
            out.insert(at(&format!("{lead}{f}")));
        }
        for f in &flags.valued {
            out.insert(at(&format!("{lead}{f} {PLACEHOLDER}")));
        }
        for f in &flags.loopback {
            out.insert(at(&format!("{lead}{f} {LOOPBACK}")));
            out.insert(at(&format!("{lead}{f} {PLACEHOLDER}")));
        }
    }

    let mut standalone = flags.standalone.iter();
    let first_valued = flags.valued.iter().next();
    if let (Some(a), Some(b)) = (standalone.next(), standalone.next()) {
        out.insert(at(&format!("{a} {b}")));
    }
    if let Some(a) = flags.standalone.iter().next() {
        out.insert(at(&format!("{a} {PLACEHOLDER}")));
        if let Some(v) = first_valued {
            out.insert(at(&format!("{a} {v} {PLACEHOLDER}")));
        }
    }

    for pattern in &flags.first_arg {
        let arg = concrete(pattern);
        out.insert(at(&arg));
        if let Some(f) = flags.standalone.iter().next() {
            out.insert(at(&format!("{arg} {f}")));
        }
    }

    for flag in node.get("flag").and_then(Value::as_array).into_iter().flatten() {
        let Some(name) = flag.get("name").and_then(Value::as_str) else {
            continue;
        };
        match flag.get("value_prefix").and_then(Value::as_str) {
            Some(value) => out.insert(at(&format!("{name} {value}{PLACEHOLDER}"))),
            None => out.insert(at(name)),
        };
    }

    for sub in node.get("sub").and_then(Value::as_array).into_iter().flatten() {
        let Some(sub) = sub.as_table() else { continue };
        let Some(name) = sub.get("name").and_then(Value::as_str) else {
            continue;
        };
        node_invocations(&at(name), sub, policies, out);
        for alias in strings(sub, "aliases") {
            node_invocations(&at(&alias), sub, policies, out);
        }
    }
}

/// `[[command.matrix]]`: every parent × action, with the action's policy's flags and its guard.
fn matrix_invocations(name: &str, command: &Table, policies: &Table, out: &mut BTreeSet<String>) {
    for matrix in command.get("matrix").and_then(Value::as_array).into_iter().flatten() {
        let Some(actions) = matrix.get("actions").and_then(Value::as_table) else {
            continue;
        };
        for parent in matrix.get("parents").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
            for (action, spec) in actions {
                let (policy, guard) = match spec {
                    Value::String(p) => (p.as_str(), None),
                    other => (other.get("policy").and_then(Value::as_str).unwrap_or_default(), other.get("guard").and_then(Value::as_str)),
                };
                let mut node = Table::new();
                node.insert("policy".into(), Value::String(policy.to_string()));
                let prefix = format!("{name} {parent} {action}");
                node_invocations(&prefix, &node, policies, out);
                if let Some(guard) = guard {
                    node_invocations(&format!("{prefix} {guard} {PLACEHOLDER}"), &node, policies, out);
                }
            }
        }
    }
}

/// One `[[command]]` table: its own corpus, the same under each alias, and its examples.
pub fn command_invocations(command: &Table, out: &mut BTreeSet<String>) {
    let Some(name) = command.get("name").and_then(Value::as_str) else {
        return;
    };
    let empty = Table::new();
    let policies = command.get("handler_policy").and_then(Value::as_table).unwrap_or(&empty);
    let mut names = vec![name.to_string()];
    names.extend(strings(command, "aliases"));
    for invoked in &names {
        node_invocations(invoked, command, policies, out);
        matrix_invocations(invoked, command, policies, out);
        out.insert(format!("{invoked} --help"));
        out.insert(format!("{invoked} --version"));
        if let Some(verbs) = command.get("verb_chain").and_then(|c| c.get("verbs")).and_then(Value::as_array) {
            for verb in verbs.iter().filter_map(Value::as_str) {
                out.insert(format!("{invoked} {verb}"));
            }
        }
    }
    for key in ["examples_safe", "examples_denied"] {
        out.extend(strings(command, key));
    }
}

/// The registry's TOML files, sorted, `SAMPLE.toml` (documentation, not a command) left out.
pub fn command_files(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "toml") && path.file_name().is_some_and(|n| n != "SAMPLE.toml") {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    walk(&root.join("commands"), &mut files);
    files.sort();
    files
}

/// The registry-derived invocations, keyed by the file they came from so a test can count them.
pub fn registry_corpus(root: &Path) -> BTreeMap<PathBuf, BTreeSet<String>> {
    command_files(root)
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            let value: Table = toml::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
            let mut out = BTreeSet::new();
            for command in value.get("command").and_then(Value::as_array).into_iter().flatten() {
                if let Some(command) = command.as_table() {
                    command_invocations(command, &mut out);
                }
            }
            (path, out)
        })
        .collect()
}
