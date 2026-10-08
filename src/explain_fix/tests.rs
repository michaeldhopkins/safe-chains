use super::*;
use crate::engine::resolve::regions::hypothetical::with_hypothetical_grant;
use crate::pathctx::{PathCtx, enter};
use proptest::prelude::*;

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/home/tester".into())
}

/// The fix for `command` run from a project under the home directory, at the default level.
fn fix(command: &str) -> Fix {
    let project = format!("{}/projects/fixprobe", home());
    let _ctx = enter(PathCtx { cwd: Some(project.clone()), root: Some(project), session_id: None });
    fix_for(command, &|c| crate::is_safe_command(c), false)
}

#[test]
fn an_unknown_command_is_fixed_by_an_entry() {
    assert_eq!(
        fix("zzfixtool sync --dry-run"),
        Fix::AddEntry { names: vec!["zzfixtool".into()], command: "zzfixtool sync --dry-run".into() }
    );
}

#[test]
fn a_known_command_refused_for_an_option_has_no_config_fix() {
    match fix("git push --force origin main") {
        Fix::Nothing(Because::BuiltInRule { names }) => assert!(names.is_empty() || names == ["git"], "{names:?}"),
        other => panic!("expected a built-in rule, got {other:?}"),
    }
}

#[test]
fn a_write_outside_the_project_is_fixed_by_a_grant_on_its_folder() {
    match fix(&format!("tee {}/fixnotes/today.md", home())) {
        Fix::Grant { path, access } => {
            assert_eq!(path, "~/fixnotes");
            assert_eq!(access, Access::Write, "the narrowest face that works");
        }
        other => panic!("expected a grant, got {other:?}"),
    }
}

#[test]
fn protected_paths_have_no_config_fix() {
    for (command, expected) in [
        (format!("cat {}/.ssh/id_rsa", home()), ReachReason::Credential),
        ("tee /etc/sudoers".to_string(), ReachReason::FrozenSystemIntegrity),
        (format!("tee {}/.config/safe-chains.toml", home()), ReachReason::FrozenTrustFile),
    ] {
        match fix(&command) {
            Fix::Nothing(Because::ProtectedPath { reason, .. }) => assert_eq!(reason, expected, "{command}"),
            other => panic!("{command}: expected a protected path, got {other:?}"),
        }
    }
}

#[test]
fn the_whole_disk_is_too_wide_to_grant() {
    assert!(matches!(fix("rm -rf /"), Fix::Nothing(Because::TooWide { .. }) | Fix::Nothing(Because::BuiltInRule { .. })));
}

#[test]
fn an_unparseable_command_says_so() {
    assert_eq!(fix("echo 'unterminated"), Fix::Nothing(Because::Unparseable));
}

#[test]
fn a_stricter_level_is_named_rather_than_blamed_on_a_rule() {
    assert_eq!(fix_for("zzfixtool", &|_| false, true), Fix::RaiseLevel);
}

/// The pairing the module exists for: a grant is offered only when it would work, which the
/// classifier confirms with that grant in place, and never on a path `config grant` would refuse.
#[test]
fn every_offered_grant_changes_the_verdict() {
    let h = home();
    let project = format!("{h}/projects/fixprobe");
    let _ctx = enter(PathCtx { cwd: Some(project.clone()), root: Some(project), session_id: None });
    let mut offered = 0;
    for command in [
        format!("tee {h}/fixnotes/a.md"),
        format!("cp ./x {h}/fixdocs/report.txt"),
        format!("mkdir {h}/fixbuild"),
        format!("touch {h}/fixdir/sub/file.rs"),
        format!("rm {h}/fixtrash/old.log"),
        format!("cat {h}/.ssh/id_rsa"),
        "tee /etc/hosts".to_string(),
        "touch /opt/fixtool/x".to_string(),
    ] {
        assert!(!crate::is_safe_command(&command), "{command} must start refused");
        if let Fix::Grant { path, access } = fix_for(&command, &|c| crate::is_safe_command(c), false) {
            offered += 1;
            assert!(crate::userconfig::grantable_path(&path).is_ok(), "{command}: offered {path}, which config grant refuses");
            let (read, write) = (matches!(access, Access::Read | Access::ReadWrite), matches!(access, Access::Write | Access::ReadWrite));
            assert!(
                with_hypothetical_grant(&path, read, write, || crate::is_safe_command(&command)),
                "{command}: the grant on {path} changes nothing"
            );
        }
    }
    assert!(offered >= 3, "only {offered} grants offered; the sweep proves nothing");
}

#[test]
fn every_fix_reads_plainly_and_names_its_next_step() {
    let fixes = [
        Fix::AddEntry { names: vec!["mytool".into()], command: "mytool sync".into() },
        Fix::Grant { path: "~/notes".into(), access: Access::Write },
        Fix::RaiseLevel,
        Fix::Nothing(Because::BuiltInRule { names: vec!["rm".into()] }),
        Fix::Nothing(Because::BuiltInRule { names: vec![] }),
        Fix::Nothing(Because::ProtectedPath { path: "~/.ssh/id_rsa".into(), reason: ReachReason::Credential }),
        Fix::Nothing(Because::ProtectedPath { path: "/dev/mem".into(), reason: ReachReason::RawDevice }),
        Fix::Nothing(Because::ProtectedPath { path: "/etc/sudoers".into(), reason: ReachReason::FrozenSystemIntegrity }),
        Fix::Nothing(Because::Unconfined { path: "./out/$i".into() }),
        Fix::Nothing(Because::TooWide { path: "/".into() }),
        Fix::Nothing(Because::RunsLater { path: "~/Library/LaunchAgents".into() }),
        Fix::Nothing(Because::RunsAnAgent { name: "goose".into() }),
        Fix::Nothing(Because::Unparseable),
    ];
    for f in fixes {
        let text = f.render();
        assert!(text.starts_with("Fix: "), "{text}");
        assert!(!text.contains('—') && !text.contains(';'), "{text}");
        for word in crate::refusal::AVOID {
            assert!(!text.to_lowercase().contains(word), "`{word}` in {text}");
        }
    }
    assert!(
        Fix::AddEntry { names: vec!["mytool".into()], command: "mytool sync".into() }
            .render()
            .contains("safe-chains config add 'mytool sync'")
    );
    assert!(
        Fix::Grant { path: "~/my notes".into(), access: Access::Read }
            .render()
            .contains("safe-chains config grant ~/'my notes' --read")
    );
}

proptest! {
    /// Command text never forges a line of the fix.
    #[test]
    fn command_text_cannot_add_a_line(name in "[a-z]{3,8}", tail in "[a-z\n\r ]{0,20}") {
        let text = Fix::AddEntry { names: vec![name.clone()], command: format!("{name} {tail}") }.render();
        prop_assert_eq!(text.lines().count(), 3, "{}", text);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Whatever folder a write lands in, a grant is offered only if it works.
    #[test]
    fn an_offered_grant_always_works(folder in "zz[a-z]{2,8}", file in "[a-z]{1,8}\\.(txt|md|rs)", verb in prop::sample::select(vec!["tee", "touch", "rm", "mkdir"])) {
        let command = format!("{verb} /opt/{folder}/{file}");
        let _ctx = enter(PathCtx { cwd: Some("/work/app".into()), root: Some("/work/app".into()), session_id: None });
        if let Fix::Grant { path, access } = fix_for(&command, &|c| crate::is_safe_command(c), false) {
            let (read, write) = (matches!(access, Access::Read | Access::ReadWrite), matches!(access, Access::Write | Access::ReadWrite));
            prop_assert!(with_hypothetical_grant(&path, read, write, || crate::is_safe_command(&command)), "{} on {}", command, path);
            prop_assert!(crate::userconfig::grantable_path(&path).is_ok());
        }
    }
}

/// Found in review: `--explain` offered a grant on `~/Library/LaunchAgents`, where a grant would let
/// an agent install something that runs at every login.
#[test]
fn a_place_that_runs_later_is_never_offered() {
    let fix = fix(&format!("tee {}/Library/LaunchAgents/x.plist", home()));
    assert!(matches!(fix, Fix::Nothing(Because::RunsLater { .. })), "{fix:?}");
}
