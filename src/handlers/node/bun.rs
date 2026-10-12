use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

pub fn check_bun_x(tokens: &[Token]) -> Verdict {
    if tokens.len() == 2 && (tokens[1] == "--help" || tokens[1] == "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    match super::find_runner_package_index(tokens, 1, &super::BUNX_FLAGS_NO_ARG) {
        Some(idx) => super::runner_verdict(tokens, idx),
        None => Verdict::Denied,
    }
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        bun_version: "bun --version",
        bun_help: "bun --help",
        bun_build_entrypoint: "bun build ./src/index.ts",
        bun_build_outfile: "bun build --outfile=bundle.js ./src/index.ts",
        bun_build_outdir: "bun build --outdir=dist ./src/index.ts",
        bun_build_minify: "bun build --minify --splitting --outdir=out ./index.jsx",
        bun_build_production: "bun build --production --outdir=dist ./src/index.ts",
        bun_build_compile: "bun build --compile --outfile=my-app ./cli.ts",
        bun_build_sourcemap: "bun build --sourcemap=linked --outdir=dist ./src/index.ts",
        bun_build_target: "bun build --target=bun --outfile=server.js ./server.ts",
        bun_build_format: "bun build --format=cjs --outdir=dist ./src/index.ts",
        bun_build_external: "bun build --external react --outdir=dist ./src/index.ts",
        bun_build_no_bundle: "bun build --no-bundle ./src/index.ts",
        bun_build_watch: "bun build --watch --outdir=dist ./src/index.ts",
        bun_build_help: "bun build --help",
        bun_test: "bun test",
        bun_test_bail: "bun test --bail",
        bun_test_timeout: "bun test --timeout 5000",
        bun_pm_ls: "bun pm ls",
        bun_pm_hash: "bun pm hash",
        bun_pm_cache: "bun pm cache",
        bun_pm_bin: "bun pm bin",
        bun_outdated: "bun outdated",
        bun_x_eslint: "bun x eslint src/",
        bun_x_tsc_noemit: "bun x tsc --noEmit",
    }

    denied! {
        bun_build_bare: "bun build",
        bun_build_unknown_flag: "bun build --some-unknown ./src/index.ts",
        bun_x_tsc_denied: "bun x tsc",
        bun_x_cowsay_denied: "bun x cowsay hello",
    }

    use proptest::prelude::*;

    use crate::parse::Token;
    use crate::verdict::{SafetyLevel, Verdict};

    /// Tagged helper for `flag_only_tail_after_program_with_no_help_shortcut_is_denied`. Each
    /// `TailPiece` is either a bare known no-arg flag (consumed one-at-a-time by
    /// `find_runner_package_index`) or a `--package` / `-p` plus its value (consumed two-at-a-
    /// time). The enum lets the proptest strategy stay type-homogeneous across both shapes.
    #[derive(Debug, Clone)]
    enum TailPiece {
        Bare(String),
        Valued(String, String),
    }

    proptest::proptest! {
        /// INVARIANT (help-shortcut precedence). The first branch of `check_bun_x` matches on
        /// `tokens.len() == 2` together with `tokens[1] in {"--help", "-h"}` — it never inspects
        /// `tokens[0]`. So no matter what string the program-token holds (junk, "bun", an empty
        /// string, anything), the two-token help/h invocation must come back as `Allowed(Inert)`.
        /// Pinning that here keeps the shortcut independent of the leading token.
        #[test]
        fn help_shortcut_yields_inert_for_any_program_token_at_length_two(
            program in any::<String>(),
            help_arg in proptest::sample::select(&["--help", "-h"]),
        ) {
            let tokens = vec![Token::from_test(&program), Token::from_test(help_arg)];
            proptest::prop_assert_eq!(
                super::check_bun_x(&tokens),
                Verdict::Allowed(SafetyLevel::Inert),
                "help shortcut must return Allowed(Inert) for tokens[0]={:?}, tokens[1]={:?}",
                program,
                help_arg,
            );
        }

        /// INVARIANT (short-input denial). The help shortcut requires `tokens.len() == 2`, and
        /// `find_runner_package_index` only ever returns `Some(_)` for a non-flag positional it
        /// sees after `start = 1`. A tokens slice shorter than two therefore falls straight
        /// through the if-let to `Verdict::Denied`. The function's behavior on the 0- and 1-length
        /// inputs is fully pinned by this property.
        #[test]
        fn tokens_shorter_than_two_are_always_denied(
            tokens in proptest::collection::vec(any::<String>(), 0..=1),
        ) {
            let tokens: Vec<Token> = tokens.iter().map(|s| Token::from_test(s)).collect();
            proptest::prop_assert!(
                tokens.len() <= 1,
                "test bug: strategy must produce 0- or 1-element slices"
            );
            proptest::prop_assert_eq!(
                super::check_bun_x(&tokens),
                Verdict::Denied,
                "check_bun_x must deny a {} -element input",
                tokens.len(),
            );
        }

        /// INVARIANT (no-package denial on a flag-only tail). When every token from index 1
        /// onwards is one of `--package X`, `-p X`, or an entry in `BUNX_FLAGS_NO_ARG`, and the
        /// help shortcut does not match, `find_runner_package_index` consumes them all without
        /// ever seeing a positional package and returns `None`. The whole call therefore resolves
        /// to `Verdict::Denied`. The strategy picks the program name, then keeps appending either
        /// a known no-arg flag (consumes 1 token) or one of `--package` / `-p` followed by an
        /// arbitrary value (consumes 2 tokens); together they form the exact set the index walker
        /// accepts without ever returning `Some(_)`, so the property pins the "no positional =>
        /// Denied" half of the function. Each tail piece is encoded as a tagged enum so the
        /// proptest strategy stays type-clean.
        #[test]
        fn flag_only_tail_after_program_with_no_help_shortcut_is_denied(
            program in "[A-Za-z_][A-Za-z0-9_]{0,8}",
            tail_pieces in proptest::collection::vec(
                proptest::prop_oneof![
                    // Single known no-arg flag from BUNX_FLAGS_NO_ARG. `vec![...]` is used in
                    // preference to an array literal because `proptest::sample::select` requires
                    // its argument to live for `'static`, and an owned `Vec<TailPiece>` satisfies
                    // that via `Cow::Owned`.
                    proptest::sample::select(vec![
                        TailPiece::Bare("--bun".to_string()),
                        TailPiece::Bare("--no-install".to_string()),
                        TailPiece::Bare("--silent".to_string()),
                        TailPiece::Bare("--verbose".to_string()),
                    ]),
                    // `--package X` or `-p X`, where X is any free-form value.
                    proptest::prop_oneof![
                        Just("--package".to_string()),
                        Just("-p".to_string()),
                    ].prop_flat_map(|flag| (Just(flag), any::<String>())
                        .prop_map(|(f, v)| TailPiece::Valued(f, v))
                    ),
                ],
                1..=5,
            ),
        ) {
            // Build tokens from the tagged pieces. `tokens[0]` is the program; everything from
            // index 1 onwards is exactly what `find_runner_package_index` consumes without ever
            // returning `Some(_)`, so the walker must return `None` and the function must deny.
            let mut tokens: Vec<Token> = vec![Token::from_test(&program)];
            for piece in &tail_pieces {
                match piece {
                    TailPiece::Bare(s) => tokens.push(Token::from_test(s)),
                    TailPiece::Valued(flag, value) => {
                        tokens.push(Token::from_test(flag));
                        tokens.push(Token::from_test(value));
                    }
                }
            }
            // tokens[1] is a known no-arg flag, never "--help" or "-h" — the help shortcut
            // therefore cannot fire, and the assertion below targets the runner-dispatch path
            // exclusively. The `prop_assume!`s are belt-and-braces in case the strategy ever
            // drifts: pin the property's premise locally before evaluating it.
            proptest::prop_assume!(tokens[1].as_str() != "--help");
            proptest::prop_assume!(tokens[1].as_str() != "-h");
            proptest::prop_assert_eq!(
                super::check_bun_x(&tokens),
                Verdict::Denied,
                "flag-only tail with no package must be denied, got tokens {:?}",
                tokens.iter().map(Token::as_str).collect::<Vec<_>>(),
            );
        }
    }
}
