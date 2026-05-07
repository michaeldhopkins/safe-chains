use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::{InstallOutcome, Target};

pub struct OpenCodeTarget;

impl Target for OpenCodeTarget {
    fn name(&self) -> &'static str {
        "opencode"
    }

    fn display_name(&self) -> &'static str {
        "OpenCode"
    }

    fn detect_paths(&self, home: &Path) -> Vec<PathBuf> {
        vec![home.join(".config").join("opencode")]
    }

    fn install(&self, _home: &Path) -> Result<InstallOutcome, String> {
        Ok(InstallOutcome::Skipped {
            reason: "OpenCode integration generates `opencode.json` to stdout; \
                     run `safe-chains --opencode-config > opencode.json` from the project root"
                .to_string(),
        })
    }
}

pub fn render_opencode_json_in(dir: &Path, patterns: &[String]) -> String {
    let mut root: Map<String, Value> = std::fs::read_to_string(dir.join("opencode.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .and_then(|v: Value| v.as_object().cloned())
        .unwrap_or_else(|| {
            let mut m = Map::new();
            m.insert(
                "$schema".to_string(),
                Value::String("https://opencode.ai/config.json".to_string()),
            );
            m
        });

    let mut bash = Map::new();
    bash.insert("*".to_string(), Value::String("ask".to_string()));
    for pat in patterns {
        bash.insert(pat.clone(), Value::String("allow".to_string()));
    }

    let permission = root
        .entry("permission")
        .or_insert_with(|| Value::Object(Map::new()));
    if !permission.is_object() {
        *permission = Value::Object(Map::new());
    }
    if let Value::Object(perm_map) = permission {
        perm_map.insert("bash".to_string(), Value::Object(bash));
    }

    let mut out = serde_json::to_string_pretty(&Value::Object(root)).unwrap_or_default();
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_emits_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        let patterns = vec!["ls".to_string(), "git status".to_string()];
        let json = render_opencode_json_in(dir.path(), &patterns);
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed.pointer("/permission/bash/ls").and_then(|v| v.as_str()),
            Some("allow"),
        );
        assert_eq!(
            parsed.pointer("/permission/bash/*").and_then(|v| v.as_str()),
            Some("ask"),
        );
    }

    #[test]
    fn render_includes_schema() {
        let dir = tempfile::tempdir().unwrap();
        let json = render_opencode_json_in(dir.path(), &[]);
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed.get("$schema").and_then(|v| v.as_str()),
            Some("https://opencode.ai/config.json"),
        );
    }

    #[test]
    fn render_trailing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let json = render_opencode_json_in(dir.path(), &[]);
        assert!(json.ends_with('\n'));
    }

    #[test]
    fn render_merges_existing_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.json");
        std::fs::write(
            &path,
            r#"{"$schema": "https://opencode.ai/config.json", "model": "sonnet"}"#,
        )
        .unwrap();
        let json = render_opencode_json_in(dir.path(), &["ls".to_string()]);
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.get("model").and_then(|v| v.as_str()), Some("sonnet"));
        assert_eq!(
            parsed.pointer("/permission/bash/ls").and_then(|v| v.as_str()),
            Some("allow"),
        );
    }

    #[test]
    fn install_returns_skip_with_guidance() {
        let dir = tempfile::tempdir().unwrap();
        let outcome = OpenCodeTarget.install(dir.path()).unwrap();
        match outcome {
            InstallOutcome::Skipped { reason } => {
                assert!(reason.contains("opencode.json"));
            }
            other => panic!("expected Skipped, got {:?}", std::mem::discriminant(&other)),
        }
    }
}
