use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static MVN_STANDALONE: WordSet = WordSet::new(&[
    "--also-make", "--also-make-dependents", "--batch-mode",
    "--debug", "--errors", "--fail-at-end",
    "--fail-fast", "--fail-never", "--no-transfer-progress",
    "--offline", "--quiet", "--show-version",
    "--strict-checksums", "--update-snapshots",
    "-B", "-U", "-X", "-e", "-o", "-q",
]);

static MVN_VALUED: WordSet = WordSet::new(&[
    "--activate-profiles", "--projects", "--threads",
    "-P", "-T",
]);

static MVN_PROJECT_FILES: WordSet = WordSet::new(&["--file", "-f", "-s"]);

static MVN_LOG_FILES: WordSet = WordSet::new(&["--log-file", "-l"]);

static MVN_DEFINES: WordSet = WordSet::new(&["--define", "-D"]);

static MVN_PROPERTIES: WordSet = WordSet::new(&[
    "cmd", "detail", "excludedGroups", "excludes", "failIfNoTests",
    "goal", "groups", "includes", "it.test", "maven.compiler.release",
    "maven.compiler.source", "maven.compiler.target", "maven.javadoc.skip",
    "maven.main.skip", "maven.test.failure.ignore", "maven.test.skip",
    "medium", "minimal", "plugin", "scope", "skipITs", "skipTests",
    "style.color", "surefire.failIfNoSpecifiedTests", "test", "verbose",
]);

static MVN_SAFE_PHASES: WordSet = WordSet::new(&[
    "--version", "-v", "compile", "dependency:list", "dependency:tree",
    "help:describe", "test", "test-compile", "validate", "verify",
]);

fn phase_level(phase: &str) -> Option<SafetyLevel> {
    match phase {
        "compile" | "test-compile" => Some(SafetyLevel::SafeWrite),
        "test" | "validate" | "verify" => Some(SafetyLevel::SafeRead),
        "dependency:list" | "dependency:tree" | "help:describe" => Some(SafetyLevel::Inert),
        _ => None,
    }
}

fn property_verdict(definition: &str) -> Verdict {
    let name = definition.split_once('=').map_or(definition, |(name, _)| name);
    if MVN_PROPERTIES.contains(name) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }
}

fn valued_verdict(flag: &str, value: &str) -> Option<Verdict> {
    if MVN_VALUED.contains(flag) {
        return Some(Verdict::Allowed(SafetyLevel::Inert));
    }
    if MVN_DEFINES.contains(flag) {
        return Some(property_verdict(value));
    }
    if MVN_PROJECT_FILES.contains(flag) {
        return Some(match crate::engine::resolve::execute_file_verdict(value) {
            Verdict::Denied => Verdict::Denied,
            Verdict::Allowed(_) => Verdict::Allowed(SafetyLevel::Inert),
        });
    }
    if MVN_LOG_FILES.contains(flag) {
        if names_build_input(value) {
            return Some(Verdict::Denied);
        }
        return Some(crate::engine::resolve::write_target_verdict(value));
    }
    None
}

/// A log written over the build itself: a `pom.xml`, or anything under `.mvn/` (`jvm.config`,
/// `maven.config`, `extensions.xml`), which the next build reads as its code and options.
fn names_build_input(path: &str) -> bool {
    let mut parts = path.split('/').filter(|p| !p.is_empty());
    parts.clone().any(|p| p.eq_ignore_ascii_case(".mvn")) || parts.next_back().is_some_and(|p| p.eq_ignore_ascii_case("pom.xml"))
}

fn token_verdict(tokens: &[Token], i: usize) -> Option<(Verdict, usize)> {
    let t = tokens[i].as_str();
    if !t.starts_with('-') {
        return phase_level(t).map(|level| (Verdict::Allowed(level), 1));
    }
    if MVN_STANDALONE.contains(t) {
        return Some((Verdict::Allowed(SafetyLevel::Inert), 1));
    }
    if let Some(definition) = t.strip_prefix("-D").filter(|d| !d.is_empty()) {
        return Some((property_verdict(definition), 1));
    }
    if let Some(value) = tokens.get(i + 1)
        && let Some(verdict) = valued_verdict(t, value.as_str())
    {
        return Some((verdict, 2));
    }
    let (flag, value) = t.split_once('=').filter(|(flag, _)| flag.starts_with("--"))?;
    valued_verdict(flag, value).map(|verdict| (verdict, 1))
}

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
    let mut verdict = Verdict::Allowed(SafetyLevel::Inert);
    let mut i = 1;
    while i < tokens.len() {
        let Some((judged, used)) = token_verdict(tokens, i) else {
            return Verdict::Denied;
        };
        verdict = verdict.combine(judged);
        if verdict == Verdict::Denied {
            return Verdict::Denied;
        }
        i += used;
    }
    verdict
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
             test, test-compile, validate, verify. \
             Properties (-D): cmd, detail, excludedGroups, excludes, failIfNoTests, goal, groups, \
             includes, it.test, maven.compiler.release, maven.compiler.source, \
             maven.compiler.target, maven.javadoc.skip, maven.main.skip, \
             maven.test.failure.ignore, maven.test.skip, medium, minimal, plugin, scope, skipITs, \
             skipTests, style.color, surefire.failIfNoSpecifiedTests, test, verbose.",
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
        mvn_two_phases: "mvn compile test",
        mvn_phase_then_listed_goal: "mvn test dependency:tree",
        mvn_project_pom: "mvn test -f sub/pom.xml",
        mvn_project_pom_equals: "mvn test --file=sub/pom.xml",
        mvn_project_settings: "mvn test -s .mvn/settings.xml",
        mvn_project_log: "mvn compile -l build.log",
        mvn_define_space: "mvn test -D skipTests",
        mvn_define_long_equals: "mvn test --define=test=FooTest",
        mvn_profiles_threads: "mvn verify -P ci -T 4 --projects core",
    }

    denied! {
        mvn_deploy_denied: "mvn deploy",
        mvn_install_denied: "mvn install",
        mvn_clean_denied: "mvn clean",
        bare_mvn_denied: "mvn",
        mvn_extra_plugin_goal_denied: "mvn test exec:exec",
        mvn_extra_plugin_goal_after_flag_denied: "mvn test -q exec:java",
        mvn_extra_unknown_phase_denied: "mvn test install",
        mvn_foreign_pom_denied: "mvn test -f /tmp/x/pom.xml",
        mvn_foreign_pom_long_denied: "mvn test --file /tmp/x/pom.xml",
        mvn_foreign_pom_equals_denied: "mvn test --file=/tmp/x/pom.xml",
        mvn_home_pom_denied: "mvn test -f ~/other/pom.xml",
        mvn_foreign_settings_denied: "mvn test -s /tmp/settings.xml",
        mvn_foreign_settings_glued_denied: "mvn test -s/tmp/settings.xml",
        mvn_settings_long_denied: "mvn test --settings /tmp/settings.xml",
        mvn_global_settings_denied: "mvn test -gs /tmp/settings.xml",
        mvn_global_settings_long_denied: "mvn test --global-settings /tmp/settings.xml",
        mvn_toolchains_denied: "mvn test -t /tmp/toolchains.xml",
        mvn_toolchains_long_denied: "mvn test --toolchains /tmp/toolchains.xml",
        mvn_foreign_log_denied: "mvn compile -l ~/x.log",
        mvn_foreign_log_long_denied: "mvn compile --log-file /etc/x.log",
        mvn_foreign_log_equals_denied: "mvn compile --log-file=~/.bashrc",
        mvn_log_into_git_denied: "mvn compile -l .git/hooks/pre-commit",
        mvn_log_over_pom_denied: "mvn compile -l pom.xml",
        mvn_log_over_module_pom_denied: "mvn compile --log-file=core/POM.xml",
        mvn_log_into_mvn_config_denied: "mvn compile -l .mvn/jvm.config",
        mvn_log_into_mvn_extensions_denied: "mvn compile -l ./sub/.mvn/extensions.xml",
        mvn_exec_executable_denied: "mvn test -Dexec.executable=/tmp/evil",
        mvn_exec_args_denied: "mvn test -Dexec.args=-c",
        mvn_repo_local_denied: "mvn test -Dmaven.repo.local=/tmp/repo",
        mvn_ext_class_path_denied: "mvn test -Dmaven.ext.class.path=/tmp/evil.jar",
        mvn_surefire_arg_line_denied: "mvn test -DargLine=-javaagent:/tmp/evil.jar",
        mvn_surefire_jvm_denied: "mvn test -Djvm=/tmp/java",
        mvn_compiler_executable_denied: "mvn compile -Dmaven.compiler.fork=true -Dmaven.compiler.executable=/tmp/javac",
        mvn_tree_output_file_denied: "mvn dependency:tree -DoutputFile=/tmp/tree.txt",
        mvn_define_space_denied: "mvn test -D exec.executable=/tmp/evil",
        mvn_define_long_denied: "mvn test --define exec.executable=/tmp/evil",
        mvn_define_long_equals_denied: "mvn test --define=maven.repo.local=/tmp/repo",
    }

    mod proptests {
        use proptest::prelude::*;

        use super::super::{is_safe_mvn, MVN_LOG_FILES, MVN_PROPERTIES, MVN_SAFE_PHASES, MVN_STANDALONE};
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
                    "-DskipTests", "-Dtest=FooTest", "-Dmaven.test.skip=true",
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

            #[test]
            fn an_operand_that_is_not_a_listed_phase_or_goal_denies(
                phase in proptest::sample::select(vec!["compile", "test", "verify", "dependency:tree"]),
                operand in "[a-z][a-z0-9:.-]{0,15}".prop_filter("must not be listed", |w| {
                    !MVN_SAFE_PHASES.contains(w.as_str())
                }),
            ) {
                prop_assert_eq!(is_safe_mvn(&tokens(&["mvn", phase, &operand])), Verdict::Denied);
                prop_assert_eq!(is_safe_mvn(&tokens(&["mvn", phase, "-q", &operand])), Verdict::Denied);
            }

            #[test]
            fn an_unlisted_property_denies_in_every_spelling(
                name in "[a-zA-Z][a-zA-Z0-9.]{0,20}".prop_filter("must not be listed", |n| {
                    !MVN_PROPERTIES.contains(n.as_str())
                }),
                value in "[a-zA-Z0-9/._-]{0,12}",
            ) {
                let definition = format!("{name}={value}");
                for words in [
                    vec!["mvn", "test", &*format!("-D{definition}")],
                    vec!["mvn", "test", "-D", &definition],
                    vec!["mvn", "test", "--define", &definition],
                    vec!["mvn", "test", &*format!("--define={definition}")],
                ] {
                    prop_assert_eq!(is_safe_mvn(&tokens(&words)), Verdict::Denied, "{:?}", words);
                }
            }

            #[test]
            fn a_file_flag_judges_its_value_the_same_in_every_spelling(
                flag in proptest::sample::select(vec!["--file", "-f", "-s", "--log-file", "-l"]),
                value in proptest::sample::select(vec![
                    "pom.xml", "sub/pom.xml", "/tmp/x/pom.xml", "~/x.xml", "/etc/x", "../x/pom.xml", ".git/hooks/x",
                ]),
            ) {
                let spaced = is_safe_mvn(&tokens(&["mvn", "test", flag, value]));
                if flag.starts_with("--") {
                    prop_assert_eq!(is_safe_mvn(&tokens(&["mvn", "test", &format!("{flag}={value}")])), spaced);
                }
                let judged = if MVN_LOG_FILES.contains(flag) && super::super::names_build_input(value) {
                    Verdict::Denied
                } else if MVN_LOG_FILES.contains(flag) {
                    crate::engine::resolve::write_target_verdict(value)
                } else {
                    crate::engine::resolve::execute_file_verdict(value)
                };
                prop_assert_eq!(spaced == Verdict::Denied, judged == Verdict::Denied);
            }
        }
    }
}
