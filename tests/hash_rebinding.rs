//! `hash` can bind a bare name to any file, so every later use of that name runs the file instead of
//! the tool on PATH. Binding a name is refused for every documented command, in bash's and zsh's
//! spelling, so a command added later is covered without editing this file.

use safe_chains::command_verdict;

const SPELLINGS: &[&str] = &["hash -p /tmp/x/{n} {n}", "hash {n}=/tmp/x/{n}", "hash -p ./x/{n} {n}", "hash -r; hash -p /tmp/x/{n} {n}"];

#[test]
fn binding_any_command_name_with_hash_is_refused() {
    let mut approved = Vec::new();
    let mut checked = 0;
    for doc in safe_chains::docs::all_command_docs() {
        let Some(example) = doc.examples.iter().find(|e| command_verdict(e).is_allowed()) else { continue };
        for name in std::iter::once(&doc.name).chain(&doc.aliases).filter(|n| !n.contains('/')) {
            for spelling in SPELLINGS {
                let cmd = format!("{}; {example}", spelling.replace("{n}", name));
                checked += 1;
                if command_verdict(&cmd).is_allowed() {
                    approved.push(cmd);
                }
            }
        }
    }
    assert!(checked > 1000, "only {checked} rebindings were checked");
    assert!(approved.is_empty(), "{} hash rebindings were approved:\n{}", approved.len(), approved.join("\n"));
}

#[test]
fn reading_the_hash_table_stays_allowed() {
    for cmd in ["hash", "hash -r", "hash -l", "hash -d", "hash; ls", "hash -r && git status"] {
        assert!(command_verdict(cmd).is_allowed(), "should be allowed: {cmd}");
    }
}
