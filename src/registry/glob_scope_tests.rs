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
    choices: std::collections::HashMap<String, Vec<String>>,
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
                choices: sub.flag_values.clone(),
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

fn value_for(scope: &Scope, flag: &str) -> String {
    scope
        .choices
        .get(flag)
        .and_then(|c| c.first())
        .map_or_else(|| "zv".to_string(), |c| c.trim_end_matches('*').to_string())
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

fn subs_with_values<'a>(words: &[String], list: &'a [TomlSub], out: &mut Vec<(Vec<String>, &'a TomlSub)>) {
    for sub in list {
        let mut path = words.to_vec();
        path.push(sub.name.clone());
        if !sub.flag_values.is_empty() {
            out.push((path.clone(), sub));
        }
        subs_with_values(&path, &sub.sub, out);
    }
}

fn spellings(flag: &str, value: &str) -> Vec<Vec<String>> {
    let mut out = vec![vec![flag.to_string(), value.to_string()], vec![format!("{flag}={value}")]];
    if flag.len() == 2 && !flag.starts_with("--") {
        out.push(vec![format!("{flag}{value}")]);
    }
    out
}

#[test]
fn every_listed_flag_value_is_the_only_one_admitted() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("commands");
    let mut files = Vec::new();
    toml_files(&root, &mut files);
    let parsed: Vec<TomlFile> = files
        .iter()
        .map(|f| toml::from_str(&std::fs::read_to_string(f).expect("read toml")).expect("parse toml"))
        .collect();
    let mut found = Vec::new();
    for file in &parsed {
        for cmd in &file.command {
            subs_with_values(std::slice::from_ref(&cmd.name), &cmd.sub, &mut found);
        }
    }
    assert!(!found.is_empty(), "no sub declares flag_values — the walk is broken");
    let mut failures = Vec::new();
    for (path, sub) in &found {
        let arg: Vec<String> = sub
            .first_arg
            .first()
            .map(|p| p.strip_suffix('*').map_or_else(|| p.clone(), |x| format!("{x}zq")))
            .into_iter()
            .collect();
        for (flag, choices) in &sub.flag_values {
            let good = choices[0].trim_end_matches('*');
            for spelled in spellings(flag, good) {
                if !verdict(&line(path, &[&arg, &spelled])).is_allowed() {
                    failures.push(format!("  {} {} {}: a listed value is refused", path.join(" "), arg.join(" "), spelled.join(" ")));
                }
            }
            for spelled in spellings(flag, "go-template-file=/etc/zq") {
                for words in [line(path, &[&arg, &spelled]), line(path, &[&spelled, &arg])] {
                    if verdict(&words).is_allowed() {
                        failures.push(format!("  {}: an unlisted value is approved", words.join(" ")));
                    }
                }
            }
            if verdict(&line(path, &[&arg, std::slice::from_ref(flag)])).is_allowed() {
                failures.push(format!("  {} {flag}: a listed flag with no value is approved", path.join(" ")));
            }
        }
    }
    assert!(failures.is_empty(), "flag_values not enforced:\n{}", failures.join("\n"));
}

#[test]
fn kubectl_output_formats_that_read_a_file_are_refused() {
    for c in [
        "kubectl get -o go-template-file=$HOME/.ssh/id_rsa pods",
        "kubectl get -o=go-template-file=/etc/passwd pods",
        "kubectl get pods --output=jsonpath-file=/etc/passwd",
        "kubectl get pods -o go-template-file --template=/etc/passwd",
        "kubectl get pods -o go-template --template=/etc/passwd",
        "kubectl get pods -Ao templatefile=/etc/passwd",
        "kubectl get pods -ocustom-columns-file=/etc/passwd",
        "kubectl config view -o go-template-file=/etc/passwd",
        "kubectl events -o jsonpath-file=/etc/passwd",
    ] {
        assert!(!crate::is_safe_command(c), "must refuse: {c}");
    }
    for c in [
        "kubectl get pods -o json",
        "kubectl get pods -o=yaml",
        "kubectl get pods -A -o wide",
        "kubectl get pods -o 'jsonpath={.items[*].metadata.name}'",
        "kubectl get pods -o custom-columns=NAME:.metadata.name",
        "kubectl config view -o yaml",
        "kubectl events -o json",
    ] {
        assert!(crate::is_safe_command(c), "must approve: {c}");
    }
}
