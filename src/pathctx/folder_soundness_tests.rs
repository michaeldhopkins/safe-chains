//! Anchor soundness (docs/design/unknown-folder-writes.md §8): whatever a folder level approves
//! with the folder unknown, running the same command in any folder that level claims to cover
//! must not write into a place the region table protects.

use proptest::prelude::*;

use super::*;
use crate::engine::facet::LocalLocus;
use crate::engine::resolve::regions::classify_region;
use crate::pathctx::PathCtx;
use crate::verdict::{SafetyLevel, Verdict};

/// A project under the real home, deep enough that its siblings are peer projects.
fn root() -> String {
    format!("{}/sc-anchor-probe/projects/app", std::env::var("HOME").unwrap_or_else(|_| "/nonexistent-home".to_string()))
}

/// The folders each level claims to cover (§3, §6): the project root, a subdirectory, a sibling
/// project and scratch. `workspace` treats the folder as the workspace, so the same list.
fn covered() -> [String; 4] {
    let root = root();
    let parent = root.trim_end_matches("/app").to_string();
    [root.clone(), format!("{root}/web"), format!("{parent}/lib"), "/tmp/x".to_string()]
}

const SPELLINGS: &[&str] = &[
    "echo x > {t}", "echo x >> {t}", "tee {t}", "tee -a {t}", "cp a.txt {t}", "mv a.txt {t}", "touch {t}", "mkdir -p {t}",
    "sed -i s/a/b/ {t}", "ln -s a.txt {t}", "rm -rf {t}", "sort -o {t} a.txt", "install -m 644 a.txt {t}", "cd {d} && echo x > {t}",
];

fn approved(level: FolderLevel, command: &str) -> bool {
    let _ctx =
        crate::pathctx::enter(PathCtx { cwd: Some(crate::targets::UNKNOWN_WORKDIR.to_string()), root: Some(root()), session_id: None });
    let _folder = enter(level);
    crate::cst::command_verdict(command).is_allowed()
}

/// Where `target` lands when the command runs in `anchor`, and the worst face a write or a rebind
/// of it reaches.
fn landing(anchor: &str, target: &str) -> LocalLocus {
    let _ctx = crate::pathctx::enter(PathCtx { cwd: Some(anchor.to_string()), root: Some(root()), session_id: None });
    let resolved = crate::pathctx::resolve(target).into_owned();
    if crate::engine::resolve::is_unpinnable(&resolved) {
        return LocalLocus::Machine;
    }
    let role = classify_region(&resolved);
    role.write_locus.max(role.rebind_locus)
}

fn segment() -> impl Strategy<Value = String> {
    prop_oneof![
        8 => "[a-z]{1,6}(\\.[a-z]{1,3})?",
        3 => Just("..".to_string()),
        1 => Just(".".to_string()),
        1 => Just(".git".to_string()),
        1 => Just("hooks".to_string()),
        1 => Just("pre-commit".to_string()),
        1 => Just(".ssh".to_string()),
        1 => Just("authorized_keys".to_string()),
        1 => Just(".envrc".to_string()),
        1 => Just(".zshrc".to_string()),
        1 => Just("Library".to_string()),
        1 => Just("LaunchAgents".to_string()),
        1 => Just("bin".to_string()),
        1 => Just(".config".to_string()),
        1 => Just("credentials".to_string()),
    ]
}

fn target() -> impl Strategy<Value = String> {
    proptest::collection::vec(segment(), 1..5).prop_map(|s| s.join("/"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1500))]

    #[test]
    fn an_approved_write_lands_nowhere_protected_in_any_covered_folder(
        spelling in proptest::sample::select(SPELLINGS),
        t in target(),
        d in target(),
        level in prop_oneof![Just(FolderLevel::Developer), Just(FolderLevel::Workspace)],
    ) {
        let command = spelling.replace("{t}", &t).replace("{d}", &d);
        if !approved(level, &command) {
            return Ok(());
        }
        let target = if spelling.starts_with("cd ") { format!("{d}/{t}") } else { t.clone() };
        for anchor in covered() {
            let locus = landing(&anchor, &target);
            prop_assert!(locus < LocalLocus::WorktreeTrusted, "`{command}` is approved at {} with the folder unknown, and run in {anchor} it writes `{target}` at {locus:?}", level.name());
        }
        if level == FolderLevel::Developer {
            let home = std::env::var("HOME").unwrap_or_default();
            let _ctx = crate::pathctx::enter(PathCtx { cwd: Some(home.clone()), root: Some(root()), session_id: None });
            let at_home = crate::pathctx::resolve(&target).into_owned();
            prop_assert!(!home.is_empty(), "HOME is unset, so the home check cannot run");
            prop_assert!(!crate::engine::resolve::regions::names_a_region(&at_home), "`{command}` is approved at developer and run in ~ it writes `{at_home}`, a named place");
        }
    }
}

/// The property must be able to fail: with nothing refused, `echo x > .git/hooks/pre-commit` lands
/// in a protected place in the project root.
#[test]
fn the_soundness_check_sees_a_protected_landing() {
    assert!(landing(&root(), ".git/hooks/pre-commit") >= LocalLocus::WorktreeTrusted);
    assert!(landing(&root(), "out.txt") < LocalLocus::WorktreeTrusted);
    assert!(landing(&format!("{}/web", root()), "../../lib/x") < LocalLocus::WorktreeTrusted);
    assert!(!approved(FolderLevel::Workspace, "echo x > .git/hooks/pre-commit"));
}

static ALL_SPELLINGS: std::sync::LazyLock<Vec<&str>> =
    std::sync::LazyLock::new(|| SPELLINGS.iter().copied().chain(["cat {t}", "cp {t} b.txt", "grep -r x {t}", "ls {t}"]).collect());

fn at_root(root: &str, command: &str) -> bool {
    let _ctx = crate::pathctx::enter(PathCtx { cwd: Some(root.to_string()), root: Some(root.to_string()), session_id: None });
    crate::cst::command_verdict(command).is_allowed()
}

/// At `reads` the hook adds the read ceiling; the verdict alone is what the other levels grant.
fn granted(root: &str, level: FolderLevel, command: &str) -> bool {
    let _ctx = crate::pathctx::enter(PathCtx {
        cwd: Some(crate::targets::UNKNOWN_WORKDIR.to_string()),
        root: Some(root.to_string()),
        session_id: None,
    });
    let _folder = enter(level);
    match crate::cst::command_verdict(command) {
        Verdict::Allowed(l) => level > FolderLevel::Reads || l <= SafetyLevel::SafeRead,
        Verdict::Denied => false,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// An unknown folder only takes approvals away: no level approves what the project root, as a
    /// known folder, refuses; and each level approves at least what the stricter one does. The
    /// `level_monotonic` fuzz target asserts the same over arbitrary bytes.
    #[test]
    fn an_unknown_folder_never_approves_what_the_root_refuses(
        spelling in proptest::sample::select(ALL_SPELLINGS.clone()),
        t in target(),
        d in target(),
        elsewhere in proptest::bool::ANY,
    ) {
        let command = spelling.replace("{t}", &t).replace("{d}", &d);
        // A project outside the home directory too, where a parent can be another user's home.
        let root = if elsewhere { someone_else() } else { root() };
        let root_allows = at_root(&root, &command);
        let mut stricter: Option<(&str, bool)> = None;
        for level in [FolderLevel::Reads, FolderLevel::Developer, FolderLevel::Workspace] {
            let allowed = granted(&root, level, &command);
            prop_assert!(!allowed || root_allows, "`{command}` is approved at {} with the folder unknown but refused in the project root", level.name());
            if let Some((name, was)) = stricter {
                prop_assert!(!was || allowed, "`{command}`: approved at {name} but not at {}", level.name());
            }
            stricter = Some((level.name(), allowed));
        }
    }
}

/// The `level_monotonic` find (2026-10-08): `cp a ../ x` was approved at `developer` because `../`
/// joined onto the unknown folder named an ordinary directory, while the project's real parent was
/// another user's home. A path that climbs out of an unknown folder is now placed nowhere.
#[test]
fn a_path_climbing_out_of_an_unknown_folder_is_placed_nowhere() {
    let elsewhere = &someone_else();
    for command in ["cat ../x", "cp a ../ b", "cp ../x b", "cat ../../x"] {
        assert!(!at_root(elsewhere, command), "precondition: `{command}` is refused in a project under another user's home");
        for level in [FolderLevel::Reads, FolderLevel::Developer, FolderLevel::Workspace] {
            assert!(!granted(elsewhere, level, command), "`{command}` approved at {}", level.name());
        }
    }
}

/// A project in ANOTHER user's home: a sibling of the real home directory, where a parent of the
/// project is that user's files.
fn someone_else() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/nonexistent-home".to_string());
    let users = home.rsplit_once('/').map_or("", |(p, _)| p);
    format!("{users}/sc-someone-else/projects/app")
}
