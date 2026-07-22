//! `--suggest`: generate the minimal custom-command TOML that would let safe-chains recognize a
//! command it currently denies because the command NAME is unknown.
//!
//! OPT-IN only. Nothing here is ever referenced by the deny/hook output — a user reaches it solely by
//! running `safe-chains --suggest "<cmd>"`. It rides the existing trust rails (`registry::custom`):
//! the generated `[[command]]` block goes in a project `.safe-chains.toml`, which is inert until the
//! user hand-pins its SHA-256 in the write-protected `~/.config/safe-chains.toml`. This module never
//! touches `~/`, so it cannot self-approve.
//!
//! Scope (v1): the "I don't recognize this tool" case. For each simple command whose name safe-chains
//! doesn't know, emit a `[[command]]` scoped to exactly the observed flags and positional count.
//! Commands safe-chains DOES recognize are never overridden — a recognized command that is denied is
//! a deliberate classification (a flag, subcommand, or path), not an unknown, so `--suggest` reports
//! that rather than generating a bypass.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use sha2::{Digest, Sha256};

use crate::cst::{self, Cmd, Script, SimpleCmd, Word, WordPart};
use crate::registry;

/// safe-chains cannot infer an unknown tool's real risk, so the generated entry defaults to the
/// developer band's ceiling — it auto-approves at the default level and the user narrows it (to
/// `SafeRead`/`Inert`) if the tool is lighter. The `~/` hash-pin review is the backstop.
const DEFAULT_LEVEL: &str = "SafeWrite";

/// A scoped custom-command entry for one unknown command name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedEntry {
    pub name: String,
    /// Observed flags, sorted and de-duplicated (every `-`-prefixed argument, verbatim).
    pub standalone: Vec<String>,
    /// The largest positional-argument count observed across occurrences.
    pub max_positional: usize,
    pub level: String,
}

/// What `analyze` concluded about a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Already auto-approves — nothing to suggest.
    AlreadyAllowed,
    /// Could not parse; a command definition can't fix a structural parse failure.
    Unparseable,
    /// Denied, but every command in it is one safe-chains RECOGNIZES — the block is a classification
    /// decision, not an unknown command. `--suggest` will not generate an override.
    RecognizedButDenied { names: Vec<String> },
    /// One or more unknown commands can be supported by the generated entries. `also_recognized`
    /// lists any recognized commands that also appear (informational).
    Generated {
        entries: Vec<GeneratedEntry>,
        also_recognized: Vec<String>,
    },
}

/// Analyze `command` and decide what, if anything, `--suggest` can generate. Pure — no I/O.
pub fn analyze(command: &str) -> Outcome {
    if cst::command_verdict(command).is_allowed() {
        return Outcome::AlreadyAllowed;
    }
    let Some(script) = cst::parse(command) else {
        return Outcome::Unparseable;
    };

    let mut simples: Vec<&SimpleCmd> = Vec::new();
    collect_script(&script, &mut simples);

    // name -> (observed flags, max observed positional count)
    let mut unknown: BTreeMap<String, (BTreeSet<String>, usize)> = BTreeMap::new();
    let mut recognized: BTreeSet<String> = BTreeSet::new();
    for sc in simples {
        let Some(name) = command_basename(sc) else {
            continue;
        };
        if is_known(&name) {
            recognized.insert(name);
            continue;
        }
        let (flags, positionals) = observed_shape(sc);
        let entry = unknown.entry(name).or_default();
        entry.0.extend(flags);
        entry.1 = entry.1.max(positionals);
    }

    if unknown.is_empty() {
        return Outcome::RecognizedButDenied {
            names: recognized.into_iter().collect(),
        };
    }
    let entries = unknown
        .into_iter()
        .map(|(name, (flags, max_positional))| GeneratedEntry {
            name,
            standalone: flags.into_iter().collect(),
            max_positional,
            level: DEFAULT_LEVEL.to_string(),
        })
        .collect();
    Outcome::Generated {
        entries,
        also_recognized: recognized.into_iter().collect(),
    }
}

/// The basename of a simple command's name word (`/usr/bin/foo` → `foo`), or `None` for an env-only
/// command with no name word.
fn command_basename(sc: &SimpleCmd) -> Option<String> {
    let raw = sc.words.first()?.eval();
    if raw.is_empty() {
        return None;
    }
    Some(crate::parse::Token::from_raw(raw).command_name().to_string())
}

/// The observed flags (each `-`-prefixed argument, but not a lone `-`) and the count of positional
/// arguments. A valued flag's value counts as a positional, which is fine: the generated grammar
/// still admits the observed invocation, just classified as flag + positional.
fn observed_shape(sc: &SimpleCmd) -> (Vec<String>, usize) {
    let mut flags = Vec::new();
    let mut positionals = 0;
    for word in sc.words.iter().skip(1) {
        let s = word.eval();
        if s.starts_with('-') && s != "-" {
            flags.push(s);
        } else {
            positionals += 1;
        }
    }
    (flags, positionals)
}

fn collect_script<'a>(script: &'a Script, out: &mut Vec<&'a SimpleCmd>) {
    for stmt in &script.0 {
        for cmd in &stmt.pipeline.commands {
            collect_cmd(cmd, out);
        }
    }
}

fn collect_cmd<'a>(cmd: &'a Cmd, out: &mut Vec<&'a SimpleCmd>) {
    match cmd {
        Cmd::Simple(sc) => {
            out.push(sc);
            for word in &sc.words {
                collect_word(word, out);
            }
        }
        Cmd::Subshell { body, .. } | Cmd::BraceGroup { body, .. } => collect_script(body, out),
        Cmd::For { items, body, .. } => {
            for word in items {
                collect_word(word, out);
            }
            collect_script(body, out);
        }
        Cmd::While { cond, body, .. } | Cmd::Until { cond, body, .. } => {
            collect_script(cond, out);
            collect_script(body, out);
        }
        Cmd::If {
            branches,
            else_body,
            ..
        } => {
            for branch in branches {
                collect_script(&branch.cond, out);
                collect_script(&branch.body, out);
            }
            if let Some(body) = else_body {
                collect_script(body, out);
            }
        }
        Cmd::DoubleBracket { words, .. } => {
            for word in words {
                collect_word(word, out);
            }
        }
    }
}

/// Recurse into command/process substitutions nested inside a word (`$(foo)`, `<(bar)`, and the same
/// inside double quotes), so an unknown command hidden in a substitution is surfaced too. Backticks
/// (`WordPart::Backtick`) hold an UNPARSED string, so an unknown command inside one is not surfaced —
/// harmless: the classifier still denies an unsafe backtick, so the command stays blocked (a coverage
/// gap, never a bypass). The user can `--suggest` the inner command directly.
fn collect_word<'a>(word: &'a Word, out: &mut Vec<&'a SimpleCmd>) {
    for part in &word.0 {
        match part {
            WordPart::CmdSub(s) | WordPart::ProcSub(s) => collect_script(s, out),
            WordPart::DQuote(w) => collect_word(w, out),
            _ => {}
        }
    }
}

/// Whether safe-chains recognizes `name` (built-in handler or registry command, via its canonical
/// spelling). Custom user commands that already ALLOW an invocation are caught earlier by the
/// `AlreadyAllowed` check, so they need not be enumerated here.
fn is_known(name: &str) -> bool {
    known_names().contains(registry::canonical_name(name))
}

fn known_names() -> &'static BTreeSet<String> {
    static KNOWN: OnceLock<BTreeSet<String>> = OnceLock::new();
    KNOWN.get_or_init(|| {
        let mut set: BTreeSet<String> = crate::docs::all_command_docs()
            .into_iter()
            .map(|d| d.name)
            .collect();
        for name in registry::toml_command_names() {
            set.insert(name.to_string());
        }
        set
    })
}

/// Render the generated entries as `[[command]]` TOML blocks. Minimal and always valid — this is
/// exactly the text written to `.safe-chains.toml` and fed to the SHA-256 pin, so it must be stable.
pub fn render_toml(entries: &[GeneratedEntry]) -> String {
    let mut out = String::new();
    for (i, entry) in entries.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str("[[command]]\n");
        out.push_str(&format!("name = {}\n", toml_str(&entry.name)));
        if !entry.standalone.is_empty() {
            let items: Vec<String> = entry.standalone.iter().map(|f| toml_str(f)).collect();
            out.push_str(&format!("standalone = [{}]\n", items.join(", ")));
        }
        out.push_str(&format!("max_positional = {}\n", entry.max_positional));
        out.push_str(&format!("level = {}\n", toml_str(&entry.level)));
    }
    out
}

/// A TOML basic (double-quoted) string with the control/quote/backslash escapes TOML requires, so
/// even an odd flag spelling produces valid TOML.
fn toml_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// SHA-256 of `bytes` as lowercase hex — the exact hash `registry::custom::repo_is_trusted` computes
/// over a `.safe-chains.toml`, so the printed pin matches what the trust check will require.
pub fn config_hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// The new `.safe-chains.toml` content: the existing file (empty if none) with the generated blocks
/// appended, separated by a blank line.
pub fn merged_content(existing: &str, entries: &[GeneratedEntry]) -> String {
    let block = render_toml(entries);
    if existing.trim().is_empty() {
        return block;
    }
    let mut content = existing.to_string();
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push('\n');
    content.push_str(&block);
    content
}

/// The `[[trusted]]` pin the user pastes into `~/.config/safe-chains.toml` to approve the file.
pub fn pin_block(canonical_dir: &str, hash: &str) -> String {
    format!(
        "[[trusted]]\npath = {}\nsha256 = {}\n",
        toml_str(canonical_dir),
        toml_str(hash),
    )
}

#[cfg(test)]
mod tests;
