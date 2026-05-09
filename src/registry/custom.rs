use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::build::{insert_spec, load_toml};
use super::types::CommandSpec;

const REPO_FILENAME: &str = ".safe-chains.toml";
const USER_FILENAME: &str = "safe-chains.toml";

/// Walk up from CWD looking for a project-level custom TOML.
fn find_repo_custom() -> Option<PathBuf> {
    let mut dir = env::current_dir().ok()?;
    loop {
        let candidate = dir.join(REPO_FILENAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// $XDG_CONFIG_HOME/safe-chains.toml, falling back to ~/.config/safe-chains.toml.
fn find_user_custom() -> Option<PathBuf> {
    let dir = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    let candidate = dir.join(USER_FILENAME);
    candidate.is_file().then_some(candidate)
}

/// Apply user-level then repo-level custom TOMLs to the registry,
/// in that order so repo-level wins on conflicts.
pub(super) fn apply_custom(map: &mut HashMap<String, CommandSpec>) {
    if env::var_os("SAFE_CHAINS_NO_LOCAL").is_some() {
        return;
    }
    if let Some(path) = find_user_custom() {
        apply_file(map, &path, "custom-user");
    }
    if let Some(path) = find_repo_custom() {
        apply_file(map, &path, "custom-project");
    }
}

fn apply_file(map: &mut HashMap<String, CommandSpec>, path: &Path, category: &'static str) {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("safe-chains: failed to read {}: {e}", path.display());
            return;
        }
    };
    for spec in load_toml(&source, category) {
        insert_spec(map, spec);
    }
}
