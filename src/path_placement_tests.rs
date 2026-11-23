//! Placement invariants for paths: `~/…` spellings of a workspace path, and a workspace root that
//! contains a protected place.

use proptest::prelude::*;

use crate::command_verdict_in;
use crate::pathctx::PathCtx;

fn workspace() -> PathCtx {
    PathCtx { cwd: Some("/work".into()), root: Some("/work".into()), ..Default::default() }
}

/// The `$HOME` that `~` stands for. `None` (the guard then has nothing to probe) only where the
/// test process has no absolute home. A stand-in installed with [`crate::pathctx::enter_home`]
/// wins over the process home.
fn absolute_home() -> Option<String> {
    crate::pathctx::home_dir().filter(|h| h.len() > 1)
}

/// Contexts that consume a path and gate differently: plain and recursive reads, a write, an
/// in-place edit, and a delete. `{}` is the path slot.
const HOME_SPELLING_CONTEXTS: &[&str] = &["cat {}", "grep -rn x {}", "ls {}", "echo hi > {}", "sed -i s/a/b/ {}", "rm -rf {}"];

/// Paths under the workspace's parent: the workspace itself, a sibling, a name sharing its prefix,
/// and names that hit the frozen and credential rungs at any depth.
fn arb_home_relative_path() -> impl Strategy<Value = String> {
    let first = proptest::sample::select(vec!["app", "app", "peer", "app-evil", ".config", ".ssh"]);
    let rest = proptest::collection::vec(
        proptest::sample::select(vec!["src", "main.rs", "notes", ".git", "hooks", "pre-commit", ".envrc", ".ssh", "id_rsa", ".aws", "x"]),
        0..4,
    );
    (first, rest).prop_map(|(f, r)| std::iter::once(f).chain(r).collect::<Vec<_>>().join("/"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    /// `~/p` and `$HOME/p` name one file, so they classify alike. A workspace at
    /// `~/projects/app` saw `grep -r x ~/projects/app/src` refused as
    /// outside the working directory while the `$HOME/…` spelling was approved, because only the
    /// absolute spelling was resolved against the root.
    #[test]
    fn home_and_absolute_spellings_classify_alike(rel in arb_home_relative_path(), ctx in proptest::sample::select(HOME_SPELLING_CONTEXTS.to_vec())) {
        let _stand_in = crate::pathctx::enter_home(stand_in_home().to_string());
        let Some(home) = absolute_home() else { return Ok(()) };
        let root = format!("{home}/projects/app");
        let ws = || PathCtx { cwd: Some(root.clone()), root: Some(root.clone()), ..Default::default() };
        let tilde = ctx.replace("{}", &format!("~/projects/{rel}"));
        let absolute = ctx.replace("{}", &format!("{home}/projects/{rel}"));
        prop_assert_eq!(
            command_verdict_in(&tilde, ws()).is_allowed(),
            command_verdict_in(&absolute, ws()).is_allowed(),
            "`{}` and `{}` disagree", tilde, absolute,
        );
    }
}

/// A workspace whose root CONTAINS a fixed-place protection (a session started in `/`, `/etc` or
/// `$HOME`) must not make that place an ordinary worktree file, whichever way the path is spelled.
/// The expectation is the same command from an unrelated workspace, so the guard asserts no more
/// than the region table already says. Every anchored region that applies on the platform is
/// probed, on both platforms, so a newly declared one is covered the day it lands. A root AT or
/// inside the protected place is where the user chose to work, and is not probed.
#[test]
fn a_root_above_a_protected_place_does_not_unprotect_it() {
    let _stand_in = crate::pathctx::enter_home(stand_in_home().to_string());
    for os in ["linux", "macos"] {
        let failures = crate::engine::resolve::regions::with_os(os, roots_above_protected_places_unprotecting);
        assert!(failures.is_empty(), "[{os}] a root above a protected place unprotected it:\n  {}", failures.join("\n  "));
    }
}

fn roots_above_protected_places_unprotecting() -> Vec<String> {
    let Some(home) = absolute_home() else { return Vec::new() };
    let anchored: Vec<String> = crate::engine::resolve::regions::anchored_protected_paths_here()
        .into_iter()
        .filter(|p| !p.contains('*'))
        .collect();
    assert!(anchored.len() > 20, "the region table must supply the witnesses");
    let mut failures = Vec::new();
    for region in &anchored {
        let place = match region.trim_end_matches('/').strip_prefix('~') {
            Some(rest) => format!("{home}{rest}"),
            None => region.trim_end_matches('/').to_string(),
        };
        let file = if region.ends_with('/') { format!("{region}inner") } else { region.clone() };
        let absolute = match file.strip_prefix('~') {
            Some(rest) => format!("{home}{rest}"),
            None => file.clone(),
        };
        let parents: Vec<String> = std::iter::successors(std::path::Path::new(&absolute).parent(), |p| p.parent())
            .map(|p| p.to_string_lossy().into_owned())
            .filter(|p| !p.is_empty() && *p != place && !p.starts_with(&format!("{place}/")))
            .collect();
        for ctx in ["cat {}", "echo hi > {}", "rm -rf {}"] {
            let elsewhere = command_verdict_in(&ctx.replace("{}", &absolute.replace(' ', "\\ ")), workspace()).is_allowed();
            for root in &parents {
                let here = PathCtx { cwd: Some(root.clone()), root: Some(root.clone()), ..Default::default() };
                let relative = absolute.strip_prefix(root.as_str()).map_or(absolute.as_str(), |r| r.trim_start_matches('/'));
                for spelling in [absolute.as_str(), relative, file.as_str()] {
                    let line = ctx.replace("{}", &spelling.replace(' ', "\\ "));
                    if command_verdict_in(&line, here.clone()).is_allowed() && !elsewhere {
                        failures.push(format!("`{line}` from root `{root}`"));
                    }
                }
            }
        }
    }
    failures
}

/// Another user's home is shielded by its shape rather than by a declared region, so the region
/// table cannot supply it as a witness: probed directly, from each root above it.
#[test]
fn another_users_home_stays_shielded_from_a_root_above_it() {
    for root in ["/", "/Users", "/home"] {
        let here = PathCtx { cwd: Some(root.into()), root: Some(root.into()), ..Default::default() };
        for parent in ["/Users", "/home"] {
            let path = format!("{parent}/someone-else/notes.txt");
            if !path.starts_with(&format!("{}/", root.trim_end_matches('/'))) {
                continue;
            }
            let relative = path.strip_prefix(root).unwrap_or(&path).trim_start_matches('/');
            for spelling in [path.as_str(), relative] {
                let line = format!("cat {spelling}");
                assert!(!command_verdict_in(&line, here.clone()).is_allowed(), "`{line}` from root `{root}`");
            }
        }
    }
}

/// Paths under `$HOME` and `/`, including the whole home, dotfiles, credential stores, the trust
/// files and system directories that contain protected places.
///
/// A glob is not appended to another user's home. Listing `/Users` or `/home` opens a real
/// directory, and from launchd that `open` can wait forever on a TCC prompt. The names are still
/// judged; only the directory walk is kept off the real tree.
fn arb_sweep_target() -> impl Strategy<Value = String> {
    let first = proptest::sample::select(vec![
        "~", "~/", "~/projects", "~/.zshrc", "~/.ssh", "~/.ssh/id_rsa", "~/.config", "~/.config/safe-chains.toml",
        "~/.claude/settings.json", "~/Library", "~/Library/Keychains", "~/Library/LaunchAgents/x.plist", "/etc", "/etc/shadow",
        "/etc/sudoers", "/Users/someone-else", "/Users/someone-else/notes.txt", "/home/someone-else", "/home/someone-else/notes.txt",
    ]);
    let glob = proptest::sample::select(vec!["", "/*", "/.ss?", "/x"]);
    (first, glob).prop_map(|(f, g)| {
        let base = f.trim_end_matches('/');
        if base.starts_with("/Users") || base.starts_with("/home") { base.to_string() } else { format!("{base}{g}") }
    })
}

/// A home directory full of the protected-place names a real home has, under a temp directory.
/// Listings of `~/Library/Keychains` and `~/Desktop` hit this tree.
fn stand_in_home() -> &'static str {
    struct Kept(tempfile::TempDir, String);
    static FIXTURE: std::sync::OnceLock<Kept> = std::sync::OnceLock::new();
    let kept = FIXTURE.get_or_init(|| {
        let dir = tempfile::tempdir().expect("stand-in home");
        let home = dir.path().join("Users").join("me");
        for rel in [
            "Desktop", "Documents", "Downloads", "Movies", "Music", "Pictures", "Library/Keychains", "Library/Messages", "Library/Mail",
            "Library/Safari", "Library/Cookies", "Library/LaunchAgents", ".ssh", ".config", ".config/gh", ".claude", "projects",
        ] {
            std::fs::create_dir_all(home.join(rel)).expect("fixture dir");
        }
        std::fs::write(home.join(".zshrc"), "").expect("fixture file");
        std::fs::write(home.join(".ssh/id_rsa"), "").expect("fixture file");
        std::fs::write(home.join(".config/safe-chains.toml"), "").expect("fixture file");
        std::fs::write(home.join(".config/gh/hosts.yml"), "").expect("fixture file");
        std::fs::write(home.join(".claude/settings.json"), "").expect("fixture file");
        std::fs::write(home.join("Library/LaunchAgents/x.plist"), "").expect("fixture file");
        let home = home.to_str().expect("utf-8 temp path").to_string();
        let real = crate::process_home::var().ok();
        assert!(!crate::pathctx::walks_real_home(&home, real.as_deref()), "the stand-in {home} is inside the real home");
        Kept(dir, home)
    });
    // The directory is held so the fixture stays on disk for the whole process.
    let _held = &kept.0;
    kept.1.as_str()
}

/// Listing the process home fails the test. `~/Library/Keychains` is one of the directories the
/// placement property opened; the predicate covers that path. This call stats the home directory
/// itself, which exists and is not a TCC-protected folder, so the guard's check cannot block.
#[test]
#[should_panic(expected = "under the real home or /Users or /home")]
fn listing_the_real_home_fails_the_test() {
    let home = crate::process_home::var().expect("HOME");
    crate::pathctx::guard_listing(&home);
}

#[test]
fn a_fixture_listing_is_not_the_real_home() {
    let real = crate::process_home::var().ok();
    crate::pathctx::guard_listing(stand_in_home());
    crate::pathctx::guard_listing("/");
    crate::pathctx::guard_listing("/etc");
    for path in [
        "/Users", "/Users/someone-else", "//Users", "/home", "/home/someone-else", "/System/Volumes/Data/Users",
        "/System/Volumes/Data/home",
    ] {
        assert!(crate::pathctx::walks_real_home(path, real.as_deref()), "{path}");
    }
    if cfg!(target_os = "macos") {
        assert!(crate::pathctx::walks_real_home("/USERS", real.as_deref()));
    }
    if let Some(home) = real.as_deref() {
        assert!(crate::pathctx::walks_real_home(home, Some(home)), "{home}");
        assert!(crate::pathctx::walks_real_home(&format!("{home}/Library/Keychains"), Some(home)));
        assert!(crate::pathctx::walks_real_home(&format!("/{home}"), Some(home)));
    }
    assert!(!crate::pathctx::walks_real_home(stand_in_home(), real.as_deref()));
    assert!(!crate::pathctx::walks_real_home("/", real.as_deref()));
    assert!(!crate::pathctx::walks_real_home("/etc", real.as_deref()));
    assert!(!crate::pathctx::walks_real_home("/usr", real.as_deref()));
    assert!(!crate::pathctx::walks_real_home("/System/Volumes/Data", real.as_deref()));
}

/// Contexts that sweep, persist, delete or exfiltrate. `{}` is the path slot.
const SWEEP_CONTEXTS: &[&str] =
    &["cat {}", "grep -r token {}", "rm -rf {}", "echo x >> {}", "cp -r {} /tmp/x", "tar czf /tmp/x.tgz {}", "find {} -delete"];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(600))]

    /// A workspace rooted at `$HOME`, `/` or `/Users` is never more permissive than an unrelated
    /// one, in any spelling of the path: `~/…`, absolute, or relative to that root.
    #[test]
    fn a_root_holding_protected_places_is_never_more_permissive(target in arb_sweep_target(), ctx in proptest::sample::select(SWEEP_CONTEXTS.to_vec()), root_pick in 0usize..3) {
        let _stand_in = crate::pathctx::enter_home(stand_in_home().to_string());
        let Some(home) = absolute_home() else { return Ok(()) };
        let root = [home.as_str(), "/", "/Users"][root_pick].to_string();
        let absolute = match target.strip_prefix('~') {
            Some(rest) => format!("{home}{rest}"),
            None => target.clone(),
        };
        let elsewhere = command_verdict_in(&ctx.replace("{}", &absolute), workspace()).is_allowed()
            && command_verdict_in(&ctx.replace("{}", &target), workspace()).is_allowed();
        let here = PathCtx { cwd: Some(root.clone()), root: Some(root.clone()), ..Default::default() };
        let relative = match absolute.strip_prefix(root.trim_end_matches('/')) { Some("") => ".", Some(r) if r.starts_with('/') => r.trim_start_matches('/'), _ => absolute.as_str() };
        for spelling in [target.as_str(), absolute.as_str(), relative] {
            let line = ctx.replace("{}", spelling);
            prop_assert!(elsewhere || !command_verdict_in(&line, here.clone()).is_allowed(), "`{}` from root `{}` approves, from /work it does not", line, root);
        }
    }
}

/// The root arrives as the harness spelled it: macOS reports a session started in `/etc` as
/// `/private/etc` (a firmlink only there), and a root may carry `.`, `..` or a case variant.
/// Each names a root that holds protected places and must be recognised as one.
#[test]
fn a_root_holding_protected_places_is_recognised_in_every_spelling() {
    let Some(home) = absolute_home() else { return };
    let mut cases = vec![
        ("/.".to_string(), vec!["cat etc/master.passwd", "rm -rf etc"]),
        (format!("{home}/."), vec!["echo x >> .zshrc", "rm -rf ."]),
        (format!("{home}/projects/.."), vec!["echo x >> .zshrc", "cat .config/gh/hosts.yml"]),
    ];
    if cfg!(target_os = "macos") {
        cases.push(("/private/etc".to_string(), vec!["cat master.passwd", "echo x > sudoers", "rm -rf ."]));
        cases.push(("/private".to_string(), vec!["cat etc/master.passwd", "echo x > etc/sudoers", "rm -rf etc"]));
        cases.push(("/ETC".to_string(), vec!["echo x > sudoers"]));
        cases.push((home.to_uppercase(), vec!["echo x >> .zshrc", "echo x > .config/safe-chains.toml"]));
    }
    for (root, commands) in cases {
        let here = PathCtx { cwd: Some(root.clone()), root: Some(root.clone()), ..Default::default() };
        for line in commands {
            assert!(!command_verdict_in(line, here.clone()).is_allowed(), "`{line}` from root `{root}`");
        }
    }
}

/// A root below every home that holds a protected place keeps working as a project: only the
/// protected place, what is inside it, and the directories above it stay out of the worktree.
#[test]
fn a_project_root_holding_a_trust_file_still_writes_its_other_files() {
    let _stand_in = crate::pathctx::enter_home(stand_in_home().to_string());
    let Some(home) = absolute_home() else { return };
    let root = format!("{home}/.claude");
    let here = PathCtx { cwd: Some(root.clone()), root: Some(root), ..Default::default() };
    assert!(command_verdict_in("cat settings.json", here.clone()).is_allowed(), "`cat settings.json` should approve");
    for line in [
        "echo x > CLAUDE.md", "echo x > skills/foo/SKILL.md", "echo x > settings.json", "echo x > settings.jso?", "rm -rf .",
        "cp x settings.json",
    ] {
        assert!(!command_verdict_in(line, here.clone()).is_allowed(), "`{line}` should ask");
    }
}

proptest! {
    /// A copy into a directory writes the name the source arrives under, so it is judged there too.
    /// `cp /tmp/.envrc .` wrote `./.envrc`, which direnv runs on the next `cd`, and was approved in
    /// a known workspace because only the destination `.` was classified. Every spelling of a
    /// transfer with a frozen basename is refused, wherever in the workspace it lands.
    #[test]
    fn a_transfer_is_judged_at_the_name_it_arrives_under(
        verb in proptest::sample::select(vec!["cp", "cp -r", "mv", "cp -f"]),
        frozen in proptest::sample::select(vec![".envrc", ".git"]),
        from in proptest::sample::select(vec!["/tmp", "/tmp/x", "./src", "vendor"]),
        into in proptest::sample::select(vec![".", "./", "sub", "sub/", "a/b"]),
    ) {
        let command = format!("{verb} {from}/{frozen} {into}");
        prop_assert!(!command_verdict_in(&command, workspace()).is_allowed(), "`{command}` writes {into}/{frozen}");
    }
}

#[test]
fn a_transfer_with_an_ordinary_name_still_lands_in_the_workspace() {
    for command in ["cp /tmp/notes.md .", "cp a.txt b.txt", "cp a.txt out/", "mv a.txt sub/", "cp -r /tmp/d ."] {
        assert!(command_verdict_in(command, workspace()).is_allowed(), "{command}");
    }
}

/// An in-place edit's backup is a write too. GNU sed and perl put the file's name where the
/// suffix has a `*` and accept a directory in it, so this wrote a git hook from a plain file name.
#[test]
fn an_in_place_backup_is_judged_where_it_lands() {
    for command in [
        "sed -i'.git/hooks/*' s/x/x/ pre-commit", "sed --in-place=.git/hooks/* s/x/x/ pre-commit",
        "perl -i'.git/hooks/*' -pe 1 pre-commit", "perl -pi.git/hooks/* -e 1 pre-commit",
    ] {
        assert!(!command_verdict_in(command, workspace()).is_allowed(), "`{command}` writes its backup into a protected place");
    }
    for command in ["sed -i.bak s/x/y/ notes", "sed -i '' s/x/y/ notes", "perl -pi.orig -e 1 notes"] {
        assert!(command_verdict_in(command, workspace()).is_allowed(), "{command}");
    }
}
