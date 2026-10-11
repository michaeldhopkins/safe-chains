use crate::parse::Token;
use crate::verdict::Verdict;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "bunx" => Some(super::runner_dispatch(tokens, &super::BUNX_FLAGS_NO_ARG)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocBuilder};
    vec![CommandDoc::handler(
        "bunx",
        "https://bun.sh/docs/cli/bunx",
        DocBuilder::new()
            .section("Delegates to the inner command's safety rules.")
            .section("Skips flags: --bun/--no-install/--package/-p.")
            .build(),
        "node",
    )]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        bunx_eslint: "bunx eslint src/",
        bunx_tsc_noemit: "bunx tsc --noEmit",
        bunx_tsc_project_noemit: "bunx tsc --project tsconfig.json --noEmit",
        bunx_bun_flag: "bunx --bun eslint src/",
        bunx_no_install_flag: "bunx --no-install eslint .",
        bunx_package_flag: "bunx --package eslint eslint src/",
        bunx_double_dash: "bunx -- eslint src/",
        bunx_version: "bunx --version",
        bunx_prettier_check: "bunx prettier --check src/",
        bunx_biome_check: "bunx biome check src/",
    }

    denied! {
        bunx_tsc_without_noemit_denied: "bunx tsc",
        bunx_tsc_with_other_flags_denied: "bunx tsc --pretty",
        bunx_cowsay_denied: "bunx cowsay hello",
        bare_bunx_denied: "bunx",
    }

    use crate::parse::Token;
    use crate::verdict::Verdict;
    use proptest::prelude::*;

    proptest::proptest! {
        /// Routing invariant: `dispatch` is the *only* function in this file that callers see, and
        /// its entire behaviour is "route `bunx` to `runner_dispatch` using BUNX_FLAGS_NO_ARG;
        /// return `None` for every other command name." Pinning both arms on every generated
        /// (cmd, tokens) pair keeps both halves of that contract honest.
        #[test]
        fn dispatch_only_routes_bunx(
            cmd in "[a-z][a-z0-9_-]{0,15}",
            tail in proptest::collection::vec(any::<String>(), 0..=8),
        ) {
            let tokens: Vec<Token> = tail.iter().map(|s| Token::from_test(s)).collect();
            if cmd == "bunx" {
                proptest::prop_assert_eq!(
                    super::dispatch(&cmd, &tokens),
                    Some(super::super::runner_dispatch(&tokens, &super::super::BUNX_FLAGS_NO_ARG)),
                    "bunx dispatch must wrap runner_dispatch with BUNX_FLAGS_NO_ARG",
                );
            } else {
                proptest::prop_assert_eq!(
                    super::dispatch(&cmd, &tokens),
                    None,
                    "non-bunx command `{}` must not dispatch",
                    cmd,
                );
            }
        }

        /// Denial invariant on `runner_dispatch`: every invocation that names no package is
        /// refused. Two cases contribute -- the input is too short to name a package, or every
        /// token after the program is a known no-arg flag with no `--version` shortcut. In both
        /// cases the function returns `Verdict::Denied`, because `find_runner_package_index` only
        /// ever returns `Some(idx)` for a non-flag token (or via the `--` sentinel, which we
        /// exclude here). The `prop_assume!` rules out `--version` so this property asserts the
        /// denial path; the version shortcut has its own dedicated handling.
        #[test]
        fn runner_dispatch_denies_when_no_package_is_named(
            picks in proptest::collection::vec(any::<proptest::sample::Index>(), 0..=6),
        ) {
            let flags: Vec<&'static str> = super::super::BUNX_FLAGS_NO_ARG.iter().collect();
            let mut tail: Vec<String> = picks
                .into_iter()
                .map(|i| flags[i.index(flags.len())].to_string())
                .collect();
            // Guarantee the invocation names no package and is not the version shortcut.
            proptest::prop_assume!(!tail.iter().any(|t| t == "--version"));
            // Program-name only (length 1) is also a "no package" case the function denies.
            tail.insert(0, "bunx".to_string());
            let tokens: Vec<Token> = tail.iter().map(|s| Token::from_test(s)).collect();
            proptest::prop_assert_eq!(
                super::super::runner_dispatch(&tokens, &super::super::BUNX_FLAGS_NO_ARG),
                Verdict::Denied,
                "no-package invocation must be denied, got tokens {:?}",
                tail,
            );
        }

        /// Sentinel invariant on `find_runner_package_index`: the `--` token marks "everything
        /// after is positional", so when it appears at `start` the function hands the caller the
        /// index immediately after. This holds regardless of which flags the runner knows and
        /// regardless of what follows the `--` (even an empty tail still yields `Some(start+1)`).
        #[test]
        fn find_runner_package_index_double_dash_yields_start_plus_one(
            start in 1usize..=3,
            tail in proptest::collection::vec(any::<String>(), 0..=6),
        ) {
            // The prefix before `start` is non-empty only to keep `start` a valid index; the
            // function does not inspect anything before `start`, so any junk there is fine.
            let mut raw: Vec<String> = vec!["bunx".to_string()];
            while raw.len() < start {
                raw.push("--bun".to_string());
            }
            raw.push("--".to_string());
            raw.extend(tail);
            let tokens: Vec<Token> = raw.iter().map(|s| Token::from_test(s)).collect();
            proptest::prop_assert_eq!(
                super::super::find_runner_package_index(&tokens, start, &super::super::BUNX_FLAGS_NO_ARG),
                Some(start + 1),
                "`--` at index {} must hand back start+1, got tokens {:?}",
                start,
                raw,
            );
        }
    }
}
