//! The short-flag half of the arity guards: a short flag declared `standalone` where its own command
//! declares it `valued`, and a short spelling of a gated long flag that the gate does not name.
//!
//! Its own file because `tests.rs` is pinned by the file-length gate, and because both guards read
//! the registry the same way and share their helpers.

use super::types::{TomlCommand, TomlFile, TomlSub};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

struct Scope<'a> {
    label: String,
    standalone: &'a [String],
    valued: &'a [String],
    optional: &'a [String],
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

fn registry() -> Vec<TomlFile> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("commands");
    let mut files = Vec::new();
    toml_files(&root, &mut files);
    assert!(!files.is_empty(), "no command TOMLs found under {}", root.display());
    files
        .iter()
        .map(|f| toml::from_str(&std::fs::read_to_string(f).expect("read toml")).expect("parse toml"))
        .collect()
}

fn worklist(name: &str, columns: usize) -> HashSet<Vec<String>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {name}: {e}"));
    let rows: HashSet<Vec<String>> = raw
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| l.split('\t').map(|c| c.trim().to_string()).collect::<Vec<_>>())
        .inspect(|row| assert_eq!(row.len(), columns, "{name}: row {row:?} must have {columns} tab-separated columns"))
        .collect();
    assert!(!rows.is_empty(), "{name} has no rows — the guard would be vacuous");
    rows
}

fn is_short(f: &str) -> bool {
    f.len() == 2 && f.starts_with('-') && f != "--"
}

fn sub_scopes<'a>(prefix: &str, list: &'a [TomlSub], out: &mut Vec<Scope<'a>>) {
    for sub in list {
        let label = format!("{prefix} {}", sub.name);
        out.push(Scope { label: label.clone(), standalone: &sub.standalone, valued: &sub.valued, optional: &sub.optional_valued });
        sub_scopes(&label, &sub.sub, out);
    }
}

fn scopes(cmd: &TomlCommand) -> Vec<Scope<'_>> {
    let mut out = vec![Scope { label: cmd.name.clone(), standalone: &cmd.standalone, valued: &cmd.valued, optional: &cmd.optional_valued }];
    if let Some(fb) = &cmd.fallback {
        out.push(Scope {
            label: format!("{} [fallback]", cmd.name),
            standalone: &fb.standalone,
            valued: &fb.valued,
            optional: &fb.optional_valued,
        });
    }
    for (key, hp) in &cmd.handler_policy {
        out.push(Scope {
            label: format!("{} [policy {key}]", cmd.name),
            standalone: &hp.standalone,
            valued: &hp.valued,
            optional: &hp.optional_valued,
        });
    }
    if let Some(b) = &cmd.behavior {
        out.push(Scope { label: format!("{} [behavior]", cmd.name), standalone: &b.standalone, valued: &b.valued, optional: &[] });
    }
    sub_scopes(&cmd.name, &cmd.sub, &mut out);
    out
}

fn compare(name: &str, candidates: &BTreeMap<Vec<String>, String>, listed: &HashSet<Vec<String>>, unlisted_why: &str) {
    let mut unlisted: Vec<String> = candidates
        .iter()
        .filter(|(k, _)| !listed.contains(*k))
        .map(|(k, ctx)| format!("  {}{ctx}", k.join("\t")))
        .collect();
    let mut stale: Vec<String> = listed
        .iter()
        .filter(|k| !candidates.contains_key(*k))
        .map(|k| format!("  {}", k.join("\t")))
        .collect();
    unlisted.sort();
    stale.sort();
    assert!(unlisted.is_empty(), "{unlisted_why} ({}), and not on {name}:\n{}", unlisted.len(), unlisted.join("\n"));
    assert!(
        stale.is_empty(),
        "these {name} rows are no longer candidates ({}) — the declaration changed, so delete the row in \
             the same change and keep the file from rotting into a list of things that used to be true:\n{}",
        stale.len(),
        stale.join("\n"),
    );
}

/// A SHORT flag declared `standalone` in one scope of a command while declared `valued` in another
/// scope of the SAME command.
///
/// `a_long_flag_is_not_standalone_where_it_is_valued_elsewhere` compares a flag across tools, which
/// works because a `--name` means roughly the same thing everywhere. A short flag does not: `-o` is
/// an output path in one tool and a boolean in the next. But within ONE tool a letter usually keeps
/// its meaning from subcommand to subcommand, so the tool's own entry is the comparison that carries
/// information. pre-commit's `-c` was valued on `run` and standalone on `install`, where it names
/// the config whose hooks `install` writes; that is the shape this hunts.
///
/// The scopes compared are the command itself, every nested sub, the fallback grammar, each
/// `handler_policy` block and the `behavior` grammar. A flag in both lists of one scope, or in
/// `optional_valued`, takes a value optionally and counts as neither.
///
/// It is evidence, not proof: suites of separate programs (git, cargo, docker) reuse letters freely.
/// So the fixture is a worklist, and a row means "check this one scope's grammar".
#[test]
fn a_short_flag_is_not_standalone_where_its_own_command_declares_it_valued() {
    let mut candidates: BTreeMap<Vec<String>, BTreeSet<String>> = BTreeMap::new();
    for file in registry() {
        for cmd in &file.command {
            let scopes = scopes(cmd);
            for s in &scopes {
                for f in s.standalone.iter().filter(|f| is_short(f) && !s.valued.contains(f) && !s.optional.contains(f)) {
                    let elsewhere = scopes
                        .iter()
                        .filter(|o| o.label != s.label && o.valued.contains(f) && !o.standalone.contains(f) && !o.optional.contains(f))
                        .map(|o| o.label.clone());
                    let mut elsewhere = elsewhere.peekable();
                    if elsewhere.peek().is_some() {
                        candidates.entry(vec![s.label.clone(), f.clone()]).or_default().extend(elsewhere);
                    }
                }
            }
        }
    }
    let candidates: BTreeMap<Vec<String>, String> = candidates
        .into_iter()
        .map(|(k, v)| (k, format!("\t(valued in {})", v.into_iter().collect::<Vec<_>>().join(", "))))
        .collect();
    compare(
        "short_flag_arity_worklist.tsv",
        &candidates,
        &worklist("short_flag_arity_worklist.tsv", 2),
        "these scopes declare a short flag `standalone` that another scope of the same command declares \
         `valued`. Check the scope's grammar: move the flag to `valued` (or `optional_valued`), or add \
         the row because the declaration is right",
    );
}

/// A short flag that may be the other spelling of a GATED long flag, admitted while the gate names
/// only the long one.
///
/// Path gates are keyed by flag name, and the schema has no way to say that `-d` and
/// `--destination` are one flag. So a gate on `--destination` leaves `-d` ungated whatever its
/// arity, and when `-d` is also declared `standalone` its value lands as a positional besides.
/// `hugo build -d /etc/out` auto-approved while `hugo build --destination /etc/out` was refused,
/// and `yard doc -o` stood beside a gated `--output-dir` the same way.
///
/// `a_gated_flag_is_never_declared_value_less` cannot see this: it checks the gated name, and the
/// short spelling is not one. The pairing is guessed from the first letter after `--`, the usual
/// convention, which makes this a worklist rather than a deny-list: a row means "check whether
/// this letter is that flag". The fix is to gate the short spelling too; a row whose letter is a
/// different option stays listed.
#[test]
fn a_short_spelling_of_a_gated_long_flag_is_gated_too() {
    let mut central: HashMap<String, Vec<String>> = HashMap::new();
    for (key, flag, role) in crate::pathgate::central_flag_gates() {
        if role == crate::pathgate::Role::Ignore {
            continue;
        }
        let cmd = key.split(' ').next().unwrap_or_default().to_string();
        central.entry(cmd).or_default().push(flag);
    }

    let mut candidates: BTreeMap<Vec<String>, String> = BTreeMap::new();
    for file in registry() {
        for cmd in &file.command {
            let mut gated: BTreeSet<String> = BTreeSet::new();
            if let Some(gate) = &cmd.path_gate {
                gated.extend(gate.flag_roles().filter(|(_, r)| *r != crate::pathgate::Role::Ignore).map(|(f, _)| f.to_string()));
            }
            if let Some(flags) = central.get(&cmd.name) {
                gated.extend(flags.iter().cloned());
            }
            let shorts: BTreeSet<&String> = scopes(cmd)
                .iter()
                .flat_map(|s| s.standalone.iter().chain(s.valued).chain(s.optional))
                .filter(|f| is_short(f))
                .collect();
            for long in gated.iter().filter(|g| g.starts_with("--")) {
                let Some(letter) = long.chars().nth(2) else { continue };
                let short = format!("-{letter}");
                if shorts.contains(&short) && !gated.contains(&short) {
                    candidates.insert(vec![cmd.name.clone(), short, long.clone()], String::new());
                }
            }
        }
    }
    compare(
        "gated_short_spelling_worklist.tsv",
        &candidates,
        &worklist("gated_short_spelling_worklist.tsv", 3),
        "these commands gate a long flag and admit an ungated short flag of the same letter. If the \
         short flag is the same option, gate it with the same role and declare it `valued`; otherwise \
         add the row",
    );
}

#[test]
fn the_researched_value_taking_flags_are_declared_valued() {
    let wanted: &[(&str, &[&str])] = &[
        ("docker ps", &["-n", "--last"]),
        ("docker container ls", &["-n", "--last"]),
        ("docker container list", &["-n", "--last"]),
        ("podman ps", &["-n", "--last"]),
        ("podman container ls", &["-n", "--last"]),
        ("podman container list", &["-n", "--last"]),
        ("nerdctl ps", &["-n", "--last"]),
        ("git switch", &["-c", "--create"]),
        ("cargo tree", &["-e", "--edges", "-i", "--invert"]),
        ("hg status", &["-T", "--template", "--rev", "--terse"]),
        ("hg tip", &["-T", "--template", "--style"]),
        ("pip list", &["-i", "--index-url"]),
        ("dotnet nuget verify", &["-v", "--verbosity"]),
    ];
    let files = registry();
    let all: Vec<Scope<'_>> = files.iter().flat_map(|f| f.command.iter().flat_map(scopes)).collect();
    for (label, flags) in wanted {
        let found: Vec<&Scope<'_>> = all.iter().filter(|s| s.label == *label).collect();
        assert!(!found.is_empty(), "no scope named {label}");
        for s in found {
            for f in *flags {
                let f = (*f).to_string();
                assert!(s.valued.contains(&f) && !s.standalone.contains(&f), "{label} must declare {f} valued and only valued");
            }
        }
    }
}

#[test]
fn a_value_that_looks_like_a_flag_is_read_as_the_value_of_a_valued_short_flag() {
    for cmd in ["docker ps -n -1", "docker container ls -n -1", "podman ps --last -1", "nerdctl ps -n -1"] {
        assert!(crate::is_safe_command(cmd), "should be allowed: {cmd}");
    }
}

#[test]
fn an_unknown_word_anywhere_in_hg_may_be_an_early_global_option() {
    let files = registry();
    let hg = files.iter().flat_map(|f| &f.command).find(|c| c.name == "hg").expect("hg is declared");
    let mut checked = 0;
    for s in scopes(hg) {
        let mut refused = vec![
            format!("{} -R /tmp/other", s.label),
            format!("{} -vR/tmp/other", s.label),
            format!("{} --repo=/tmp/other", s.label),
            format!("{} --cwd /tmp/other", s.label),
            format!("{} --config=extensions.e=/tmp/e.py", s.label),
            format!("{} \"-$X\"", s.label),
            format!("{} -*", s.label),
            format!("{} -?/x", s.label),
        ];
        refused.extend(s.valued.iter().map(|f| format!("{} {f} \"$X\"", s.label)));
        refused.extend(s.valued.iter().map(|f| format!("{} {f} \"$(cat f)\"", s.label)));
        refused.extend(s.valued.iter().map(|f| format!("{} {f} \"-$X\"", s.label)));
        for cmd in refused {
            assert!(!crate::is_safe_command(&cmd), "{cmd} may run code through an hg early option");
            checked += 1;
        }
    }
    assert!(checked > 100, "only {checked} hg invocations checked");
    for cmd in ["hg status -T json", "hg tip --style compact", "hg log -r tip", "hg status --rev=tip"] {
        assert!(crate::is_safe_command(cmd), "should be allowed: {cmd}");
    }
}

#[test]
fn hg_takes_its_global_options_after_any_sub_and_refuses_what_runs_or_reaches_out() {
    for cmd in [
        "hg status -v", "hg diff -q", "hg summary -q", "hg help -v log", "hg status --color=always", "hg log --pager=never",
        "hg status -S", "hg log -l5", "hg log -I '*.py'", "hg log -m", "hg grep -u foo", "hg id -T json", "hg config -u", "hg status -r",
    ] {
        assert!(crate::is_safe_command(cmd), "should be allowed: {cmd}");
    }
    for cmd in [
        "hg incoming --ssh 'touch /tmp/x' ssh://h/r", "hg outgoing --remotecmd x", "hg incoming", "hg outgoing", "hg config --edit",
        "hg config -e", "hg summary --remote",
    ] {
        assert!(!crate::is_safe_command(cmd), "should be refused: {cmd}");
    }
}

fn toks(words: &[&str]) -> Vec<crate::parse::Token> {
    words.iter().map(|s| crate::parse::Token::from_test(s)).collect()
}

fn collect_output_path_subs(
    prefix: &mut Vec<String>,
    kind: &'static super::types::DispatchKind,
    out: &mut Vec<(Vec<String>, &'static [String])>,
) {
    use super::types::DispatchKind as D;
    let (D::Branching { subs, .. } | D::Custom { subs, .. }) = kind else {
        return;
    };
    for sub in subs {
        prefix.push(sub.name.clone());
        if !sub.output_path_flags.is_empty() {
            out.push((prefix.clone(), &sub.output_path_flags));
        }
        collect_output_path_subs(prefix, &sub.kind, out);
        prefix.pop();
    }
}

#[test]
fn output_flag_value_extracts_every_spelling_and_rejects_non_short_glued_forms() {
    let mut subs = Vec::new();
    for (name, spec) in super::TOML_REGISTRY.iter() {
        if name == &spec.name {
            collect_output_path_subs(&mut vec![spec.name.clone()], &spec.kind, &mut subs);
        }
    }
    assert!(!subs.is_empty(), "expected at least one sub with output_path_flags");

    for (path, flags) in subs {
        let base: Vec<&str> = path.iter().map(String::as_str).collect();
        assert_eq!(super::sub_output_path_token(&toks(&base)), None);
        for flag in flags {
            let mut spaced = base.clone();
            spaced.extend([flag.as_str(), "/etc/passwd"]);
            assert_eq!(super::sub_output_path_token(&toks(&spaced)), Some("/etc/passwd"));

            let eq = format!("{flag}=/etc/passwd");
            let mut eq_tokens = base.clone();
            eq_tokens.push(&eq);
            assert_eq!(super::sub_output_path_token(&toks(&eq_tokens)), Some("/etc/passwd"));

            let mut bare = base.clone();
            bare.push(flag.as_str());
            assert_eq!(super::sub_output_path_token(&toks(&bare)), None);

            let fused = format!("{flag}/etc/passwd");
            let mut fused_tokens = base.clone();
            fused_tokens.push(&fused);
            let expected = is_short(flag).then_some("/etc/passwd");
            assert_eq!(super::sub_output_path_token(&toks(&fused_tokens)), expected);
        }
    }

    assert_eq!(super::output_flag_value(&toks(&["-f/etc/passwd"]), "-f"), Some("/etc/passwd"));
    assert_eq!(super::output_flag_value(&toks(&["-f", "out.sql"]), "-f"), Some("out.sql"));
    assert_eq!(super::output_flag_value(&toks(&["--file=out.sql"]), "--file"), Some("out.sql"));
    assert_eq!(super::output_flag_value(&toks(&["-f"]), "-f"), None);
    assert_eq!(super::output_flag_value(&toks(&["--file"]), "--"), None);
    assert_eq!(super::output_flag_value(&toks(&["-file/etc/passwd"]), "-file"), None);
    assert_eq!(super::output_flag_value(&toks(&["db/etc/passwd"]), "db"), None);
}

#[test]
fn flag_present_matches_bare_glued_and_short_clusters_only() {
    assert!(super::flag_present(&toks(&["--force"]), "--force"));
    assert!(super::flag_present(&toks(&["--force=true"]), "--force"));
    assert!(super::flag_present(&toks(&["-d"]), "-d"));
    assert!(super::flag_present(&toks(&["-d=1"]), "-d"));
    assert!(super::flag_present(&toks(&["-vd"]), "-d"));
    assert!(super::flag_present(&toks(&["-da=val"]), "-d"));
    assert!(!super::flag_present(&toks(&["-o=d"]), "-d"));
    assert!(!super::flag_present(&toks(&["--other"]), "--force"));
    assert!(!super::flag_present(&toks(&["-a-b"]), "--force"));
    assert!(!super::flag_present(&toks(&["-fa"]), "-file"));
    assert!(!super::flag_present(&toks(&["-ba"]), "ab"));
}
