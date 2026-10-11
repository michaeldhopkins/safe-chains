use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static MVN_STANDALONE: WordSet = WordSet::new(&[
    "--also-make", "--also-make-dependents", "--batch-mode",
    "--debug", "--errors", "--fail-at-end",
    "--fail-fast", "--fail-never", "--no-transfer-progress",
    "--offline", "--quiet", "--show-version",
    "--strict-checksums", "--update-snapshots",
    "-B", "-U", "-X", "-e", "-f", "-o", "-q",
]);

static MVN_VALUED: WordSet = WordSet::new(&[
    "--activate-profiles", "--define", "--file",
    "--log-file", "--projects", "--threads",
    "-D", "-P", "-T", "-f", "-l", "-s",
]);

static MVN_SAFE_PHASES: WordSet = WordSet::new(&[
    "--version", "-v", "compile", "dependency:list", "dependency:tree",
    "help:describe", "test", "test-compile", "validate", "verify",
]);

pub fn is_safe_mvn(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Denied;
    }
    if !MVN_SAFE_PHASES.contains(&tokens[1]) {
        return Verdict::Denied;
    }
    if tokens[1] == "--version" || tokens[1] == "-v" {
        return if tokens.len() == 2 { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }
    let mut i = 2;
    while i < tokens.len() {
        let t = &tokens[i];
        if !t.starts_with('-') {
            i += 1;
            continue;
        }
        if t.starts_with("-D") && t.len() > 2 {
            i += 1;
            continue;
        }
        if MVN_STANDALONE.contains(t) {
            i += 1;
            continue;
        }
        if MVN_VALUED.contains(t) {
            i += 2;
            continue;
        }
        if let Some((flag, _)) = t.as_str().split_once('=')
            && MVN_VALUED.contains(flag)
        {
            i += 1;
            continue;
        }
        return Verdict::Denied;
    }
    let level = match tokens[1].as_str() {
        "compile" | "test-compile" => SafetyLevel::SafeWrite,
        "test" | "validate" | "verify" => SafetyLevel::SafeRead,
        _ => SafetyLevel::Inert,
    };
    Verdict::Allowed(level)
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "mvn" | "mvnw" => Some(is_safe_mvn(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("mvn / mvnw",
            "https://maven.apache.org/ref/current/maven-embedder/cli.html",
            "Phases: compile, dependency:list, dependency:tree, help:describe, \
             test, test-compile, validate, verify.",
            "jvm"),
    ]
}

#[cfg(test)]
pub(in crate::handlers::jvm) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "mvn", valid_prefix: Some("mvn test") },
    crate::handlers::CommandEntry::Custom { cmd: "mvnw", valid_prefix: Some("mvnw test") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        mvn_version: "mvn --version",
        mvn_version_short: "mvn -v",
        mvn_dependency_tree: "mvn dependency:tree",
        mvn_dependency_tree_offline: "mvn dependency:tree --offline",
        mvn_dependency_list: "mvn dependency:list",
        mvn_help_describe: "mvn help:describe -Dplugin=compiler",
        mvn_validate: "mvn validate",
        mvn_test: "mvn test",
        mvn_test_define: "mvn test -Dtest=MyTest",
        mvn_test_batch: "mvn test --batch-mode",
        mvn_compile: "mvn compile",
        mvn_verify: "mvn verify",
        mvn_test_compile: "mvn test-compile",
        mvnw_test: "mvnw test",
        mvnw_version: "mvnw --version",
    }

    denied! {
        mvn_deploy_denied: "mvn deploy",
        mvn_install_denied: "mvn install",
        mvn_clean_denied: "mvn clean",
        bare_mvn_denied: "mvn",
    }

    mod proptests {
        use proptest::prelude::*;

        use super::super::{is_safe_mvn, MVN_SAFE_PHASES, MVN_STANDALONE};
        use crate::parse::Token;
        use crate::verdict::{SafetyLevel, Verdict};

        fn token(s: &str) -> Token {
            Token::from_test(s)
        }

        fn tokens(words: &[&str]) -> Vec<Token> {
            words.iter().map(|s| token(s)).collect()
        }

        fn any_token_string() -> impl Strategy<Value = String> {
            // Token strings must be non-empty; WordSet and matching use `&str` equality.
            // Allow any printable ASCII that won't trip up the parser, plus a dash so flags are
            // reachable as a generated case.
            "[A-Za-z0-9_.@:/+\\-]{1,16}"
        }

        /// The level `is_safe_mvn` assigns to an allowed command with this phase.
        fn expected_phase_level(phase: &str) -> SafetyLevel {
            match phase {
                "compile" | "test-compile" => SafetyLevel::SafeWrite,
                "test" | "validate" | "verify" => SafetyLevel::SafeRead,
                _ => SafetyLevel::Inert,
            }
        }

        proptest! {
            /// An input with fewer than two tokens never has a recognised phase in `tokens[1]`,
            /// so the function must refuse it — independent of what the lone token actually is.
            #[test]
            fn too_short_input_is_always_denied(first in any_token_string()) {
                let toks = tokens(&[&first]);
                prop_assert_eq!(is_safe_mvn(&toks), Verdict::Denied);

                let empty: Vec<Token> = vec![];
                prop_assert_eq!(is_safe_mvn(&empty), Verdict::Denied);
            }

            /// If the phase (tokens[1]) is not in the safe-phase whitelist, the verdict is
            /// `Denied` no matter what other flags or arguments follow it. The function does
            /// its flag scan only AFTER the phase check, so adding well-formed flags below an
            /// unsafe phase must NOT rescue it.
            #[test]
            fn unknown_phase_is_always_denied(
                phase in any_token_string().prop_filter("must not be a safe phase", |p| {
                    !MVN_SAFE_PHASES.contains(p.as_str())
                }),
                tail in proptest::collection::vec(any_token_string(), 0..=20),
            ) {
                let mut words: Vec<String> = Vec::with_capacity(2 + tail.len());
                words.push("mvn".to_string());
                words.push(phase);
                words.extend(tail);
                let refs: Vec<&str> = words.iter().map(String::as_str).collect();
                let toks = tokens(&refs);
                prop_assert_eq!(is_safe_mvn(&toks), Verdict::Denied);
            }

            /// Idempotence-style invariant: appending a single known-standalone safe flag to an
            /// already-allowed command must NOT change the verdict OR the assigned safety level.
            /// Standalone flags don't shift what a phase does, so the level must stay constant.
            /// Restrict the head flags to known-safe flag shapes so most generated inputs
            /// actually exercise the property (rather than being filtered as Denied).
            #[test]
            fn appending_a_standalone_flag_preserves_verdict_and_level(
                base in proptest::sample::select(vec![
                    "compile", "test-compile", "test", "validate", "verify",
                    "dependency:list", "dependency:tree", "help:describe",
                ]),
                head in proptest::collection::vec(proptest::sample::select(vec![
                    "-q", "-B", "-X", "--offline", "--quiet", "--errors",
                    "-DskipTests", "-Dfoo=bar", "-Dmaven.test.skip=true",
                ]), 0..=3),
                trailing_flag_idx in 0usize..MVN_STANDALONE.iter().count(),
            ) {
                let mut base_words: Vec<String> = vec!["mvn".to_string(), base.to_string()];
                base_words.extend(head.iter().map(|s| s.to_string()));
                let base_refs: Vec<&str> = base_words.iter().map(String::as_str).collect();
                let base_toks = tokens(&base_refs);

                // Compute the level the phase would yield if allowed; this is the only allowed
                // level any allowed result can have for that phase, regardless of head flags.
                let expected_level = expected_phase_level(base);

                // First: the well-formed base is itself allowed with the expected level.
                prop_assert_eq!(
                    is_safe_mvn(&base_toks),
                    Verdict::Allowed(expected_level),
                    "restricted head flags must keep base allowed at its phase level",
                );

                let standalone: Vec<&'static str> = MVN_STANDALONE.iter().collect();
                let flag = standalone[trailing_flag_idx % standalone.len()];

                // Now: appending a single standalone flag must not change verdict or level.
                let mut extended_words = base_words;
                extended_words.push(flag.to_string());
                let ext_refs: Vec<&str> = extended_words.iter().map(String::as_str).collect();
                let ext_toks = tokens(&ext_refs);

                prop_assert_eq!(
                    is_safe_mvn(&ext_toks),
                    Verdict::Allowed(expected_level),
                    "standalone flag {:?} must not change verdict or level",
                    flag,
                );
            }

            /// `mvn --version` and `mvn -v` are inert ONLY with exactly two tokens; any extra
            /// argument flips the verdict to `Denied`. This pins both directions of the rule.
            #[test]
            fn version_phase_is_inert_only_when_solitary(
                variant_idx in 0usize..2,
                extra in any_token_string().prop_filter("must be non-empty", |s| !s.is_empty()),
            ) {
                let flag = if variant_idx == 0 { "--version" } else { "-v" };

                // Solitary → allowed with Inert level.
                prop_assert_eq!(
                    is_safe_mvn(&tokens(&["mvn", flag])),
                    Verdict::Allowed(SafetyLevel::Inert),
                    "{:?} with no extra args must be Inert",
                    flag,
                );

                // With one extra token → Denied. The phase rule for `--version`/`-v` is exact.
                prop_assert_eq!(
                    is_safe_mvn(&tokens(&["mvn", flag, &extra])),
                    Verdict::Denied,
                    "{:?} with any extra arg must be Denied",
                    flag,
                );
            }
        }
    }
}
