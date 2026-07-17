use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::build::{insert_spec, load_toml};
use super::types::CommandSpec;

const REPO_FILENAME: &str = ".safe-chains.toml";
const USER_FILENAME: &str = "safe-chains.toml";

#[derive(Deserialize)]
struct TrustedEntry {
    path: String,
    sha256: String,
}

#[derive(Deserialize)]
struct TrustedConfig {
    #[serde(default)]
    trusted: Vec<TrustedEntry>,
    /// The user's chosen auto-approve CEILING (`level = "network-admin"`). Read ONLY from the
    /// write-protected user config (`~/.config/safe-chains.toml`) — never from a repo
    /// `.safe-chains.toml`, which the agent can write (raising a ceiling from a checked-out repo is
    /// exactly the self-escalation the config-write freeze prevents). Absent → the default band.
    #[serde(default)]
    level: Option<String>,
}

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

/// `~/.config/safe-chains.toml` — the ONLY user-config location. `XDG_CONFIG_HOME` is
/// deliberately NOT honored: it's an agent-mutable env var, and if a harness ever passed the
/// agent's environment to the hook, a redirected `XDG_CONFIG_HOME` could point the trust root at
/// an agent-writable directory (plant a "grant everything" config, load it as trusted). Reading
/// only from the real home directory closes that off — a common stance for a security-sensitive
/// CLI. Trades away XDG relocation until a protected third-party config location exists.
fn find_user_custom() -> Option<PathBuf> {
    let dir = env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))?;
    let candidate = dir.join(USER_FILENAME);
    candidate.is_file().then_some(candidate)
}

fn parse_trusted(source: &str) -> Vec<TrustedEntry> {
    toml::from_str::<TrustedConfig>(source)
        .map(|c| c.trusted)
        .unwrap_or_default()
}

fn parse_level(source: &str) -> Option<String> {
    toml::from_str::<TrustedConfig>(source).ok()?.level
}

/// The `level = "…"` ceiling from the USER config (`~/.config/safe-chains.toml`) only — the
/// write-protected location an agent cannot rewrite. Returns the raw name (the caller validates it
/// against the known levels; an unknown name falls back to the default band). `None` when no config,
/// no `level`, or local config is disabled (`SAFE_CHAINS_NO_LOCAL`). The repo file (`find_repo_custom`)
/// is NEVER consulted here — a repo `.safe-chains.toml` cannot raise the ceiling (the agent writes it).
pub(crate) fn user_config_level() -> Option<String> {
    if env::var_os("SAFE_CHAINS_NO_LOCAL").is_some() {
        return None;
    }
    let path = find_user_custom()?;
    let source = fs::read_to_string(&path).ok()?;
    parse_level(&source)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// A repo `.safe-chains.toml` is honored only when the user has pinned its
/// directory in the user config and the file's hash matches the pin. The
/// directory the agent works in is otherwise untrusted — it can write the file
/// freely, so reading it on sight would let an agent approve any command by
/// editing the file first. See `docs/design/trusted-customization.md`.
fn repo_is_trusted(repo_file: &Path, bytes: &[u8], trusted: &[TrustedEntry]) -> bool {
    let Some(parent) = repo_file.parent() else {
        return false;
    };
    let Ok(dir) = fs::canonicalize(parent) else {
        return false;
    };
    let hash = sha256_hex(bytes);
    trusted.iter().any(|t| {
        t.sha256.trim().eq_ignore_ascii_case(&hash)
            && fs::canonicalize(&t.path).map(|p| p == dir).unwrap_or(false)
    })
}

/// Apply user-level then repo-level custom TOMLs to the registry, in that order
/// so a trusted repo-level definition wins on conflicts. The user file
/// (`~/.config/safe-chains.toml`) is trusted as-is and also carries the
/// `[[trusted]]` list that pins repo files.
pub(super) fn apply_custom(map: &mut HashMap<String, CommandSpec>) {
    if env::var_os("SAFE_CHAINS_NO_LOCAL").is_some() {
        return;
    }

    let mut trusted = Vec::new();
    if let Some(path) = find_user_custom()
        && let Ok(source) = fs::read_to_string(&path)
    {
        for spec in load_toml(&source, "custom-user") {
            insert_spec(map, spec);
        }
        trusted = parse_trusted(&source);
    }

    if let Some(repo_file) = find_repo_custom()
        && let Ok(bytes) = fs::read(&repo_file)
        && repo_is_trusted(&repo_file, &bytes, &trusted)
        && let Ok(source) = std::str::from_utf8(&bytes)
    {
        for spec in load_toml(source, "custom-project") {
            insert_spec(map, spec);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn parse_trusted_reads_entries() {
        let src = r#"
            [[trusted]]
            path = "/a/b"
            sha256 = "abc123"

            [[trusted]]
            path = "/c/d"
            sha256 = "def456"
        "#;
        let t = parse_trusted(src);
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].path, "/a/b");
        assert_eq!(t[1].sha256, "def456");
    }

    #[test]
    fn parse_level_reads_the_ceiling() {
        assert_eq!(parse_level("level = \"network-admin\"").as_deref(), Some("network-admin"));
        // alongside trusted/commands still parses.
        assert_eq!(
            parse_level("level = \"yolo\"\n[[trusted]]\npath = \"/a\"\nsha256 = \"x\"\n").as_deref(),
            Some("yolo"),
        );
        // absent / malformed / empty → None (fail-safe to the default band).
        assert!(parse_level("[[trusted]]\npath = \"/a\"\nsha256 = \"x\"\n").is_none());
        assert!(parse_level("not valid toml {{{").is_none());
        assert!(parse_level("").is_none());
    }

    #[test]
    fn parse_trusted_absent_or_malformed_is_empty() {
        assert!(parse_trusted("[[command]]\nname = \"x\"").is_empty());
        assert!(parse_trusted("not valid toml {{{").is_empty());
        assert!(parse_trusted("").is_empty());
    }

    #[test]
    fn load_toml_tolerates_trusted_sections() {
        // A user config holding only [[trusted]] must parse to zero commands,
        // not panic on a missing `command` field.
        assert!(load_toml("[[trusted]]\npath = \"/a\"\nsha256 = \"x\"\n", "custom-user").is_empty());
        // command alongside trusted: command parsed, trusted ignored here.
        let specs = load_toml(
            "[[command]]\nname = \"myco\"\nbare = true\n\n[[trusted]]\npath = \"/a\"\nsha256 = \"x\"\n",
            "custom-user",
        );
        assert_eq!(specs.len(), 1);
    }

    fn write_repo_file(dir: &Path, body: &str) -> PathBuf {
        let f = dir.join(REPO_FILENAME);
        fs::write(&f, body).unwrap();
        f
    }

    #[test]
    fn repo_trusted_when_path_and_hash_match() {
        let dir = tempfile::tempdir().unwrap();
        let body = "[[command]]\nname = \"myco\"\n";
        let f = write_repo_file(dir.path(), body);
        let canon = fs::canonicalize(dir.path()).unwrap();
        let trusted = vec![TrustedEntry {
            path: canon.to_string_lossy().into_owned(),
            sha256: sha256_hex(body.as_bytes()),
        }];
        assert!(repo_is_trusted(&f, body.as_bytes(), &trusted));
    }

    #[test]
    fn repo_untrusted_when_hash_differs() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_repo_file(dir.path(), "[[command]]\nname = \"myco\"\n");
        let canon = fs::canonicalize(dir.path()).unwrap();
        let trusted = vec![TrustedEntry {
            path: canon.to_string_lossy().into_owned(),
            sha256: sha256_hex(b"different content"),
        }];
        // An agent rewrote the file after it was pinned: hash no longer matches.
        let tampered = b"[[command]]\nname = \"curl\"\nlevel = \"Inert\"\n";
        assert!(!repo_is_trusted(&f, tampered, &trusted));
    }

    #[test]
    fn repo_untrusted_when_path_not_listed() {
        let dir = tempfile::tempdir().unwrap();
        let body = "[[command]]\nname = \"myco\"\n";
        let f = write_repo_file(dir.path(), body);
        let trusted = vec![TrustedEntry {
            path: "/some/other/dir".to_string(),
            sha256: sha256_hex(body.as_bytes()),
        }];
        assert!(!repo_is_trusted(&f, body.as_bytes(), &trusted));
    }

    #[test]
    fn repo_untrusted_when_list_empty() {
        let dir = tempfile::tempdir().unwrap();
        let body = "[[command]]\nname = \"myco\"\n";
        let f = write_repo_file(dir.path(), body);
        assert!(!repo_is_trusted(&f, body.as_bytes(), &[]));
    }
}
