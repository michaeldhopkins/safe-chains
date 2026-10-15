//! `jj -R DIR` with `--ignore-working-copy` reads DIR when the subcommand only observes.
//!
//! Without the flag every jj command snapshots the working copy into DIR's store, so DIR is a
//! write target and a repository outside the workspace is refused. With it an observing
//! subcommand writes nothing, and the repository is judged as a read: approved in home space,
//! refused for a credential store. A mutating subcommand writes DIR's store with or without the
//! flag, so it stays refused outside the workspace.
//!
//! Every sub in `commands/vcs/jj.toml` must be classified here, so a sub added later is looked
//! at before it ships.

use std::collections::BTreeSet;

const JJ_TOML: &str = include_str!("../commands/vcs/jj.toml");

const GROUPS: &[&str] = &["bookmark", "config", "file", "git", "git remote", "op", "tag", "workspace"];

const OBSERVING: &[(&str, &str)] = &[
    ("cat", "cat f"),
    ("diff", "diff --stat"),
    ("help", "help"),
    ("log", "log -r @ --no-graph"),
    ("root", "root"),
    ("show", "show"),
    ("st", "st"),
    ("status", "status"),
    ("version", "version"),
    ("bookmark list", "bookmark list"),
    ("config get", "config get user.name"),
    ("config list", "config list"),
    ("file list", "file list"),
    ("file show", "file show f"),
    ("git remote list", "git remote list"),
    ("op log", "op log"),
    ("tag list", "tag list"),
    ("workspace list", "workspace list"),
    ("workspace root", "workspace root"),
];

const MUTATING: &[(&str, &str)] = &[
    ("abandon", "abandon"),
    ("absorb", "absorb"),
    ("backout", "backout -r @"),
    ("commit", "commit -m x"),
    ("describe", "describe -m x"),
    ("duplicate", "duplicate"),
    ("edit", "edit @"),
    ("new", "new"),
    ("parallelize", "parallelize"),
    ("rebase", "rebase -d @-"),
    ("resolve", "resolve --list"),
    ("restore", "restore"),
    ("simplify-parents", "simplify-parents"),
    ("split", "split -m x f"),
    ("squash", "squash -m x"),
    ("undo", "undo"),
    ("bookmark create", "bookmark create b"),
    ("bookmark delete", "bookmark delete b"),
    ("bookmark forget", "bookmark forget b"),
    ("bookmark move", "bookmark move b --to @"),
    ("bookmark rename", "bookmark rename a b"),
    ("bookmark set", "bookmark set b"),
    ("bookmark track", "bookmark track b@origin"),
    ("bookmark untrack", "bookmark untrack b@origin"),
    ("config path", "config path --repo"),
    ("git fetch", "git fetch"),
    ("git import", "git import"),
    ("git init", "git init"),
    ("workspace add", "workspace add w"),
    ("workspace forget", "workspace forget w"),
    ("workspace rename", "workspace rename w"),
    ("workspace update-stale", "workspace update-stale"),
];

const OUTSIDE: &[&str] = &["~/.claude", "~/x"];

fn subs(table: &toml::Value, prefix: &str, out: &mut BTreeSet<String>) {
    let Some(list) = table.get("sub").and_then(toml::Value::as_array) else { return };
    for sub in list {
        let name = sub.get("name").and_then(toml::Value::as_str).expect("every sub has a name");
        let full = if prefix.is_empty() { name.to_string() } else { format!("{prefix} {name}") };
        if sub.get("candidate").and_then(toml::Value::as_bool) == Some(true) {
            continue;
        }
        subs(sub, &full, out);
        out.insert(full);
    }
}

fn jj_subs() -> BTreeSet<String> {
    let doc: toml::Value = toml::from_str(JJ_TOML).expect("jj.toml parses");
    let command = doc
        .get("command")
        .and_then(toml::Value::as_array)
        .and_then(|c| c.iter().find(|c| c.get("name").and_then(toml::Value::as_str) == Some("jj")))
        .expect("jj.toml defines jj");
    let mut out = BTreeSet::new();
    subs(command, "", &mut out);
    out
}

fn approved(command: &str) -> bool {
    safe_chains::is_safe_command(command)
}

#[test]
fn every_jj_sub_is_classified_as_observing_or_mutating() {
    let classified: BTreeSet<String> = OBSERVING
        .iter()
        .chain(MUTATING)
        .map(|(sub, _)| (*sub).to_string())
        .chain(GROUPS.iter().map(|g| (*g).to_string()))
        .collect();
    let unclassified: Vec<String> = jj_subs().difference(&classified).cloned().collect();
    assert!(unclassified.is_empty(), "classify these jj subs in tests/jj_repository_reads.rs: {unclassified:?}");
    let stale: Vec<String> = classified.difference(&jj_subs()).cloned().collect();
    assert!(stale.is_empty(), "these rows name no jj sub: {stale:?}");
}

#[test]
fn an_observing_sub_reads_an_outside_repository_only_with_ignore_working_copy() {
    for (sub, args) in OBSERVING {
        assert!(approved(&format!("jj --ignore-working-copy -R ./sub {args}")), "{sub}: the local form must be approved");
        for dir in OUTSIDE {
            for (with, without) in [
                (format!("jj --ignore-working-copy -R {dir} {args}"), format!("jj -R {dir} {args}")),
                (format!("jj --repository={dir} {args} --ignore-working-copy"), format!("jj --repository={dir} {args}")),
                (format!("jj {args} -R {dir} --ignore-working-copy"), format!("jj {args} -R {dir}")),
            ] {
                assert!(approved(&with), "{sub}: {with} only reads {dir}");
                assert!(!approved(&without), "{sub}: {without} snapshots into {dir}");
            }
        }
    }
}

#[test]
fn a_mutating_sub_never_reaches_an_outside_repository() {
    for (sub, args) in MUTATING {
        assert!(approved(&format!("jj --ignore-working-copy -R ./sub {args}")), "{sub}: the local form must be approved");
        for dir in OUTSIDE {
            for command in [format!("jj --ignore-working-copy -R {dir} {args}"), format!("jj -R {dir} {args}")] {
                assert!(!approved(&command), "{sub}: {command} writes {dir}'s store");
            }
        }
    }
}

#[test]
fn a_read_never_reaches_a_credential_store_or_hides_the_sub() {
    for command in [
        "jj --ignore-working-copy -R ~/.ssh log",
        "jj --ignore-working-copy -R ~/.aws status",
        "jj --ignore-working-copy -GR/etc log",
        "jj --ignore-working-copy --bogus -R ~/x log",
        "jj -R ~/x log -- --ignore-working-copy",
        "jj -R ~/x --color log new --ignore-working-copy",
    ] {
        assert!(!approved(command), "{command}");
    }
}

fn approved_in_project(command: &str) -> bool {
    use safe_chains::pathctx::{PathCtx, enter};
    let home = std::env::var("HOME").expect("HOME");
    let project = format!("{home}/projects/jjprobe");
    let _ctx = enter(PathCtx { cwd: Some(project.clone()), root: Some(project), session_id: None });
    approved(command)
}

#[test]
fn a_cd_into_an_outside_repository_is_judged_like_its_repository_flag() {
    let approved = approved_in_project;
    for (sub, args) in OBSERVING {
        assert!(approved(&format!("cd ./sub && jj {args}")), "{sub}: a cd inside the workspace stays approved");
        for dir in OUTSIDE {
            let positional_gated = sub.starts_with("git ") || sub.starts_with("workspace ");
            assert!(positional_gated || approved(&format!("cd {dir} && jj --ignore-working-copy {args}")), "{sub}: only reads {dir}");
            assert!(!approved(&format!("cd {dir} && jj {args}")), "{sub}: snapshots into {dir}");
        }
    }
    for (sub, args) in MUTATING {
        assert!(approved(&format!("cd ./sub && jj --ignore-working-copy {args}")), "{sub}: a cd inside the workspace stays approved");
        for dir in OUTSIDE {
            for command in [format!("cd {dir} && jj --ignore-working-copy {args}"), format!("cd {dir} && jj {args}")] {
                assert!(!approved(&command), "{sub}: {command} writes {dir}'s store");
            }
        }
    }
}

#[test]
fn global_flags_after_a_group_word_are_judged_like_leading_ones() {
    let after_group = |sub: &str, args: &str, flags: &str| {
        let depth = sub.split(' ').count() - 1;
        let mut words: Vec<&str> = args.split(' ').collect();
        let tail = words.split_off(depth.min(1));
        format!("jj {} {flags} {}", words.join(" "), tail.join(" "))
    };
    let mut checked = 0;
    for (sub, args) in OBSERVING.iter().filter(|(sub, _)| sub.contains(' ')) {
        checked += 1;
        assert!(approved(&after_group(sub, args, "-R ./sub --quiet")), "{sub}: local, after the group");
        for dir in OUTSIDE {
            let with = after_group(sub, args, &format!("-R {dir} --ignore-working-copy"));
            let without = after_group(sub, args, &format!("-R {dir}"));
            assert!(approved(&with), "{with} only reads {dir}");
            assert!(!approved(&without), "{without} snapshots into {dir}");
        }
    }
    for (sub, args) in MUTATING.iter().filter(|(sub, _)| sub.contains(' ')) {
        checked += 1;
        for dir in OUTSIDE {
            let command = after_group(sub, args, &format!("-R {dir} --ignore-working-copy"));
            assert!(!approved(&command), "{command} writes {dir}'s store");
        }
    }
    assert!(checked > 20, "only {checked} grouped subs checked");
    assert!(approved("jj --ignore-working-copy git remote -R ~/x list"));
    assert!(!approved("jj git remote -R ~/x list"));
}
