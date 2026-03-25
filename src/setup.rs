use std::path::{Path, PathBuf};
use std::process;

use serde_json::{Map, Value, json};

fn claude_dir(home: &Path) -> PathBuf {
    home.join(".claude")
}

fn settings_path(home: &Path) -> PathBuf {
    claude_dir(home).join("settings.json")
}

fn hook_entry(binary: &str) -> Value {
    json!({
        "matcher": "Bash",
        "hooks": [{
            "type": "command",
            "command": binary,
        }]
    })
}

fn has_safe_chains_hook(settings: &Value) -> bool {
    settings
        .get("hooks")
        .and_then(|h| h.get("PreToolUse"))
        .and_then(|arr| arr.as_array())
        .is_some_and(|entries| {
            entries.iter().any(|entry| {
                entry
                    .get("hooks")
                    .and_then(|h| h.as_array())
                    .is_some_and(|hooks| {
                        hooks.iter().any(|hook| {
                            hook.get("command")
                                .and_then(|c| c.as_str())
                                .is_some_and(|cmd| cmd.contains("safe-chains"))
                        })
                    })
            })
        })
}

fn add_hook(settings: &mut Value, binary: &str) {
    if !settings.is_object() {
        *settings = json!({});
    }
    let Some(obj) = settings.as_object_mut() else {
        unreachable!("settings was just set to an object");
    };
    let hooks = obj
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .unwrap_or_else(|| {
            eprintln!("Error: \"hooks\" key in settings.json is not an object");
            process::exit(1);
        });
    let pre_tool_use = hooks
        .entry("PreToolUse")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .unwrap_or_else(|| {
            eprintln!("Error: \"hooks.PreToolUse\" in settings.json is not an array");
            process::exit(1);
        });
    pre_tool_use.push(hook_entry(binary));
}

pub fn run_setup_with_home(home: &Path) -> Result<String, String> {
    let dir = claude_dir(home);
    if !dir.exists() {
        return Err(format!(
            "~/.claude directory not found at {}. Install Claude Code first.",
            dir.display()
        ));
    }

    let binary = "safe-chains";
    let path = settings_path(home);

    if path.exists() {
        let contents = std::fs::read_to_string(&path)
            .map_err(|e| format!("Could not read {}: {e}", path.display()))?;
        let mut settings: Value = serde_json::from_str(&contents)
            .map_err(|e| format!("Could not parse {}: {e}", path.display()))?;

        if has_safe_chains_hook(&settings) {
            return Ok(format!(
                "safe-chains hook already configured in {}",
                path.display()
            ));
        }

        add_hook(&mut settings, binary);
        let output = serde_json::to_string_pretty(&settings).expect("serializing valid JSON");
        std::fs::write(&path, format!("{output}\n"))
            .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
        Ok(format!("safe-chains hook added to {}", path.display()))
    } else {
        let mut settings = Value::Object(Map::new());
        add_hook(&mut settings, binary);
        let output = serde_json::to_string_pretty(&settings).expect("serializing valid JSON");
        std::fs::write(&path, format!("{output}\n"))
            .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
        Ok(format!("Created {} with safe-chains hook", path.display()))
    }
}

pub fn run_setup() {
    let Some(home) = std::env::var_os("HOME") else {
        eprintln!("Error: HOME environment variable not set");
        process::exit(1);
    };
    match run_setup_with_home(Path::new(&home)) {
        Ok(msg) => println!("{msg}"),
        Err(msg) => {
            eprintln!("Error: {msg}");
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_claude_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = run_setup_with_home(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn no_settings_file_creates_it() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".claude")).unwrap();
        let result = run_setup_with_home(dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Created"));

        let contents = std::fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
        let settings: Value = serde_json::from_str(&contents).unwrap();
        assert!(has_safe_chains_hook(&settings));
    }

    #[test]
    fn existing_settings_without_hook() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir(&claude_dir).unwrap();
        std::fs::write(
            claude_dir.join("settings.json"),
            r#"{"permissions": {"allow": ["Bash(cargo test *)"]}}"#,
        )
        .unwrap();

        let result = run_setup_with_home(dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().contains("added"));

        let contents = std::fs::read_to_string(claude_dir.join("settings.json")).unwrap();
        let settings: Value = serde_json::from_str(&contents).unwrap();
        assert!(has_safe_chains_hook(&settings));
        assert!(
            settings
                .get("permissions")
                .and_then(|p| p.get("allow"))
                .is_some(),
            "existing content should be preserved"
        );
    }

    #[test]
    fn already_configured() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir(&claude_dir).unwrap();
        std::fs::write(
            claude_dir.join("settings.json"),
            r#"{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"safe-chains"}]}]}}"#,
        )
        .unwrap();

        let result = run_setup_with_home(dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().contains("already configured"));
    }

    #[test]
    fn already_configured_with_full_path() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir(&claude_dir).unwrap();
        std::fs::write(
            claude_dir.join("settings.json"),
            r#"{"hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"/opt/homebrew/bin/safe-chains"}]}]}}"#,
        )
        .unwrap();

        let result = run_setup_with_home(dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().contains("already configured"));
    }

    #[test]
    fn malformed_json() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("settings.json"), "not json{{{").unwrap();

        let result = run_setup_with_home(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Could not parse"));

        let contents = std::fs::read_to_string(claude_dir.join("settings.json")).unwrap();
        assert_eq!(contents, "not json{{{", "should not clobber malformed file");
    }

    #[test]
    fn idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir(&claude_dir).unwrap();

        let result1 = run_setup_with_home(dir.path());
        assert!(result1.is_ok());
        assert!(result1.unwrap().contains("Created"));

        let result2 = run_setup_with_home(dir.path());
        assert!(result2.is_ok());
        assert!(result2.unwrap().contains("already configured"));

        let contents = std::fs::read_to_string(claude_dir.join("settings.json")).unwrap();
        let settings: Value = serde_json::from_str(&contents).unwrap();
        let hooks = settings["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(hooks.len(), 1, "should not duplicate hook entry");
    }
}
