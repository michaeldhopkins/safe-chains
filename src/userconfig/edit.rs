//! The edits themselves, from the file's text to the new text. No filesystem here.
//!
//! `toml_edit` keeps everything an edit does not touch exactly as it was, comments and spacing
//! included. An added entry is appended, so it never lands between a table and its sub-tables.

use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, value};

use super::{EditError, SHOWN_PATH, Step, guard, step, surgery};
use crate::suggest::{self, GeneratedEntry, Outcome};

/// Which faces a grant opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
    ReadWrite,
}

impl Access {
    pub fn from_flags(read: bool, write: bool) -> Result<Self, EditError> {
        match (read, write) {
            (true, true) => Ok(Access::ReadWrite),
            (true, false) => Ok(Access::Read),
            (false, true) => Ok(Access::Write),
            (false, false) => Err(EditError::NoAccess),
        }
    }

    /// As the MCP `access` argument spells it.
    pub fn name(self) -> &'static str {
        match self {
            Access::Read => "read",
            Access::Write => "write",
            Access::ReadWrite => "read_write",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Access::Read, Access::Write, Access::ReadWrite].into_iter().find(|a| a.name() == name)
    }

    /// As `safe-chains config grant` takes it.
    pub fn flags(self) -> &'static str {
        match self {
            Access::Read => "--read",
            Access::Write => "--write",
            Access::ReadWrite => "--read --write",
        }
    }

    /// The faces in words.
    pub fn faces(self) -> &'static str {
        Access::describe(self.read(), self.write())
    }

    fn read(self) -> bool {
        matches!(self, Access::Read | Access::ReadWrite)
    }

    fn write(self) -> bool {
        matches!(self, Access::Write | Access::ReadWrite)
    }

    fn describe(read: bool, write: bool) -> &'static str {
        match (read, write) {
            (true, true) => "read and write",
            (false, true) => "write",
            _ => "read",
        }
    }
}

/// Refuse text safe-chains would not load: invalid TOML, a command definition it rejects, or a
/// grant it cannot read. Writing such a file would silently switch off every rule in it.
pub fn check_loads(source: &str) -> Result<(), EditError> {
    if source.trim().is_empty() {
        return Ok(());
    }
    let parsed: toml::Value = toml::from_str(source).map_err(|e| EditError::DoesNotLoad(first_line(&e.to_string())))?;
    crate::registry::load_toml(source, "custom-user").map_err(|e| EditError::DoesNotLoad(first_line(&e)))?;
    let grants = match parsed.get("grant") {
        None => return Ok(()),
        Some(toml::Value::Array(grants)) => grants,
        Some(_) => return Err(EditError::DoesNotLoad("`grant` must be a list of [[grant]] tables".into())),
    };
    for grant in grants {
        let has_path = grant.get("path").is_some_and(toml::Value::is_str);
        let flags_ok = ["read", "write"].iter().all(|k| grant.get(k).is_none_or(toml::Value::is_bool));
        if !has_path || !flags_ok {
            return Err(EditError::DoesNotLoad("every [[grant]] needs a path, and read and write must be true or false".into()));
        }
    }
    Ok(())
}

fn first_line(text: &str) -> String {
    text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("unknown error").to_string()
}

fn document(source: &str) -> Result<DocumentMut, EditError> {
    source.parse::<DocumentMut>().map_err(|e| EditError::DoesNotLoad(first_line(&e.to_string())))
}

fn names_in(doc: &DocumentMut, key: &str, field: &str) -> Vec<String> {
    doc.get(key)
        .and_then(Item::as_array_of_tables)
        .map(|tables| tables.iter().filter_map(|t| t.get(field).and_then(Item::as_str)).map(String::from).collect())
        .unwrap_or_default()
}

/// Append the entry `suggest` drafts for `command`, at `level`.
pub(super) fn add_entry(existing: &str, command: &str, level: &str) -> Result<(String, String), EditError> {
    if command.trim().is_empty() {
        return Err(EditError::Unparseable);
    }
    let (mut entries, also_recognized) = match suggest::analyze(command) {
        Outcome::AlreadyAllowed => return Err(EditError::AlreadyApproved),
        Outcome::Unparseable => return Err(EditError::Unparseable),
        Outcome::RecognizedButDenied { names } => return Err(EditError::BuiltIn(names)),
        Outcome::Generated { entries, also_recognized } => (entries, also_recognized),
    };
    let present = names_in(&document(existing)?, "command", "name");
    for entry in &mut entries {
        guard::check_entry_name(&entry.name)?;
        guard::check_spelling(command, entry)?;
        if present.contains(&entry.name) {
            return Err(EditError::AlreadyHasEntry(entry.name.clone()));
        }
        entry.level = level.to_string();
    }
    let updated = surgery::append(existing, &suggest::render_toml(&entries));
    let mut message = format!("Added to {SHOWN_PATH}:\n\n{}\n", suggest::render_toml(&entries));
    for entry in &entries {
        message.push_str(&format!("{}\n", describe(entry)));
    }
    message.push_str(if entries.len() == 1 {
        "safe-chains approves it from the next command on."
    } else {
        "safe-chains approves them from the next command on."
    });
    if !also_recognized.is_empty() {
        message.push_str(&format!(
            " The command also runs {}, which safe-chains already knows. If it is still not approved, \
             run {} to see why.",
            also_recognized.iter().map(|n| format!("`{n}`")).collect::<Vec<_>>().join(", "),
            step(Step::Explain)
        ));
    }
    for entry in &entries {
        message.push_str(&format!("\nTo undo it: {}", step(Step::RemoveCommand(&entry.name))));
    }
    Ok((updated, message))
}

/// What an entry approves, in words.
fn describe(entry: &GeneratedEntry) -> String {
    let shape = |flags: &[String], max: usize| {
        let flags = match flags {
            [] => "no flags".to_string(),
            [one] => format!("the flag {one}"),
            many => format!("the flags {}", many.join(" ")),
        };
        let args = match max {
            0 => "no other arguments".to_string(),
            1 => "one other argument of any value".to_string(),
            n => format!("up to {n} other arguments of any value"),
        };
        format!("{flags} and {args}")
    };
    if entry.subs.is_empty() {
        return format!("It approves `{}` with {}.", entry.name, shape(&entry.standalone, entry.max_positional));
    }
    let subs: Vec<String> = entry
        .subs
        .iter()
        .map(|s| format!("`{} {}` with {}", entry.name, s.name, shape(&s.standalone, s.max_positional)))
        .collect();
    format!("It approves {}.", subs.join(", and "))
}

pub(super) fn remove_command(existing: &str, name: &str) -> Result<(String, String), EditError> {
    let mut doc = document(existing)?;
    let present = names_in(&doc, "command", "name");
    if !present.iter().any(|p| p == name) {
        return Err(EditError::NotFound { kind: "command entry", key: name.to_string(), present });
    }
    let keep = |t: &Table| t.get("name").and_then(Item::as_str) != Some(name);
    let doomed = doomed(&doc, "command", &keep);
    retain(&mut doc, "command", keep);
    let updated = cut_elements(existing, "command", &doomed, &doc)?;
    let message = format!("Removed the entry for `{name}` from {SHOWN_PATH}. safe-chains stops approving it from the next command on.");
    Ok((updated, message))
}

pub(super) fn remove_grant(existing: &str, path: &str) -> Result<(String, String), EditError> {
    let mut doc = document(existing)?;
    let present = names_in(&doc, "grant", "path");
    if !present.iter().any(|p| same_path(p, path)) {
        return Err(EditError::NotFound { kind: "grant", key: path.to_string(), present });
    }
    let keep = |t: &Table| !t.get("path").and_then(Item::as_str).is_some_and(|p| same_path(p, path));
    let doomed = doomed(&doc, "grant", &keep);
    retain(&mut doc, "grant", keep);
    let updated = cut_elements(existing, "grant", &doomed, &doc)?;
    let message = format!("Removed the grant on `{path}` from {SHOWN_PATH}. It stops applying from the next command on.");
    Ok((updated, message))
}

/// The positions, in file order, of the elements of `key` that `keep` would drop.
fn doomed(doc: &DocumentMut, key: &str, keep: &impl Fn(&Table) -> bool) -> Vec<usize> {
    doc.get(key)
        .and_then(Item::as_array_of_tables)
        .map(|tables| tables.iter().enumerate().filter(|(_, t)| !keep(t)).map(|(i, _)| i).collect())
        .unwrap_or_default()
}

fn retain(doc: &mut DocumentMut, key: &str, keep: impl Fn(&Table) -> bool) {
    let emptied = match doc.get_mut(key).and_then(Item::as_array_of_tables_mut) {
        Some(tables) => {
            tables.retain(|t| keep(t));
            tables.is_empty()
        }
        None => false,
    };
    if emptied {
        doc.remove(key);
    }
}

/// Cut the doomed elements out of the text, and confirm the result means what the same change
/// made to the parsed document means. If it does not, the file is left alone.
fn cut_elements(existing: &str, key: &str, doomed: &[usize], expected: &DocumentMut) -> Result<String, EditError> {
    let ranges = surgery::elements(existing, key).ok_or(EditError::CannotEditCleanly)?;
    let chosen: Vec<_> = doomed.iter().filter_map(|&i| ranges.get(i).cloned()).collect();
    if chosen.len() != doomed.len() {
        return Err(EditError::CannotEditCleanly);
    }
    same_meaning(surgery::cut(existing, &chosen), expected)
}

fn same_meaning(text: String, expected: &DocumentMut) -> Result<String, EditError> {
    let got = toml::from_str::<toml::Value>(&text).map_err(|_| EditError::CannotEditCleanly)?;
    let want = toml::from_str::<toml::Value>(&expected.to_string()).map_err(|_| EditError::CannotEditCleanly)?;
    // Compared as written out, not with `==`: a `nan` in the file is never equal to itself, and
    // fuzzing found that refusing every edit of a file holding one.
    if toml::to_string(&got).ok() == toml::to_string(&want).ok() { Ok(text) } else { Err(EditError::CannotEditCleanly) }
}

fn same_path(written: &str, normalized: &str) -> bool {
    guard::normalize_path(written).is_ok_and(|w| w == normalized)
}

pub(super) fn grant(existing: &str, path: &str, access: Access) -> Result<(String, String), EditError> {
    let mut doc = document(existing)?;
    if doc.get("grant").is_none() {
        doc.insert("grant", Item::ArrayOfTables(ArrayOfTables::new()));
    }
    let Some(grants) = doc.get_mut("grant").and_then(Item::as_array_of_tables_mut) else {
        return Err(EditError::CannotEditCleanly);
    };
    let found = grants
        .iter_mut()
        .enumerate()
        .find(|(_, t)| t.get("path").and_then(Item::as_str).is_some_and(|p| same_path(p, path)));
    let (read, write, before, text) = match found {
        Some((index, table)) => {
            let flag = |t: &Table, k: &str| t.get(k).and_then(Item::as_bool).unwrap_or(false);
            let before = (flag(table, "read"), flag(table, "write"));
            let now = (before.0 || access.read(), before.1 || access.write());
            table["read"] = value(now.0);
            table["write"] = value(now.1);
            let text = if now == before {
                existing.to_string()
            } else {
                surgery::set_flags(existing, "grant", index, &[("read", now.0), ("write", now.1)]).ok_or(EditError::CannotEditCleanly)?
            };
            (now.0, now.1, Some(before), text)
        }
        None => {
            let mut table = Table::new();
            table["path"] = value(path);
            table["read"] = value(access.read());
            table["write"] = value(access.write());
            grants.push(table);
            let block =
                format!("[[grant]]\npath = {}\nread = {}\nwrite = {}\n", crate::suggest::toml_str(path), access.read(), access.write());
            (access.read(), access.write(), None, surgery::append(existing, &block))
        }
    };
    if doc.get("grant").and_then(Item::as_array_of_tables).is_some_and(ArrayOfTables::is_empty) {
        doc.remove("grant");
    }
    let text = same_meaning(text, &doc)?;
    let faces = Access::describe(read, write);
    let message = match before {
        Some(before) if before == (read, write) => {
            format!("{SHOWN_PATH} already grants {faces} access to `{path}`. Nothing needed changing.")
        }
        _ => format!(
            "Granted {faces} access to `{path}` in {SHOWN_PATH}. It applies from the next command on. Anywhere it does not cover still asks.\n\
             To undo it: {}",
            step(Step::RemoveGrant(path))
        ),
    };
    Ok((text, message))
}
