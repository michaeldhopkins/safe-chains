//! `safe-chains config list`: every rule safe-chains reads from a file, under the file it came from.

use std::fmt::Write as _;
use std::path::Path;

use super::SHOWN_PATH;

/// The listing, ready to print. Reads the files afresh every call.
pub fn list() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut out = String::new();
    if std::env::var_os("SAFE_CHAINS_NO_LOCAL").is_some() {
        out.push_str("SAFE_CHAINS_NO_LOCAL is set, so safe-chains is ignoring every file below and using only its built-in rules.\n\n");
    }
    let user = std::fs::read_to_string(Path::new(&home).join(".config/safe-chains.toml")).unwrap_or_default();
    out.push_str(SHOWN_PATH);
    out.push('\n');
    out.push_str(&user_rules(&user));

    if let Some((path, trusted)) = crate::registry::custom::project_config() {
        let shown = crate::sanitize_display(&tilde(&path.to_string_lossy(), &home));
        let status = if trusted {
            "trusted, so its rules apply here"
        } else {
            "not trusted, so safe-chains ignores it. A [[trusted]] pin in ~/.config/safe-chains.toml with its current hash would trust it"
        };
        let _ = writeln!(out, "\n{shown} ({status})");
        out.push_str(&command_rules(&std::fs::read_to_string(&path).unwrap_or_default()));
    }

    let claude = std::fs::read_to_string(Path::new(&home).join(".claude/settings.json")).unwrap_or_default();
    let borrowed = claude_rules(&claude);
    if !borrowed.is_empty() {
        out.push_str("\n~/.claude/settings.json (used only when Claude Code runs safe-chains)\n");
        out.push_str(&borrowed);
    }

    let _ = writeln!(out, "\nBuilt in: {} commands. `safe-chains --list-commands` lists them.", crate::docs::all_command_docs().len());
    let _ = writeln!(
        out,
        "To remove a rule from {SHOWN_PATH}: {}, or {}.",
        super::step(super::Step::RemoveCommand("NAME")),
        super::step(super::Step::RemoveGrant("PATH"))
    );
    out
}

fn tilde(path: &str, home: &str) -> String {
    match path.strip_prefix(home) {
        Some(rest) if !home.is_empty() && rest.starts_with('/') => format!("~{rest}"),
        _ => path.to_string(),
    }
}

fn user_rules(source: &str) -> String {
    let Ok(doc) = toml::from_str::<toml::Value>(source) else {
        return if source.trim().is_empty() {
            "  (no file, so no rules)\n".to_string()
        } else {
            "  (does not load as TOML, so safe-chains ignores every rule in it)\n".to_string()
        };
    };
    let mut out = String::new();
    if let Some(level) = doc.get("level").and_then(toml::Value::as_str) {
        let _ = writeln!(out, "  level {}", crate::sanitize_display(level));
    }
    out.push_str(&command_rules(source));
    for grant in tables(&doc, "grant") {
        let path = grant.get("path").and_then(toml::Value::as_str).unwrap_or("(no path)");
        let flag = |k: &str| grant.get(k).and_then(toml::Value::as_bool).unwrap_or(false);
        let faces = match (flag("read"), flag("write")) {
            (true, true) => "read and write",
            (true, false) => "read",
            (false, true) => "write",
            (false, false) => "nothing (neither read nor write is set)",
        };
        let _ = writeln!(out, "  grant {}: {faces}", crate::sanitize_display(path));
    }
    for trusted in tables(&doc, "trusted") {
        let path = trusted.get("path").and_then(toml::Value::as_str).unwrap_or("(no path)");
        let _ = writeln!(out, "  trusted {} (its .safe-chains.toml, at the pinned hash)", crate::sanitize_display(path));
    }
    if out.is_empty() {
        out.push_str("  (no rules)\n");
    }
    out
}

fn tables<'a>(doc: &'a toml::Value, key: &str) -> impl Iterator<Item = &'a toml::Value> {
    doc.get(key).and_then(toml::Value::as_array).into_iter().flatten()
}

fn command_rules(source: &str) -> String {
    let Ok(doc) = toml::from_str::<toml::Value>(source) else {
        return String::new();
    };
    let mut out = String::new();
    for command in tables(&doc, "command") {
        let name = crate::sanitize_display(command.get("name").and_then(toml::Value::as_str).unwrap_or("(no name)"));
        if command.get("deny").and_then(toml::Value::as_bool) == Some(true) {
            let _ = writeln!(out, "  command {name}: never approved (deny = true)");
            continue;
        }
        let level = command.get("level").and_then(toml::Value::as_str).unwrap_or("SafeRead");
        let subs: Vec<String> = tables(command, "sub")
            .filter_map(|s| s.get("name").and_then(toml::Value::as_str))
            .map(crate::sanitize_display)
            .collect();
        match subs.as_slice() {
            [] => {
                let _ = writeln!(out, "  command {name}: {}", crate::sanitize_display(level));
            }
            subs => {
                let _ = writeln!(out, "  command {name}: subcommands {}", subs.join(", "));
            }
        }
    }
    out
}

fn claude_rules(settings: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(settings) else {
        return String::new();
    };
    let mut out = String::new();
    let lists = [value.get("approved_commands"), value.get("permissions").and_then(|p| p.get("allow"))];
    for entry in lists
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_array)
        .flatten()
        .filter_map(serde_json::Value::as_str)
    {
        if entry.starts_with("Bash(") {
            let _ = writeln!(out, "  allow {}", crate::sanitize_display(entry));
        }
    }
    for path in crate::engine::resolve::regions::hypothetical::claude_read_grants(settings) {
        let _ = writeln!(out, "  grant {}: read (from a Read rule)", crate::sanitize_display(&path));
    }
    out
}
