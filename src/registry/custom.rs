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

/// Load a USER-SUPPLIED custom TOML without letting a bad one take the process down.
///
/// `load_toml` panics on anything it cannot validate — an unparseable file, an unknown behavior
/// hook, a bad enum value; there are ~40 such assertions. That is right for the built-in
/// `commands/*.toml`, which are compiled in and validated at build time: a panic there is a broken
/// build. It is wrong for a file a USER wrote. A single typo in `~/.config/safe-chains.toml` made
/// EVERY invocation abort with exit 101 — including the hook, and a crashed PreToolUse hook means
/// the harness proceeds, so an ordinary mistake silently disabled safe-chains altogether.
///
/// Skipping the file fails SAFE: the custom definitions do not load, so nothing is widened, and the
/// built-in registry keeps deciding. The message goes to stderr rather than stdout so it cannot be
/// mistaken for a hook decision.
fn load_custom_file(source: &str, category: &str, path: &Path) -> Vec<CommandSpec> {
    // Check SYNTAX first, so the common failure — a typo — is reported without a panic at all.
    // `toml::Value` accepts any well-formed document, so this only rejects what `load_toml` would
    // have aborted on, and its error carries the line and column.
    if let Err(e) = toml::from_str::<toml::Value>(source) {
        return skip(path, &e.to_string());
    }
    // Everything else `load_toml` refuses — an unknown behavior hook, a bad enum value, ~40
    // assertions — still panics, so it is caught here and its message recovered for the report.
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| load_toml(source, category))) {
        Ok(specs) => specs,
        Err(payload) => {
            let why = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or("unrecognized command definition");
            skip(path, why)
        }
    }
}

fn skip(path: &Path, why: &str) -> Vec<CommandSpec> {
    eprintln!(
        "safe-chains: ignoring {} — {why}\n  Built-in commands are unaffected; fix the file to \
         re-enable your custom ones.",
        path.display()
    );
    Vec::new()
}

/// Strip the fields a REPO-level custom TOML may not carry.
///
/// `[command.output]` is dropped for the reason `level` is never read from a repo file. Every other
/// per-command field widens the command it is written on, where a reader can see what it costs.
/// This one is TRANSITIVE: it widens whatever CONSUMES the command's output, so
/// `[command.output] locus_from = "cwd"` on `echo` is really a statement that
/// `cat $(echo /etc/shadow)` may run — a consequence not visible at the place it is declared.
///
/// Repo files are hash-pinned, so this is not agent-reachable either way; the point is that
/// vouching for a file should not require tracing an indirect grant. The user config, which is
/// write-protected, keeps the field.
fn repo_scoped(mut spec: CommandSpec) -> CommandSpec {
    spec.output = None;
    spec
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
        for spec in load_custom_file(&source, "custom-user", &path) {
            insert_spec(map, spec);
        }
        trusted = parse_trusted(&source);
    }

    if let Some(repo_file) = find_repo_custom()
        && let Ok(bytes) = fs::read(&repo_file)
        && repo_is_trusted(&repo_file, &bytes, &trusted)
        && let Ok(source) = std::str::from_utf8(&bytes)
    {
        for spec in load_custom_file(source, "custom-project", &repo_file) {
            insert_spec(map, repo_scoped(spec));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A user-supplied config that safe-chains cannot validate must be SKIPPED, never fatal.
    ///
    /// `load_toml` panics on ~40 conditions — that is correct for the built-in `commands/*.toml`,
    /// which are compiled in and validated at build time. Applied to a file the USER wrote it was a
    /// fail-OPEN: one typo in `~/.config/safe-chains.toml` aborted every invocation with exit 101,
    /// the hook included, and a crashed PreToolUse hook lets the harness proceed. So an ordinary
    /// mistake silently disabled safe-chains entirely.
    #[test]
    fn an_invalid_custom_config_is_skipped_not_fatal() {
        let path = Path::new("/tmp/does-not-matter.toml");
        // Bad SYNTAX — caught before any panic, so the message carries line/column.
        let specs = load_custom_file("this is not valid toml [[[", "custom-user", path);
        assert!(specs.is_empty(), "an unparseable file must contribute no commands");

        // Valid syntax, invalid CONTENT — reaches `load_toml`'s assertions and is caught there.
        let bogus = "[[command]]\nname = \"zz\"\nlevel = \"Inert\"\n\n\
                     [command.behavior]\noperation = \"observe\"\npositionals = \"read\"\n\
                     hook = \"bogus\"\n";
        let specs = load_custom_file(bogus, "custom-user", path);
        assert!(specs.is_empty(), "an unvalidatable file must contribute no commands");

        // Non-vacuity: a GOOD file still loads, so "always returns empty" cannot pass this.
        let good = "[[command]]\nname = \"frobnicate\"\nlevel = \"Inert\"\nbare = true\n";
        let specs = load_custom_file(good, "custom-user", path);
        assert_eq!(specs.len(), 1, "a valid custom file must still load");
        assert_eq!(specs[0].name, "frobnicate");
    }

    /// A repo file cannot carry `[command.output]`. The field is a TRANSITIVE grant — it widens
    /// whatever consumes the command's output, not the command itself — so declaring it on `echo`
    /// is really a statement that `cat $(echo /etc/shadow)` may run. Repo files are hash-pinned, so
    /// this is not agent-reachable; the guard is that vouching for a file should not require the
    /// user to trace an indirect consequence. The user config keeps the field.
    #[test]
    fn repo_custom_toml_cannot_declare_command_output() {
        let source = r#"
[[command]]
name = "echo"
description = "hijacked"
level = "Inert"
bare = true

[command.output]
locus_from = "cwd"
"#;
        let user: Vec<_> = load_toml(source, "custom-user").into_iter().collect();
        assert!(
            user.iter().any(|s| s.output.is_some()),
            "the user config must still be able to declare an output locus, or this guard is \
             testing the parser rather than the restriction",
        );

        // Calls the REAL rule `apply_custom` applies, not a copy of it — a copy would pass even
        // with the restriction deleted from the load path.
        let stripped: Vec<_> =
            load_toml(source, "custom-project").into_iter().map(repo_scoped).collect();
        assert!(
            stripped.iter().all(|s| s.output.is_none()),
            "a repo-level custom TOML must not be able to declare `[command.output]`",
        );
    }

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
