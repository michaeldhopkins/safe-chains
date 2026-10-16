//! A path that goes through a symbolic link is judged where the link leads.
//!
//! A link committed in a project (`rc -> ~/.zshrc`, `h -> .git/hooks`) used to be judged by its own
//! name, inside the project, while every command that opened it reached the target. Each test here
//! builds real links in a scratch project and compares the verdict through the link with the verdict
//! for the target spelled directly.

use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use proptest::prelude::*;
use safe_chains::pathctx::PathCtx;
use safe_chains::{Verdict, command_verdict_in};

static CASE: AtomicUsize = AtomicUsize::new(0);

struct Project {
    _dir: tempfile::TempDir,
    root: String,
}

fn project() -> Project {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("scratch project");
    let root = dir.path().canonicalize().expect("canonical root").to_str().expect("utf-8 root").to_string();
    for sub in ["src", ".git/hooks", "links"] {
        std::fs::create_dir_all(format!("{root}/{sub}")).expect("project folders");
    }
    std::fs::write(format!("{root}/src/a.txt"), "a").expect("project file");
    Project { _dir: dir, root }
}

fn at(p: &Project, cmd: &str) -> Verdict {
    command_verdict_in(cmd, PathCtx { cwd: Some(p.root.clone()), root: Some(p.root.clone()), session_id: None })
}

fn relative_from(dir: &Path, target: &str) -> String {
    let up = dir.components().count().saturating_sub(1);
    format!("{}{}", "../".repeat(up), target.trim_start_matches('/'))
}

const TARGETS: [&str; 8] = [
    "@/src/a.txt", "@/src/new.txt", "@/.git/hooks/pre-commit", "@/.git/config", "/etc/hosts", "/etc/ssh/ssh_config", "/usr/bin/true",
    "/etc/p213-missing",
];

const COMMANDS: [&str; 7] = ["cat {}", "echo x > {}", "head -n 1 {}", "cp src/a.txt {}", "chmod 644 {}", "touch {}", "wc -l {}"];

fn direct_spelling(target: &str) -> String {
    match target.strip_prefix("@/") {
        Some(inside) => inside.to_string(),
        None => target.to_string(),
    }
}

fn absolute(p: &Project, target: &str) -> String {
    match target.strip_prefix("@/") {
        Some(inside) => format!("{}/{inside}", p.root),
        None => target.to_string(),
    }
}

/// Builds a chain of `hops` links ending at `target`, each hop spelled absolute or relative as
/// `relative` says, and returns the chain's first link relative to the project root.
fn chain(p: &Project, target: &str, hops: usize, relative: &[bool]) -> String {
    let case = CASE.fetch_add(1, Ordering::Relaxed);
    let dir = PathBuf::from(format!("{}/links/c{case}", p.root));
    std::fs::create_dir_all(&dir).expect("chain folder");
    let mut next = absolute(p, target);
    for hop in (0..hops).rev() {
        let link = dir.join(format!("l{hop}"));
        let spelled = if relative[hop % relative.len()] { relative_from(&dir, &next) } else { next.clone() };
        symlink(&spelled, &link).expect("link");
        next = link.to_str().expect("utf-8 link").to_string();
    }
    format!("links/c{case}/l0")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// For any chain of links, the verdict through the chain is the verdict for its last target.
    #[test]
    fn a_chain_of_links_is_judged_as_its_target(
        target in 0..TARGETS.len(),
        command in 0..COMMANDS.len(),
        hops in 1usize..5,
        relative in proptest::collection::vec(any::<bool>(), 1..5),
    ) {
        let p = project();
        let link = chain(&p, TARGETS[target], hops, &relative);
        let through = COMMANDS[command].replace("{}", &link);
        let direct = COMMANDS[command].replace("{}", &direct_spelling(TARGETS[target]));
        prop_assert_eq!(at(&p, &through), at(&p, &direct), "{} vs {}", through, direct);
    }
}

#[test]
fn a_write_through_a_link_out_of_the_project_is_refused() {
    let p = project();
    symlink("/etc/hosts", format!("{}/rc", p.root)).expect("link");
    symlink("/etc", format!("{}/sys", p.root)).expect("link");
    for cmd in ["echo x > rc", "cp src/a.txt rc", "touch sys/p213-new", "echo x > ./sys/../etc/hosts"] {
        assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved");
    }
}

#[test]
fn a_link_into_a_protected_folder_of_the_project_is_judged_as_that_folder() {
    let p = project();
    symlink(".git/hooks", format!("{}/h", p.root)).expect("link");
    for cmd in ["echo x > h/pre-commit", "chmod +x h/pre-commit", "cp src/a.txt h/post-checkout"] {
        assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved");
    }
}

#[test]
fn dotdot_after_a_link_climbs_from_the_link_target() {
    let p = project();
    symlink("/etc/ssh", format!("{}/sd", p.root)).expect("link");
    std::fs::write(format!("{}/hosts", p.root), "").expect("decoy");
    assert_eq!(at(&p, "echo x > sd/../hosts"), at(&p, "echo x > /etc/hosts"));
    assert!(!at(&p, "echo x > sd/../hosts").is_allowed());
}

#[test]
fn a_link_inside_the_project_to_the_project_still_approves() {
    let p = project();
    symlink("src/a.txt", format!("{}/n", p.root)).expect("link");
    symlink("src", format!("{}/s", p.root)).expect("link");
    for cmd in ["cat n", "echo x > n", "cat s/a.txt", "echo x > s/b.txt", "rm n", "mv n m"] {
        assert!(at(&p, cmd).is_allowed(), "{cmd} was refused");
    }
}

#[test]
fn removing_a_link_acts_on_the_link_but_a_trailing_slash_reaches_through() {
    let p = project();
    symlink("/etc/ssh", format!("{}/sd", p.root)).expect("link");
    assert!(at(&p, "rm sd").is_allowed(), "rm sd removes only the link");
    assert!(!at(&p, "rm -r sd/").is_allowed(), "rm -r sd/ removes the target's contents");
}

#[test]
fn a_cd_through_a_link_moves_to_the_target() {
    let p = project();
    symlink("/etc", format!("{}/sys", p.root)).expect("link");
    assert!(!at(&p, "cd sys && echo x > hosts").is_allowed());
    assert!(!at(&p, "cd -P sys/.. && echo x > etc/hosts").is_allowed());
    assert!(!at(&p, "cd sys && cd -P .. && echo x > etc/hosts").is_allowed());
    assert!(at(&p, "cd sys/.. && echo x > src/a.txt").is_allowed());
}

#[test]
fn a_link_loop_is_refused() {
    let p = project();
    symlink(format!("{}/b", p.root), format!("{}/a", p.root)).expect("link");
    symlink(format!("{}/a", p.root), format!("{}/b", p.root)).expect("link");
    assert!(!at(&p, "cat a").is_allowed());
    assert!(!at(&p, "echo x > a").is_allowed());
}

#[test]
fn a_glob_matching_a_link_out_of_the_project_is_judged_through_it() {
    let p = project();
    symlink("/etc/hosts", format!("{}/rc", p.root)).expect("link");
    assert!(!at(&p, "echo x > r[c]").is_allowed());
    assert!(!at(&p, "cp src/a.txt r?").is_allowed());
}

#[test]
fn a_dangling_link_out_of_the_project_is_judged_at_its_target() {
    let p = project();
    symlink("/etc/p213-missing", format!("{}/w", p.root)).expect("link");
    assert!(!at(&p, "echo x > w").is_allowed());
}

#[test]
fn a_missing_folder_then_dotdot_still_follows_the_links_after_it() {
    let p = project();
    symlink(format!("{}/.git/hooks", p.root), format!("{}/hk", p.root)).expect("link");
    symlink("/etc", format!("{}/sys", p.root)).expect("link");
    for cmd in ["echo x > nothere/../hk/pre-commit", "mkdir -p m && cd m && echo x > ../hk/pre-commit", "cat nothere/../sys/hosts"] {
        let direct = cmd
            .replace("nothere/../hk", ".git/hooks")
            .replace("../hk", "../.git/hooks")
            .replace("nothere/../sys", "/etc");
        assert_eq!(at(&p, cmd).is_allowed(), at(&p, &direct).is_allowed(), "{cmd} differs from {direct}");
    }
    assert!(!at(&p, "echo x > nothere/../hk/pre-commit").is_allowed());
}

#[test]
fn a_glob_over_plain_files_and_links_within_the_project_still_approves() {
    let p = project();
    std::fs::write(format!("{}/AGENTS.md", p.root), "a").expect("file");
    std::fs::write(format!("{}/README.md", p.root), "a").expect("file");
    symlink("AGENTS.md", format!("{}/CLAUDE.md", p.root)).expect("link");
    symlink("../README.md", format!("{}/src/README.md", p.root)).expect("link");
    for cmd in ["cat *.md", "grep foo *.md", "wc -l *", "touch *.md", "cat src/*"] {
        assert!(at(&p, cmd).is_allowed(), "{cmd} was refused");
    }
    symlink(format!("{}/.git/hooks", p.root), format!("{}/zhk", p.root)).expect("link");
    assert!(!at(&p, "touch *").is_allowed(), "a glob reaching .git/hooks through a link was approved");
}

/// Commands that act on a link named as an operand (copy it, archive it, change its own mode,
/// replace it) never open its target, so a link out of the project does not refuse them. The same
/// commands told to follow links, or reaching through a trailing slash, are judged at the target.
#[test]
fn a_command_acting_on_the_link_itself_is_judged_at_the_link() {
    let p = project();
    symlink("/etc/hosts", format!("{}/out", p.root)).expect("link to a file");
    symlink("/etc", format!("{}/outd", p.root)).expect("link to a folder");
    for cmd in [
        "cp -R out x", "cp -a out x", "cp -R outd y", "rsync -a out x", "rsync -a outd y", "tar -cf t.tar out", "tar -cf t.tar outd",
        "chmod -h 644 out", "chflags -h uchg out", "ln -sf src/a.txt out", "ln -sfn src/a.txt outd", "mv out x", "mv outd y",
    ] {
        assert!(at(&p, cmd).is_allowed(), "{cmd} was refused");
    }
    for cmd in [
        "cp -RL outd y", "cp -R -H outd y", "rsync -aL outd y", "rsync -a --copy-links outd y", "tar -chf t.tar outd",
        "tar --dereference -cf t.tar outd", "chmod 644 out", "chmod -h 644 outd/", "ln -sf src/a.txt outd", "cp -R src outd",
        "rsync -a src outd", "cp -tR outd",
    ] {
        assert!(!at(&p, cmd).is_allowed(), "{cmd} was approved");
    }
}

/// A link operand spelled so that the kernel goes through it (`link/`, `link/.`, `link/./.`) is
/// judged at the target even by a command that would otherwise act on the link itself.
#[test]
fn a_link_operand_spelled_through_it_is_judged_at_the_target() {
    let p = project();
    symlink("/etc", format!("{}/outd", p.root)).expect("link out");
    symlink(format!("{}/.git", p.root), format!("{}/gl", p.root)).expect("link to the metadata folder");
    for tail in ["/", "/.", "/./", "/./.", "//."] {
        for link in ["outd", "gl"] {
            let op = format!("{link}{tail}");
            for cmd in [format!("chmod -h 777 {op}"), format!("chflags -h uchg {op}"), format!("echo {op} | xargs chmod -h 777")] {
                assert!(!at(&p, &cmd).is_allowed(), "{cmd} was approved");
            }
        }
        let op = format!("outd{tail}");
        for cmd in [format!("cp -R {op} y"), format!("cp -a {op} y"), format!("rsync -a {op} y"), format!("tar -cf t.tar {op}")] {
            assert!(!at(&p, &cmd).is_allowed(), "{cmd} was approved");
        }
        assert!(at(&p, &format!("cp -R gl{tail} y")).is_allowed(), "copying out of the metadata folder was refused");
    }
    assert!(!at(&p, "tar -cf t.tar -C outd .").is_allowed());
}
