use super::types::{TomlCommand, TomlFile, TomlSub};
use crate::parse::Token;
use crate::verdict::Verdict;

const UNKNOWN: &str = "--zq-unknown";

#[derive(Default)]
struct Flags {
    standalone: Vec<String>,
    valued: Vec<String>,
}

impl Flags {
    fn forms(&self, value: impl Fn(&str) -> String) -> Vec<Vec<String>> {
        self.standalone
            .iter()
            .map(|f| vec![f.clone()])
            .chain(self.valued.iter().map(|f| vec![f.clone(), value(f)]))
            .collect()
    }
}

#[derive(PartialEq)]
enum Lists {
    Tolerant,
    Explicit,
    Own,
}

struct Scope {
    words: Vec<String>,
    patterns: Vec<String>,
    own: Flags,
    globals: Flags,
    lists: Lists,
    branching: bool,
}

fn toml_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).expect("read commands dir") {
        let p = e.expect("dir entry").path();
        if p.is_dir() {
            toml_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "toml") {
            out.push(p);
        }
    }
}

fn sub_scopes(words: &[String], list: &[TomlSub], globals: &Flags, out: &mut Vec<Scope>) {
    for sub in list {
        let mut path = words.to_vec();
        path.push(sub.name.clone());
        if !sub.first_arg.is_empty() && sub.policy.is_none() {
            out.push(Scope {
                words: path.clone(),
                patterns: sub.first_arg.clone(),
                own: Flags { standalone: sub.standalone.iter().chain(&sub.optional_valued).cloned().collect(), valued: sub.valued.clone() },
                globals: Flags { standalone: globals.standalone.clone(), valued: globals.valued.clone() },
                lists: if sub.tolerate_unknown_short.unwrap_or(false) || sub.tolerate_unknown_long.unwrap_or(false) {
                    Lists::Tolerant
                } else if !sub.first_arg_standalone.is_empty()
                    || !sub.first_arg_valued.is_empty()
                    || !sub.first_arg_loopback_valued.is_empty()
                {
                    Lists::Explicit
                } else {
                    Lists::Own
                },
                branching: !sub.sub.is_empty() || !sub.credential_first_arg.is_empty(),
            });
        }
        sub_scopes(&path, &sub.sub, globals, out);
    }
}

fn scopes(cmd: &TomlCommand, out: &mut Vec<Scope>) {
    let globals = cmd
        .wrapper
        .as_ref()
        .map_or_else(Flags::default, |w| Flags { standalone: w.standalone.clone(), valued: w.valued.clone() });
    sub_scopes(std::slice::from_ref(&cmd.name), &cmd.sub, &globals, out);
}

fn verdict(words: &[String]) -> Verdict {
    let tokens: Vec<Token> = words.iter().map(|w| Token::from_test(w)).collect();
    super::toml_dispatch(&tokens).unwrap_or(Verdict::Denied)
}

fn line(base: &[String], parts: &[&[String]]) -> Vec<String> {
    base.iter().cloned().chain(parts.iter().flat_map(|p| p.iter().cloned())).collect()
}

fn admitted_arg(scope: &Scope) -> Option<String> {
    scope.patterns.iter().find_map(|p| {
        let arg = p.strip_suffix('*').map_or_else(|| p.clone(), |prefix| format!("{prefix}zq"));
        verdict(&line(&scope.words, &[std::slice::from_ref(&arg)])).is_allowed().then_some(arg)
    })
}

fn value_for(_scope: &Scope, _flag: &str) -> String {
    "zv".to_string()
}

fn check_scope(scope: &Scope, failures: &mut Vec<String>) -> bool {
    let Some(arg) = admitted_arg(scope) else { return false };
    let arg = [arg];
    let name = scope.words.join(" ");
    let researched = !scope.own.standalone.is_empty() || !scope.own.valued.is_empty();
    if scope.lists == Lists::Tolerant || (scope.lists == Lists::Own && !researched) {
        return true;
    }
    for unknown in [UNKNOWN.to_string(), format!("{UNKNOWN}=v")] {
        if verdict(&line(&scope.words, &[&arg, std::slice::from_ref(&unknown)])).is_allowed() {
            failures.push(format!("  {name} {} {unknown}: an unlisted flag after the argument is approved", arg[0]));
        }
    }
    if scope.lists == Lists::Explicit {
        return true;
    }
    for flag in scope.own.forms(|f| value_for(scope, f)).iter().chain(&scope.globals.forms(|f| value_for(scope, f))) {
        let after = verdict(&line(&scope.words, &[&arg, flag]));
        if !after.is_allowed() {
            failures.push(format!("  {name} {} {}: a listed flag after the argument is refused", arg[0], flag.join(" ")));
        }
    }
    for flag in scope.own.forms(|f| value_for(scope, f)) {
        if scope.branching && !matches!(flag[0].as_str(), "--help" | "-h") {
            let after = verdict(&line(&scope.words, &[&arg, &flag]));
            let before = verdict(&line(&scope.words, &[&flag, &arg]));
            if before != after {
                failures.push(format!("  {name}: `{}` judged {before:?} before `{}` and {after:?} after", flag.join(" "), arg[0]));
            }
        }
    }
    true
}

#[test]
fn every_glob_scope_judges_flags_after_its_argument() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("commands");
    let mut files = Vec::new();
    toml_files(&root, &mut files);
    let mut all = Vec::new();
    for f in &files {
        let parsed: TomlFile = toml::from_str(&std::fs::read_to_string(f).expect("read toml")).expect("parse toml");
        for cmd in &parsed.command {
            scopes(cmd, &mut all);
        }
    }
    let mut failures = Vec::new();
    let checked = all.iter().filter(|s| check_scope(s, &mut failures)).count();
    assert!(checked > 100, "only {checked} glob scopes reached — the registry walk is broken");
    assert!(failures.is_empty(), "glob scopes that misjudge flags after their argument:\n{}", failures.join("\n"));
}

#[test]
fn a_flag_after_the_resource_is_judged_like_one_before_it() {
    for c in [
        "kubectl get pods --server https://evil.example", "kubectl get pods -s https://evil.example",
        "kubectl get pods --kubeconfig=/tmp/k", "kubectl get pods --token=abc", "kubectl get pods --as=system:admin",
        "kubectl get pods --bogus", "scw instance volume list --zq-unknown",
    ] {
        assert!(!crate::is_safe_command(c), "must refuse: {c}");
    }
    for c in [
        "kubectl get pods", "kubectl get pods -o yaml", "kubectl get pods -n x -o wide", "kubectl get -n secrets pods",
        "scw instance volume list -o json", "exo compute instance list -O json", "vultr-cli dns domain list --help",
    ] {
        assert!(crate::is_safe_command(c), "must approve: {c}");
    }
}
