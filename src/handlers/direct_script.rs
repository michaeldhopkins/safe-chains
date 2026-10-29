use std::borrow::Cow;

use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn is_relative_command_path(word: &str) -> bool {
    !word.is_empty() && !word.starts_with(['@', '/', '~']) && !word.contains('$') && word.contains('/')
}

/// Whether `arg0` is a trusted way to invoke a standard tool: a bare name (found via `$PATH`) or a
/// plain name directly inside a standard system bin directory. A path elsewhere (`./x`, `/tmp/x`,
/// `~/bin/x`, `/usr/bin/../../tmp/x`) may be an impostor.
pub(crate) fn trusted_command_path(arg0: &str) -> bool {
    const STD_BINS: &[&str] = &["/usr/bin", "/bin", "/usr/local/bin", "/opt/homebrew/bin", "/sbin", "/usr/sbin"];
    !arg0.contains('/')
        || arg0
            .rsplit_once('/')
            .is_some_and(|(dir, name)| STD_BINS.contains(&dir) && !matches!(name, "" | "." | ".."))
}

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
    let mut verdict = judge(&as_interpreter(tokens));
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
    let fold = cfg!(target_os = "macos");
    script.is_some_and(|path| {
        path.contains('/')
            && path.split('/').any(|part| {
                crate::engine::resolve::frozen_walk::is_frozen(part)
                    || if fold { part.eq_ignore_ascii_case("node_modules") } else { part == "node_modules" }
            })
    })
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

    fn spelled(path: &str) -> Option<String> {
        let name = path.rsplit('/').next().unwrap_or(path);
        let tokens = [crate::parse::Token::from_raw(name.to_string())];
        (super::is_relative_command_path(path) && crate::handlers::names_a_command(&tokens)).then(|| name.to_string())
    }

    fn assert_no_better_than_its_name(path: &str, arg: &str, name: &str) {
        let (direct, via) = pair(path, arg);
        let bare = format!("{name}{arg}");
        for level in crate::engine::authoring::default_levels() {
            let (ceiling, engine) = crate::level_ceiling(&level.name).expect("level");
            let d = crate::command_verdict_ceilinged(&direct, ceiling, engine);
            let b = crate::command_verdict_ceilinged(&via, ceiling, engine);
            let n = crate::command_verdict_ceilinged(&bare, ceiling, engine);
            assert_eq!(d, b.combine(n), "{direct} at {}", level.name);
        }
    }

    fn assert_same(path: &str, arg: &str) {
        let _ctx = crate::pathctx::enter(project());
        if let Some(name) = spelled(path) {
            return assert_no_better_than_its_name(path, arg, &name);
        }
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
            assert!(crate::command_verdict(cmd).is_allowed(), "{cmd}");
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
                        let direct = crate::command_verdict(&format!("{path}{arg}"));
                        let named = crate::command_verdict(&format!("{name}{arg}"));
                        let script = crate::command_verdict(&format!("bash {path}{arg}"));
                        assert_eq!(direct, script.combine(named), "{path}{arg}");
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 100, "only {checked} spellings checked");
    }

    #[test]
    fn a_vcs_hook_matches_bash() {
        let cmd = ".git/hooks/post-checkout";
        let verdict = crate::command_verdict(cmd);
        assert_eq!(verdict, crate::command_verdict(&format!("bash {cmd}")));
        assert_eq!(verdict, Verdict::Denied);
        let spelled = ".git/hooks/pre-commit";
        assert_eq!(
            crate::command_verdict(spelled),
            crate::command_verdict(&format!("bash {spelled}")).combine(crate::command_verdict("pre-commit"))
        );
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
