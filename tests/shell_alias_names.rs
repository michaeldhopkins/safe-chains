//! A command name that a common shell alias also claims is judged as the wrong program.
//!
//! safe-chains judges the word it is given and never reads the user's shell aliases: doing so would
//! mean starting the user's interactive shell, running its startup files, on every check. So a
//! registry name or alias must not be one that a widely installed alias set redefines. The
//! oh-my-zsh git plugin maps `gcp` to `git cherry-pick` and `grm` to `git rm`; had `gcp` stayed a
//! Homebrew spelling of `cp`, `gcp a b` would have been approved as a copy while the shell ran a
//! cherry-pick.

use std::collections::BTreeSet;

const ALIASES: &str = "tests/fixtures/shell_alias_names.txt";

fn alias_names() -> BTreeSet<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(ALIASES);
    std::fs::read_to_string(path)
        .expect("alias fixture")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}

fn registry_names() -> BTreeSet<String> {
    let mut names: BTreeSet<String> = safe_chains::registry::toml_command_names().into_iter().map(str::to_string).collect();
    for doc in safe_chains::docs::all_command_docs() {
        names.insert(doc.name);
        names.extend(doc.aliases);
    }
    names
}

#[test]
fn the_fixture_holds_the_plugin_aliases() {
    let aliases = alias_names();
    assert!(aliases.len() > 150, "{} names read", aliases.len());
    for name in ["g", "gcp", "grm", "gpr", "grev", "gst"] {
        assert!(aliases.contains(name), "{name} missing from {ALIASES}");
    }
}

#[test]
fn the_registry_names_are_read() {
    let names = registry_names();
    for name in ["cp", "rm", "gcat", "gsed", "git"] {
        assert!(names.contains(name), "{name} missing from the registry listing");
    }
}

/// A program's own name that an alias set also claims. Dropping the program would leave its users
/// with no coverage, so each is kept by name, with the reason, and listed in TODO.md.
const KEPT: [(&str, &str); 1] = [("gs", "Ghostscript's own name; Prezto's `gs` is `git stash` (TODO.md)")];

#[test]
fn no_registry_name_is_a_common_shell_alias() {
    let registry = registry_names();
    let clashes: Vec<String> = alias_names()
        .into_iter()
        .filter(|a| registry.contains(a) && !KEPT.iter().any(|(k, _)| k == a))
        .collect();
    assert!(clashes.is_empty(), "registry names a shell alias redefines: {clashes:?}");
}

#[test]
fn every_kept_clash_is_still_a_clash() {
    let (registry, aliases) = (registry_names(), alias_names());
    for (name, why) in KEPT {
        assert!(registry.contains(name) && aliases.contains(name), "{name} no longer clashes; drop it from KEPT ({why})");
    }
}

#[test]
fn an_aliased_name_is_not_approved_as_the_registry_program() {
    for command in ["gcp a.txt b.txt", "grm a.txt", "gpr", "grev"] {
        assert!(!safe_chains::is_safe_command(command), "{command} was approved");
    }
}
