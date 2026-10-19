use std::path::{Path, PathBuf};
use std::process::Command;

use crate::command_verdict_in;
use crate::pathctx::PathCtx;
use proptest::prelude::*;

fn spelled_root(canonical: &str) -> String {
    match canonical.strip_prefix("/private") {
        Some(rest) if rest.starts_with("/var/") || rest.starts_with("/tmp/") || rest.starts_with("/etc/") => rest.to_string(),
        _ => canonical.to_string(),
    }
}

fn project() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().expect("scratch");
    let canonical = dir.path().canonicalize().expect("canonical").to_str().expect("utf-8").to_string();
    (dir, spelled_root(&canonical))
}

fn ctx(root: &str) -> PathCtx {
    PathCtx { cwd: Some(root.to_string()), root: Some(root.to_string()), session_id: None }
}

fn allows(root: &str, cmd: &str) -> bool {
    command_verdict_in(cmd, ctx(root)).is_allowed()
}

fn dir(path: impl AsRef<Path>) {
    std::fs::create_dir_all(path).expect("mkdir");
}

fn file(path: impl AsRef<Path>) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        dir(parent);
    }
    std::fs::write(path, b"x").expect("write");
}

fn link(target: impl AsRef<Path>, path: impl AsRef<Path>) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        dir(parent);
    }
    std::os::unix::fs::symlink(target, path).expect("symlink");
}

fn expect(root: &str, cases: &[(&str, bool)]) {
    for (cmd, want) in cases {
        let got = allows(root, cmd);
        assert_eq!(got, *want, "{cmd}: approved={got}, want approved={want}");
    }
}

#[test]
fn a_parent_glob_is_judged_at_every_match() {
    let (_dir, root) = project();
    file(format!("{root}/foo[bar]/sub/plain"));
    file(format!("{root}/foob/sub/.git/config"));
    file(format!("{root}/a/repo/.git/config"));
    file(format!("{root}/b/repo/readme"));
    file(format!("{root}/a/sub/.envrc"));
    file(format!("{root}/b/sub/plain"));
    let abs = &root;
    expect(
        &root,
        &[
            (&format!("rm -rf {abs}/foo[bar]/sub"), false),
            (&format!("rm -rf '{abs}/foo[bar]/sub'"), false),
            ("rm -rf foo[bar]/sub", false),
            ("rm -rf 'foo[bar]/sub'", false),
            (&format!("chmod -R 755 {abs}/foo[bar]/sub"), false),
            (&format!("find {abs}/foo[bar]/sub -delete"), false),
            ("find foo[bar]/sub -delete", false),
            (&format!("rm -rf {abs}/*/repo"), false),
            (&format!("rm -rf {abs}/*/sub"), false),
            (&format!("chmod -R 755 {abs}/*/repo"), false),
            (&format!("find {abs}/*/sub -delete"), false),
            (&format!("cp -R {abs}/foo[bar]/sub {abs}/dest"), false),
        ],
    );
}

#[test]
fn an_unquoted_glob_whose_literal_is_a_symlink_is_the_match() {
    let (_dir, root) = project();
    file(format!("{root}/safe/hooks/pre-commit"));
    link("safe", format!("{root}/foo[bar]"));
    file(format!("{root}/repo/.git/hooks/pre-commit"));
    link("repo/.git", format!("{root}/foob"));
    let abs = &root;
    expect(
        &root,
        &[
            (&format!("chmod 755 {abs}/foo[bar]"), false),
            (&format!("chmod 755 {abs}/foo[bar]/hooks/pre-commit"), false),
            (&format!("rm {abs}/foo[bar]/hooks/pre-commit"), false),
            ("chmod 755 foo[bar]", false),
            ("rm foo[bar]/hooks/pre-commit", false),
            (&format!("chmod -R 755 {abs}/foo[bar]"), true),
            (&format!("rm -rf {abs}/foo[bar]"), true),
            ("rm -rf foo[bar]", true),
            (&format!("chmod 755 '{abs}/foo[bar]'"), false),
            ("chmod 755 'foo[bar]'", false),
        ],
    );
}

#[test]
fn a_recursive_chmod_does_not_follow_the_operand_symlink() {
    let (_dir, root) = project();
    file(format!("{root}/repo/.git/hooks/pre-commit"));
    link("repo/.git", format!("{root}/link"));
    link("repo/.git", format!("{root}/only[ok]"));
    expect(
        &root,
        &[
            ("chmod -R 755 link", true),
            ("chmod --recursive 755 link", true),
            ("chmod -R 755 'only[ok]'", true),
            ("chmod -R 755 only[ok]", true),
            ("chmod 755 link", false),
            ("chmod 755 'only[ok]'", false),
            ("chmod -R 755 link/", false),
            ("chmod -R 755 'only[ok]/'", false),
            ("chmod -RL 755 link", false),
        ],
    );
}

#[test]
fn a_quoted_bracket_with_another_match_is_unresolved() {
    let (_dir, root) = project();
    file(format!("{root}/foo[bar]/plain"));
    file(format!("{root}/foob/.git/config"));
    expect(
        &root,
        &[
            ("rm -rf 'foo[bar]'", false),
            ("chmod -R 755 'foo[bar]'", false),
            ("rm -rf foo[bar]", false),
            ("find 'foo[bar]' -delete", false),
            ("find foo[bar] -delete", false),
            ("chmod 755 'foo[bar]'", false),
        ],
    );
}

#[test]
fn an_unquoted_glob_is_judged_as_the_match_not_the_literal_symlink() {
    let (_dir, root) = project();
    file(format!("{root}/repo/.git/config"));
    link("repo/.git", format!("{root}/foo[bar]"));
    file(format!("{root}/foob"));
    let abs = &root;
    expect(
        &root,
        &[
            ("chmod 755 foo[bar]", true),
            (&format!("chmod 755 {abs}/foo[bar]"), true),
            ("chmod 755 'foo[bar]'", false),
            (&format!("chmod 755 '{abs}/foo[bar]'"), false),
            ("rm foo[bar]", true),
            ("rm 'foo[bar]'", false),
        ],
    );
}

#[test]
fn a_bracket_literal_is_approved_only_when_nothing_else_matches() {
    let (_dir, root) = project();
    file(format!("{root}/plain[ok]/kept"));
    file(format!("{root}/build/a.o"));
    file(format!("{root}/other.o"));
    file(format!("{root}/notes.txt"));
    dir(format!("{root}/build/obj"));
    expect(
        &root,
        &[
            ("find 'plain[ok]' -delete", true),
            ("find plain[ok] -delete", true),
            ("rm -rf 'plain[ok]'", true),
            ("rm -rf plain[ok]", true),
            ("find build -name '*.o' -delete", true),
            ("rm notes.txt", true),
            ("chmod 644 notes.txt", true),
            ("rm -rf build", true),
            ("chmod -R 755 build", true),
            ("mv build dest", true),
            ("rm -rf .git", false),
            ("chmod 755 .git", false),
        ],
    );
    file(format!("{root}/plaino"));
    expect(
        &root,
        &[("find 'plain[ok]' -delete", false), ("rm -rf 'plain[ok]'", false), ("find plain[ok] -delete", true), ("rm plain[ok]", true)],
    );
}

#[test]
fn an_absolute_bracket_whose_match_holds_git_is_refused() {
    let (_dir, root) = project();
    file(format!("{root}/plain[ok]/kept"));
    file(format!("{root}/plaino/.git/config"));
    let abs = &root;
    expect(&root, &[(&format!("rm -rf {abs}/plain[ok]"), false), (&format!("rm -rf '{abs}/plain[ok]'"), false)]);
}

#[test]
fn a_nested_symlink_is_not_a_reason_to_refuse_removal() {
    let (_dir, root) = project();
    file(format!("{root}/src/keep"));
    file(format!("{root}/other/.git/config"));
    link("../other", format!("{root}/src/link"));
    dir(format!("{root}/repo/.git"));
    file(format!("{root}/repo/readme"));
    expect(&root, &[("rm -rf src", true), ("mv repo dest", true), ("rm -rf repo", false)]);
}

const GLOB_KINDS: &[(&str, &str)] = &[("foo[bar]", "foob"), ("[abc]", "b"), ("pre[xyz]", "prex")];

fn bash_expansions(cwd: &Path, operand: &str) -> Vec<String> {
    let script = format!("printf '%s\\0' {operand}");
    let out = Command::new("bash")
        .args(["--noprofile", "--norc", "-c", &script])
        .current_dir(cwd)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("bash");
    assert!(out.status.success(), "bash: {}", String::from_utf8_lossy(&out.stderr));
    out.stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8(s.to_vec()).expect("utf-8"))
        .collect()
}

fn rm_deletes_git(cwd: &Path, expanded: &str) -> bool {
    let path = if expanded.starts_with('/') { PathBuf::from(expanded) } else { cwd.join(expanded) };
    let Ok(meta) = std::fs::symlink_metadata(&path) else { return false };
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return false;
    }
    contains_git(&path)
}

fn contains_git(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else { return false };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else { continue };
        if meta.file_type().is_symlink() {
            continue;
        }
        if entry.file_name() == ".git" && meta.is_dir() {
            return true;
        }
        if meta.is_dir() && contains_git(&path) {
            return true;
        }
    }
    false
}

struct Comp {
    literal: String,
    matched: String,
}

fn plant(root: &Path, comps: &[Comp], symlink_at: Option<usize>) {
    write_side(root, comps, true, symlink_at);
    write_side(root, comps, false, symlink_at);
}

fn write_side(root: &Path, comps: &[Comp], literal_side: bool, symlink_at: Option<usize>) {
    let mut at = root.to_path_buf();
    for (i, comp) in comps.iter().enumerate() {
        let name = if literal_side { &comp.literal } else { &comp.matched };
        let next = at.join(name);
        let last = i + 1 == comps.len();
        if symlink_at == Some(i) {
            let real_name = format!(".target-{i}-{}", if literal_side { "lit" } else { "mat" });
            let real = at.join(real_name);
            if next.symlink_metadata().is_err() {
                dir(&real);
                link(&real, &next);
            }
            at = if next.symlink_metadata().is_ok_and(|m| m.file_type().is_symlink()) {
                std::fs::read_link(&next).ok().map(|t| if t.is_absolute() { t } else { at.join(t) }).unwrap_or(real)
            } else {
                next
            };
            if last {
                finish(&at, literal_side);
            }
            continue;
        }
        if last {
            finish(&next, literal_side);
        } else if next.symlink_metadata().is_err() {
            dir(&next);
        }
        at = next;
    }
}

fn finish(dir_path: &Path, literal_side: bool) {
    dir(dir_path);
    if literal_side {
        file(dir_path.join("plain"));
    } else {
        file(dir_path.join(".git/config"));
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]
    #[test]
    fn an_expansion_that_reaches_git_is_not_approved(
        count in 1usize..4,
        glob_at in 0usize..3,
        kind in 0usize..3,
        absolute in proptest::bool::ANY,
        quoted in proptest::bool::ANY,
        link_at in 0usize..5,
    ) {
        let n = count;
        let glob_at = glob_at % n;
        let (pattern, sibling) = GLOB_KINDS[kind % GLOB_KINDS.len()];
        let symlink_at = (link_at < n).then_some(link_at % n);
        let comps: Vec<Comp> = (0..n)
            .map(|i| {
                if i == glob_at {
                    Comp { literal: pattern.to_string(), matched: sibling.to_string() }
                } else {
                    Comp { literal: format!("p{i}"), matched: format!("p{i}") }
                }
            })
            .collect();
        let (scratch, root) = project();
        let cwd = PathBuf::from(&root);
        plant(Path::new(&root), &comps, symlink_at);
        let relative = comps.iter().map(|c| c.literal.as_str()).collect::<Vec<_>>().join("/");
        let spelling = if absolute { format!("{root}/{relative}") } else { relative };
        let operand = if quoted { format!("'{spelling}'") } else { spelling.clone() };
        let _keep = scratch;
        let expansions = bash_expansions(&cwd, &operand);
        let unquoted = bash_expansions(&cwd, &spelling);
        let reaches = expansions.iter().chain(&unquoted).any(|word| rm_deletes_git(&cwd, word));
        if reaches {
            let cmd = format!("rm -rf {operand}");
            prop_assert!(!allows(&root, &cmd), "{cmd} approved; bash produced {expansions:?}; unquoted {unquoted:?}");
        }
    }
}
