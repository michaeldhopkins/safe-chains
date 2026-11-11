//! A command named by a path outside the standard bin directories may be a planted binary that only
//! shares a safe tool's name, so it is refused, whatever the tool; the same name in a standard bin
//! directory keeps the bare name's verdict. Enumerated over every documented example, so a command
//! added later is covered without editing this file.

use safe_chains::command_verdict;
use safe_chains::verdict::Verdict;

const PLANTED: &[&str] =
    &["/tmp/x/", "/tmp/evil/", "~/Downloads/", "$HOME/Downloads/", "/opt/x/", "/usr/bin/../../tmp/", "/usr/local/bin/x/"];
const BIN_DIRS: &[&str] = &["/usr/bin/", "/bin/", "/opt/homebrew/bin/", "/usr/local/bin/"];
const WRAPPERS: &[&str] = &["env ", "env -i ", "nice ", "nohup ", "time ", "timeout 5 ", "command "];

struct Example {
    name: String,
    rest: String,
    bare: Verdict,
}

fn approved_examples() -> Vec<Example> {
    let mut out = Vec::new();
    for doc in safe_chains::docs::all_command_docs() {
        let names: Vec<&String> = std::iter::once(&doc.name).chain(&doc.aliases).filter(|n| !n.contains('/')).collect();
        for example in &doc.examples {
            let (first, rest) = example.split_once(' ').map_or((example.as_str(), String::new()), |(f, r)| (f, format!(" {r}")));
            let Some(name) = names.iter().find(|n| n.as_str() == first) else { continue };
            let bare = command_verdict(example);
            if bare.is_allowed() {
                out.push(Example { name: (*name).clone(), rest, bare });
            }
        }
    }
    out
}

#[test]
fn every_command_from_a_planted_path_is_refused() {
    let examples = approved_examples();
    let mut approved = Vec::new();
    for ex in &examples {
        for dir in PLANTED {
            let cmd = format!("{dir}{}{}", ex.name, ex.rest);
            if command_verdict(&cmd).is_allowed() {
                approved.push(cmd);
            }
        }
    }
    assert!(examples.len() > 1000, "only {} approved examples were found", examples.len());
    assert!(approved.is_empty(), "{} planted-path invocations were approved:\n{}", approved.len(), approved.join("\n"));
}

#[test]
fn every_command_from_a_standard_bin_directory_keeps_its_verdict() {
    let examples = approved_examples();
    let mut differ = Vec::new();
    for ex in &examples {
        for dir in BIN_DIRS {
            let cmd = format!("{dir}{}{}", ex.name, ex.rest);
            let verdict = command_verdict(&cmd);
            if verdict != ex.bare {
                differ.push(format!("{cmd}: {verdict:?}, bare {:?}", ex.bare));
            }
        }
    }
    assert!(examples.len() > 1000, "only {} approved examples were found", examples.len());
    assert!(differ.is_empty(), "{} bin-directory invocations differ from the bare name:\n{}", differ.len(), differ.join("\n"));
}

#[test]
fn no_wrapper_runs_a_planted_command() {
    let mut approved = Vec::new();
    for ex in approved_examples().iter().step_by(7) {
        for wrapper in WRAPPERS {
            let cmd = format!("{wrapper}/tmp/x/{}{}", ex.name, ex.rest);
            if command_verdict(&cmd).is_allowed() {
                approved.push(cmd);
            }
        }
    }
    assert!(approved.is_empty(), "{} wrapped planted-path invocations were approved:\n{}", approved.len(), approved.join("\n"));
}
