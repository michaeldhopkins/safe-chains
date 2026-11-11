use std::borrow::Cow;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn is_relative_command_path(word: &str) -> bool {
    !word.is_empty() && !word.starts_with(['@', '/', '~']) && !word.contains('$') && word.contains('/')
}

pub(crate) use crate::engine::resolve::trusted_command_path;

fn is_untrusted_command_path(word: &str) -> bool {
    word.contains('/') && !word.starts_with('@') && !is_relative_command_path(word) && !trusted_command_path(word)
}

pub(crate) fn judged(tokens: &[Token], judge: impl Fn(&[Token]) -> Verdict) -> Verdict {
    if tokens.first().is_some_and(|t| is_untrusted_command_path(t.as_str())) && !crate::handlers::judges_its_own_path(tokens) {
        return Verdict::Denied;
    }
    let Some(first) = tokens.first().filter(|t| is_relative_command_path(t.as_str())) else {
        return judge(tokens);
    };
    let mut verdict = judge(tokens);
    if verdict == Verdict::Denied {
        return Verdict::Denied;
    }
    verdict = verdict.combine(judge(&as_interpreter(tokens)));
    let mut named = tokens.to_vec();
    for spelling in spellings(first.command_name()) {
        named[0] = Token::from_raw(spelling);
        if crate::handlers::names_a_command(&named) {
            verdict = verdict.combine(judge(&named));
        }
    }
    verdict
}

fn spellings(name: &str) -> Vec<String> {
    let folded = without_build_hash(&name.to_ascii_lowercase().replace('_', "-")).to_string();
    let stem = folded
        .split_once('.')
        .map(|(stem, _)| without_build_hash(stem).to_string())
        .filter(|stem| is_an_identity(stem));
    let mut all = vec![name.to_string()];
    for spelling in [Some(folded), stem].into_iter().flatten() {
        if !spelling.is_empty() && !all.contains(&spelling) {
            all.push(spelling);
        }
    }
    all.retain(|s| !s.is_empty());
    all
}

fn without_build_hash(name: &str) -> &str {
    match name.rsplit_once('-') {
        Some((stem, hash)) if !stem.is_empty() && hash.len() == 16 && hash.bytes().all(|b| b.is_ascii_hexdigit()) => stem,
        _ => name,
    }
}

fn is_an_identity(name: &str) -> bool {
    name == "safe-chains" || crate::userconfig::AGENT_COMMANDS.contains(&name)
}

/// A shell handed a script from installed packages (`node_modules`) or a frozen folder (`.git`,
/// `.envrc`), whose code no commit of the project's own shows, runs that code unjudged.
pub(crate) fn runs_packaged_or_frozen(tokens: &[Token]) -> bool {
    let mut words = tokens.iter().skip(1).map(Token::as_str);
    let script = loop {
        match words.next() {
            Some("-o" | "+o" | "-O" | "+O") => {
                words.next();
            }
            Some("--") => break words.next(),
            Some(flag) if flag.starts_with(['-', '+']) && flag.len() > 1 => {
                if !flag.starts_with("--") && flag.contains('c') {
                    return false;
                }
            }
            other => break other,
        }
    };
    use crate::engine::resolve::frozen_reach::{is_frozen, is_packaged};
    script.is_some_and(|path| path.contains('/') && path.split('/').any(|part| is_frozen(part) || is_packaged(part)))
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

    /// A program named by a path that no rule names is its own program and is refused, on every
    /// level and in every hook format, whatever a shell running the same file would be allowed.
    fn assert_refused(path: &str, arg: &str) {
        let _ctx = crate::pathctx::enter(project());
        let direct = format!("{path}{arg}");
        assert_eq!(crate::command_verdict(&direct), Verdict::Denied, "{direct}");
        for level in crate::engine::authoring::default_levels() {
            let (ceiling, engine) = crate::level_ceiling(&level.name).expect("level");
            let verdict = crate::command_verdict_ceilinged(&direct, ceiling, engine);
            assert_eq!(verdict, Verdict::Denied, "{direct} at {}", level.name);
            for target in crate::targets::registry() {
                for format in target.hook_formats() {
                    let response = crate::targets::respond(format, &direct, verdict).map(|r| r.exit_code);
                    let refused = crate::targets::respond(format, &direct, Verdict::Denied).map(|r| r.exit_code);
                    assert_eq!(response, refused, "{}: {direct} at {}", target.name(), level.name);
                }
            }
        }
        if super::is_relative_command_path(path) {
            assert_eq!(unknown_folder(&direct).0, Verdict::Denied, "{direct} in an unknown folder");
        }
    }

    #[test]
    fn every_corpus_path_is_refused() {
        for path in corpus() {
            for arg in ARGS {
                assert_refused(path, arg);
            }
        }
    }

    #[test]
    fn project_scripts_are_their_own_programs() {
        let _ctx = crate::pathctx::enter(project());
        for cmd in ["deploy/tests/push_main_test.sh", "./deploy/tests/push_main_test.sh", "scripts/attest", "./bin/check.sh"] {
            assert_eq!(crate::command_verdict(cmd), Verdict::Denied, "{cmd}");
            assert_eq!(crate::command_verdict(&format!("bash {cmd}")), Verdict::Allowed(SafetyLevel::SafeWrite), "bash {cmd}");
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
    fn a_path_that_spells_a_command_is_no_better_than_that_command() {
        let _ctx = crate::pathctx::enter(project());
        for cmd in [
            "./target/debug/safe-chains config add rm",
            "target/release/safe-chains mcp",
            "./claude -p hi",
            "./rm -rf /",
            "bin/bash -c 'rm -rf ~'",
            "node_modules/.bin/codex exec x",
            "target/debug/deps/safe_chains-b21ccc14c83abc81 config add x",
            "target/debug/SAFE-CHAINS config add x",
            "target/debug/Safe-Chains mcp",
            "node_modules/.bin/CODEX exec x",
            "node_modules/@openai/codex/bin/codex.js exec x",
        ] {
            assert_eq!(crate::command_verdict(cmd), Verdict::Denied, "{cmd}");
        }
        for cmd in [
            "./cat src", "bin/git status", "./vendor/bin/phpunit --testsuite Unit", "scripts/run.sh", "./bin/cat foo",
            "target/debug/safe-chains config list", "scripts/install.sh", "./bin/Cat foo",
        ] {
            assert_eq!(crate::command_verdict(cmd), Verdict::Denied, "{cmd}");
        }
    }

    fn identity_spellings(name: &str) -> Vec<String> {
        let upper = name.to_ascii_uppercase();
        let capital = name[..1].to_ascii_uppercase() + &name[1..];
        let snake = name.replace('-', "_");
        vec![
            upper.clone(),
            capital.clone(),
            snake.clone(),
            format!("{snake}-0123456789abcdef"),
            format!("{name}-b21ccc14c83abc81"),
            format!("{name}.js"),
            format!("{capital}.EXE"),
            format!("{upper}-0123456789abcdef.cjs"),
            format!("{name}.js.map"),
        ]
    }

    #[test]
    fn every_spelling_of_an_agent_or_rule_changing_name_is_no_better_than_that_name() {
        let _ctx = crate::pathctx::enter(project());
        let names: Vec<&str> = crate::userconfig::AGENT_COMMANDS.iter().copied().chain(["safe-chains"]).collect();
        let mut checked = 0;
        for name in names {
            let bare = [crate::parse::Token::from_raw(name.to_string())];
            if !crate::handlers::names_a_command(&bare) {
                continue;
            }
            for spelling in identity_spellings(name) {
                for dir in ["target/debug/deps/", "node_modules/.bin/", "./"] {
                    for arg in [" exec x", " config add x", " mcp", " -p hi"] {
                        let path = format!("{dir}{spelling}");
                        assert_eq!(crate::command_verdict(&format!("{path}{arg}")), Verdict::Denied, "{path}{arg}");
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 100, "only {checked} spellings checked");
    }

    #[test]
    fn a_vcs_hook_is_refused_directly_and_through_a_shell() {
        for cmd in [".git/hooks/post-checkout", ".git/hooks/pre-commit"] {
            assert_eq!(crate::command_verdict(cmd), Verdict::Denied, "{cmd}");
            assert_eq!(crate::command_verdict(&format!("bash {cmd}")), Verdict::Denied, "bash {cmd}");
        }
    }

    #[test]
    fn a_shell_handed_a_packaged_script_is_refused_in_every_spelling() {
        for shell in ["bash", "sh", "zsh", "dash", "ksh"] {
            for form in ["", "-x ", "-o pipefail ", "+o errexit ", "-- "] {
                for script in ["node_modules/pkg/run.sh", "./node_modules/.bin/run", "sub/node_modules/x/y.sh"] {
                    let cmd = format!("{shell} {form}{script}");
                    assert_eq!(crate::command_verdict(&cmd), Verdict::Denied, "{cmd}");
                }
            }
        }
        assert_eq!(crate::command_verdict("bash scripts/run.sh"), Verdict::Allowed(SafetyLevel::SafeWrite));
    }

    #[test]
    fn a_bare_name_stays_a_path_lookup() {
        assert_eq!(crate::command_verdict("push_main_test.sh"), Verdict::Denied);
        assert_eq!(crate::command_verdict("bash push_main_test.sh"), Verdict::Allowed(SafetyLevel::SafeWrite));
    }

    #[test]
    fn arguments_do_not_make_a_script_path_a_command() {
        let direct = crate::command_verdict("./bin/check.sh --no-progress extra");
        let via = crate::command_verdict("bash ./bin/check.sh --no-progress extra");
        assert_eq!(direct, Verdict::Denied);
        assert_eq!(via, Verdict::Allowed(SafetyLevel::SafeWrite));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn a_direct_path_is_refused_on_every_level_and_target(
            path in prop::sample::select(corpus()),
            arg in prop::sample::select(ARGS.to_vec()),
        ) {
            assert_refused(path, arg);
        }

        #[test]
        fn a_decorated_identity_name_still_spells_that_name(
            name in prop::sample::select(crate::userconfig::AGENT_COMMANDS.iter().copied().chain(["safe-chains"]).collect::<Vec<_>>()),
            upper in prop::collection::vec(any::<bool>(), 16),
            snake in any::<bool>(),
            hash in prop::option::of("[0-9a-f]{16}"),
            extension in prop::option::of("[a-zA-Z]{1,4}"),
        ) {
            let mut decorated: String = name.chars().zip(upper.iter().cycle()).map(|(c, up)| if *up { c.to_ascii_uppercase() } else { c }).collect();
            if snake {
                decorated = decorated.replace('-', "_");
            }
            if let Some(hash) = hash {
                decorated = format!("{decorated}-{hash}");
            }
            if let Some(extension) = extension {
                decorated = format!("{decorated}.{extension}");
            }
            prop_assert!(super::spellings(&decorated).iter().any(|s| s == name), "{decorated} -> {:?}", super::spellings(&decorated));
        }

        #[test]
        fn an_extension_is_dropped_only_for_an_identity_name(stem in "[a-z]{1,8}", extension in "[a-z]{1,4}") {
            let spelled = super::spellings(&format!("{stem}.{extension}"));
            prop_assert_eq!(spelled.contains(&stem), super::is_an_identity(&stem));
        }
    }
}
