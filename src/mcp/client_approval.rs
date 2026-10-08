//! Whether the client running this server approves its editing tools without asking anyone.
//!
//! An editing tool that runs unasked lets an agent approve itself: grant its own path, add an entry
//! for its own command, then run it. Annotations ask a client to confirm, but they are hints, and a
//! client set to approve everything ignores them. So the server looks at the settings it can read
//! and refuses to edit when they say nobody will be asked, or when a settings file is there but
//! cannot be read: a safety check that cannot check refuses. What it cannot see (a command-line
//! flag that skips every prompt, a client whose settings it does not know) the documentation covers.

use std::path::{Path, PathBuf};

use serde_json::Value;

/// The tools that change the rules.
pub const EDITING_TOOLS: &[&str] = &["add_command_entry", "grant_path", "remove_rule"];

/// Where the client is set to run an editing tool unasked, in words, or `None` when nothing says so.
pub fn auto_approval(home: &Path, cwd: &Path) -> Option<String> {
    let claude_json = read(&home.join(".claude.json"));
    let names = match claude_json.as_deref().map(serde_json::from_str::<Value>) {
        Some(Ok(value)) => {
            if let Some(why) = claude_project_approval(&value, cwd) {
                return Some(format!("{why} in ~/.claude.json"));
            }
            server_names(&value)
        }
        Some(Err(_)) => return Some("~/.claude.json could not be read as JSON, so safe-chains cannot tell".into()),
        None => Vec::new(),
    };
    let mut files: Vec<(PathBuf, String)> = vec![
        (home.join(".claude/settings.json"), "~/.claude/settings.json".into()),
        (home.join(".claude/settings.local.json"), "~/.claude/settings.local.json".into()),
        (cwd.join(".claude/settings.json"), ".claude/settings.json".into()),
        (cwd.join(".claude/settings.local.json"), ".claude/settings.local.json".into()),
        (PathBuf::from("/Library/Application Support/ClaudeCode/managed-settings.json"), "the managed settings".into()),
    ];
    if let Some(dir) = std::env::var_os("CLAUDE_CONFIG_DIR") {
        let dir = PathBuf::from(dir);
        files.push((dir.join("settings.json"), format!("{}/settings.json", dir.display())));
    }
    for (path, shown) in files {
        let Some(text) = read(&path) else { continue };
        match serde_json::from_str::<Value>(&text) {
            Ok(value) => {
                if let Some(why) = claude_settings_approval(&value, &names) {
                    return Some(format!("{why} in {shown}"));
                }
            }
            Err(_) => return Some(format!("{shown} could not be read as JSON, so safe-chains cannot tell")),
        }
    }
    if let Some(text) = read(&home.join(".gemini/settings.json")) {
        match serde_json::from_str::<Value>(&text) {
            Ok(value) => {
                if let Some(name) = gemini_trusts(&value) {
                    return Some(format!("server `{name}` has `trust: true` in ~/.gemini/settings.json"));
                }
            }
            Err(_) => return Some("~/.gemini/settings.json could not be read as JSON, so safe-chains cannot tell".into()),
        }
    }
    let codex = read(&home.join(".codex/config.toml")).unwrap_or_default();
    codex_auto_approval(&codex).map(|why| format!("{why} in ~/.codex/config.toml"))
}

fn read(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// A Claude Code settings body that approves an editing tool without asking.
pub fn claude_auto_approval(settings: &str) -> Option<String> {
    claude_settings_approval(&serde_json::from_str(settings).ok()?, &[])
}

fn claude_settings_approval(value: &Value, names: &[String]) -> Option<String> {
    let permissions = value.get("permissions")?;
    if permissions.get("defaultMode").and_then(Value::as_str) == Some("bypassPermissions") {
        return Some("`defaultMode` is `bypassPermissions`".to_string());
    }
    rule_in(permissions.get("allow"), names).map(|rule| format!("`{rule}` is in permissions.allow"))
}

/// `projects[cwd].allowedTools` in `~/.claude.json`, where Claude Code keeps per-project approvals.
fn claude_project_approval(value: &Value, cwd: &Path) -> Option<String> {
    let project = value.get("projects")?.get(cwd.to_str()?)?;
    let names = server_names(value);
    rule_in(project.get("allowedTools"), &names).map(|rule| format!("`{rule}` is in this project's allowedTools"))
}

fn rule_in(list: Option<&Value>, names: &[String]) -> Option<String> {
    list?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .find(|rule| allows_an_editing_tool(rule, names))
        .map(String::from)
}

/// The names this server was registered under in `~/.claude.json`: any `mcpServers` entry that
/// runs safe-chains with `mcp`, at the top level or in a project.
fn server_names(value: &Value) -> Vec<String> {
    let mut maps: Vec<&serde_json::Map<String, Value>> = value.get("mcpServers").and_then(Value::as_object).into_iter().collect();
    if let Some(projects) = value.get("projects").and_then(Value::as_object) {
        maps.extend(projects.values().filter_map(|p| p.get("mcpServers").and_then(Value::as_object)));
    }
    maps.into_iter()
        .flat_map(|m| m.iter())
        .filter(|(_, server)| runs_this_server(server))
        .map(|(name, _)| name.clone())
        .collect()
}

fn runs_this_server(server: &Value) -> bool {
    let command = server.get("command").and_then(Value::as_str).unwrap_or_default();
    let args: Vec<&str> = server
        .get("args")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    command.ends_with("safe-chains") && args.contains(&"mcp")
}

/// A Gemini CLI server entry for safe-chains marked `trust: true`, which skips every confirmation.
fn gemini_trusts(value: &Value) -> Option<String> {
    value
        .get("mcpServers")?
        .as_object()?
        .iter()
        .find(|(_, server)| runs_this_server(server) && server.get("trust").and_then(Value::as_bool) == Some(true))
        .map(|(name, _)| name.clone())
}

/// `mcp__<server>__<tool>`, `mcp__<server>__*` or `mcp__<server>`. A rule naming an editing tool
/// counts whatever the server is called. A rule naming a whole server counts when that server's
/// name says safe-chains, or is one of `names`, the names it was registered under.
fn allows_an_editing_tool(rule: &str, names: &[String]) -> bool {
    let Some(rest) = rule.trim().strip_prefix("mcp__") else { return false };
    // By suffix, not by splitting: a server name may itself hold underscores (`mcp_____grant_path`
    // is server `_`), and splitting at the first `__` misread that rule as no tool at all.
    if EDITING_TOOLS.iter().any(|tool| rest.strip_suffix(tool).is_some_and(|s| s.ends_with("__"))) {
        return true;
    }
    let (server, tool) = rest.split_once("__").unwrap_or((rest, "*"));
    let squashed: String = server.chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_ascii_lowercase();
    (tool == "*" || tool.is_empty()) && (squashed.contains("safechains") || server == "*" || names.iter().any(|n| n == server))
}

/// A Codex `config.toml` that never asks before a tool call.
pub fn codex_auto_approval(config: &str) -> Option<String> {
    let value: toml::Value = toml::from_str(config).ok()?;
    (value.get("approval_policy").and_then(toml::Value::as_str) == Some("never")).then(|| "`approval_policy` is `never`".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rule_naming_an_editing_tool_counts_under_any_server_name() {
        for rule in ["mcp__safe-chains__grant_path", "mcp__sc__add_command_entry", "mcp__x__remove_rule"] {
            assert!(allows_an_editing_tool(rule, &[]), "{rule}");
        }
    }

    #[test]
    fn a_whole_server_rule_counts_only_for_safe_chains() {
        for rule in ["mcp__safe-chains", "mcp__safe_chains__*", "mcp__SafeChains", "mcp__*"] {
            assert!(allows_an_editing_tool(rule, &[]), "{rule}");
        }
        for rule in ["mcp__notes", "mcp__notes__*", "mcp__safe-chains__check_command", "mcp__safe-chains__list_rules", "Bash(ls:*)"] {
            assert!(!allows_an_editing_tool(rule, &[]), "{rule}");
        }
    }

    #[test]
    fn claude_settings_are_read_for_allow_rules_and_bypass_mode() {
        assert!(claude_auto_approval(r#"{"permissions":{"allow":["mcp__safe-chains__grant_path"]}}"#).is_some());
        assert!(claude_auto_approval(r#"{"permissions":{"defaultMode":"bypassPermissions"}}"#).is_some());
        assert!(claude_auto_approval(r#"{"permissions":{"allow":["mcp__safe-chains__check_command"],"defaultMode":"default"}}"#).is_none());
        assert!(claude_auto_approval("not json").is_none());
    }

    #[test]
    fn codex_never_asking_counts() {
        assert!(codex_auto_approval("approval_policy = \"never\"\n").is_some());
        assert!(codex_auto_approval("approval_policy = \"on-request\"\n").is_none());
        assert!(codex_auto_approval("").is_none());
    }

    #[test]
    fn files_are_found_at_home_and_in_the_project() {
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        assert!(auto_approval(home.path(), project.path()).is_none());
        std::fs::create_dir_all(project.path().join(".claude")).unwrap();
        std::fs::write(project.path().join(".claude/settings.local.json"), r#"{"permissions":{"allow":["mcp__safe-chains"]}}"#).unwrap();
        let found = auto_approval(home.path(), project.path()).unwrap();
        assert!(found.contains(".claude/settings.local.json"), "{found}");
    }
}

#[cfg(test)]
mod properties {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// A rule naming an editing tool counts whatever the server is called, and a rule naming a
        /// read-only tool never does.
        #[test]
        fn only_editing_tools_count(server in "[a-zA-Z0-9_-]{1,16}", tool in prop::sample::select(vec!["add_command_entry", "grant_path", "remove_rule", "check_command", "list_rules"])) {
            prop_assume!(!server.contains("__"));
            let rule = format!("mcp__{server}__{tool}");
            prop_assert_eq!(allows_an_editing_tool(&rule, &[]), EDITING_TOOLS.contains(&tool), "{}", rule);
        }
    }
}

#[cfg(test)]
mod review_findings {
    use super::*;

    fn home_with(files: &[(&str, &str)]) -> tempfile::TempDir {
        let home = tempfile::tempdir().unwrap();
        for (path, text) in files {
            let full = home.path().join(path);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(full, text).unwrap();
        }
        home
    }

    /// A whole-server rule for safe-chains registered under another name, learned from
    /// `~/.claude.json`.
    #[test]
    fn a_server_registered_under_another_name_is_recognised() {
        let claude_json = r#"{"mcpServers":{"rules":{"command":"/usr/local/bin/safe-chains","args":["mcp"]}}}"#;
        let home = home_with(&[(".claude.json", claude_json), (".claude/settings.json", r#"{"permissions":{"allow":["mcp__rules"]}}"#)]);
        let found = auto_approval(home.path(), Path::new("/nowhere")).unwrap();
        assert!(found.contains("mcp__rules"), "{found}");
        let unrelated = home_with(&[(".claude/settings.json", r#"{"permissions":{"allow":["mcp__rules"]}}"#)]);
        assert!(auto_approval(unrelated.path(), Path::new("/nowhere")).is_none(), "a server not known to be safe-chains");
    }

    #[test]
    fn a_project_allowlist_in_claude_json_counts() {
        let claude_json = r#"{"projects":{"/work/app":{"allowedTools":["mcp__x__grant_path"]}}}"#;
        let home = home_with(&[(".claude.json", claude_json)]);
        assert!(auto_approval(home.path(), Path::new("/work/app")).is_some());
        assert!(auto_approval(home.path(), Path::new("/work/other")).is_none());
    }

    /// A safety check that cannot read what it checks refuses rather than allows.
    #[test]
    fn settings_that_cannot_be_read_count_as_approving() {
        let home = home_with(&[(".claude/settings.json", "{ not json")]);
        assert!(auto_approval(home.path(), Path::new("/nowhere")).unwrap().contains("could not be read"));
    }

    #[test]
    fn a_gemini_server_marked_trusted_counts() {
        let gemini = r#"{"mcpServers":{"sc":{"command":"safe-chains","args":["mcp"],"trust":true}}}"#;
        let home = home_with(&[(".gemini/settings.json", gemini)]);
        assert!(auto_approval(home.path(), Path::new("/nowhere")).unwrap().contains("trust"));
    }
}
