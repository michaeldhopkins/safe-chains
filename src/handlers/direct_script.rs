use std::borrow::Cow;

use crate::parse::Token;

pub(crate) fn is_relative_command_path(word: &str) -> bool {
    !word.is_empty() && !word.starts_with(['@', '/', '~']) && !word.contains('$') && word.contains('/')
}

pub(crate) fn as_interpreter(tokens: &[Token]) -> Cow<'_, [Token]> {
    if tokens.first().is_some_and(|t| is_relative_command_path(t.as_str())) {
        let mut owned = Vec::with_capacity(tokens.len() + 1);
        owned.push(Token::from_raw("bash".to_string()));
        owned.extend(tokens.iter().cloned());
        Cow::Owned(owned)
    } else {
        Cow::Borrowed(tokens)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use proptest::prelude::*;

    use crate::pathctx::PathCtx;
    use crate::verdict::{SafetyLevel, Verdict};

    const ARGS: &[&str] = &["", " --flag", " arg1 arg2", " https://example.com"];

    fn corpus() -> Vec<&'static str> {
        vec![
            "./run.sh", "scripts/deploy.sh", "deploy/tests/push_main_test.sh", "./deploy/tests/push_main_test.sh", "scripts/attest",
            "./bin/check.sh", "./cat", "bin/git", "./curl", "foo/../inside.sh", "../x.sh", "foo/../../etc/passwd", "../bin/rails",
            "/tmp/x.sh", "/tmp/cat", "/usr/local/bin/x", "~/x.sh", "~/Downloads/x.sh", "~/bin/grep", ".git/hooks/pre-commit",
            ".jj/working_copy", ".hg/hgrc", ".sl/config", "./.envrc", "links/in", "links/out", "links/cat", "links/pest",
        ]
    }

    fn project() -> PathCtx {
        static CTX: OnceLock<PathCtx> = OnceLock::new();
        CTX.get_or_init(|| {
            let root = tempfile::tempdir().expect("root");
            let outside = tempfile::tempdir().expect("outside");
            let r = root.path();
            for dir in ["scripts", "deploy/tests", "bin", "links", "foo"] {
                std::fs::create_dir_all(r.join(dir)).expect("dir");
            }
            std::fs::write(r.join("inside.sh"), "echo\n").expect("inside");
            std::fs::write(outside.path().join("x.sh"), "echo\n").expect("outside");
            std::os::unix::fs::symlink("../inside.sh", r.join("links/in")).expect("link in");
            std::os::unix::fs::symlink(outside.path().join("x.sh"), r.join("links/out")).expect("link out");
            std::os::unix::fs::symlink(outside.path().join("x.sh"), r.join("links/cat")).expect("link cat");
            std::os::unix::fs::symlink(outside.path().join("x.sh"), r.join("links/pest")).expect("link pest");
            let s = r.to_str().expect("utf8").to_string();
            std::mem::forget(root);
            std::mem::forget(outside);
            PathCtx { cwd: Some(s.clone()), root: Some(s), ..Default::default() }
        })
        .clone()
    }

    fn pair(path: &str, arg: &str) -> (String, String) {
        (format!("{path}{arg}"), format!("bash {path}{arg}"))
    }

    fn assert_level(direct: &str, via: &str, level_name: &str) {
        let (ceiling, engine) = crate::level_ceiling(level_name).expect("level");
        let d = crate::command_verdict_ceilinged(direct, ceiling, engine);
        let b = crate::command_verdict_ceilinged(via, ceiling, engine);
        assert_eq!(d, b, "{direct} at {level_name}");
        for target in crate::targets::registry() {
            for format in target.hook_formats() {
                let left = crate::targets::respond(format, direct, d).map(|r| r.exit_code);
                let right = crate::targets::respond(format, via, b).map(|r| r.exit_code);
                assert_eq!(left, right, "{}: {direct} at {level_name}", target.name());
            }
        }
    }

    fn unknown_folder(command: &str) -> (Verdict, Vec<crate::pathctx::folder::Note>) {
        let _ctx = crate::pathctx::enter(PathCtx {
            cwd: Some(crate::targets::UNKNOWN_WORKDIR.to_string()),
            root: Some("/work/projects/app".to_string()),
            session_id: None,
        });
        let _folder = crate::pathctx::folder::enter(crate::pathctx::anchor::FolderLevel::Developer);
        let verdict = crate::command_verdict(command);
        (verdict, crate::pathctx::folder::notes())
    }

    fn assert_same(path: &str, arg: &str) {
        let _ctx = crate::pathctx::enter(project());
        let (direct, via) = pair(path, arg);
        assert_eq!(crate::command_verdict(&direct), crate::command_verdict(&via), "{direct}");
        let routed = super::is_relative_command_path(path);
        for level in crate::engine::authoring::default_levels() {
            let in_band = matches!(level.name.as_str(), "paranoid" | "reader" | "editor" | "developer");
            if routed || in_band {
                assert_level(&direct, &via, &level.name);
            }
        }
        if routed {
            assert_eq!(unknown_folder(&direct), unknown_folder(&via), "{direct} in an unknown folder");
        }
    }

    #[test]
    fn every_corpus_path_matches_bash() {
        for path in corpus() {
            for arg in ARGS {
                assert_same(path, arg);
            }
        }
    }

    #[test]
    fn project_scripts_are_approved() {
        for cmd in ["deploy/tests/push_main_test.sh", "./deploy/tests/push_main_test.sh", "scripts/attest", "./bin/check.sh"] {
            assert_eq!(crate::command_verdict(cmd), Verdict::Allowed(SafetyLevel::SafeWrite), "{cmd}");
            assert_eq!(crate::command_verdict(cmd), crate::command_verdict(&format!("bash {cmd}")), "{cmd}");
        }
    }

    #[test]
    fn escapes_and_foreign_paths_are_refused() {
        for cmd in ["../x.sh", "/tmp/x.sh", "~/x.sh"] {
            let verdict = crate::command_verdict(cmd);
            assert_eq!(verdict, crate::command_verdict(&format!("bash {cmd}")), "{cmd}");
            assert!(!verdict.is_allowed(), "{cmd} -> {verdict:?}");
        }
    }

    #[test]
    fn a_vcs_hook_matches_bash() {
        let cmd = ".git/hooks/pre-commit";
        let verdict = crate::command_verdict(cmd);
        assert_eq!(verdict, crate::command_verdict(&format!("bash {cmd}")));
        assert_eq!(verdict, Verdict::Allowed(SafetyLevel::SafeWrite));
    }

    #[test]
    fn a_bare_name_stays_a_path_lookup() {
        assert_eq!(crate::command_verdict("push_main_test.sh"), Verdict::Denied);
        assert_eq!(crate::command_verdict("bash push_main_test.sh"), Verdict::Allowed(SafetyLevel::SafeWrite));
    }

    #[test]
    fn arguments_after_the_script_follow_the_bash_rule() {
        let direct = crate::command_verdict("./bin/check.sh --no-progress extra");
        let via = crate::command_verdict("bash ./bin/check.sh --no-progress extra");
        assert_eq!(direct, via);
        assert_eq!(direct, Verdict::Allowed(SafetyLevel::SafeWrite));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn a_direct_worktree_script_matches_bash_on_every_level_and_target(
            path in prop::sample::select(corpus()),
            arg in prop::sample::select(ARGS.to_vec()),
        ) {
            assert_same(path, arg);
        }
    }
}
