use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

static GRADLE_TASKS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--console", "--info", "--no-rebuild",
        "--quiet", "--stacktrace", "--warning-mode",
    ]),
    standalone_short: b"q",
    valued: WordSet::new(&["--group"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GRADLE_DEPS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--console", "--info", "--no-rebuild",
        "--quiet", "--stacktrace", "--warning-mode",
    ]),
    standalone_short: b"q",
    valued: WordSet::new(&["--configuration"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GRADLE_PROPS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--console", "--info", "--no-rebuild",
        "--quiet", "--stacktrace", "--warning-mode",
    ]),
    standalone_short: b"q",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GRADLE_BUILD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--build-cache", "--configure-on-demand", "--console",
        "--continue", "--dry-run", "--info", "--no-build-cache",
        "--no-daemon", "--no-parallel", "--no-rebuild",
        "--parallel", "--profile", "--quiet", "--rerun-tasks",
        "--scan", "--stacktrace", "--warning-mode",
    ]),
    standalone_short: b"q",
    valued: WordSet::new(&[
        "--exclude-task", "--max-workers",
    ]),
    valued_short: b"x",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_gradle(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let policy = match tokens[1].as_str() {
        "tasks" => &GRADLE_TASKS_POLICY,
        "dependencies" => &GRADLE_DEPS_POLICY,
        "properties" => &GRADLE_PROPS_POLICY,
        "build" | "check" | "test" => &GRADLE_BUILD_POLICY,
        _ => return false,
    };
    policy::check(&tokens[1..], policy)
}

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

pub fn is_safe_mvn(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if !MVN_SAFE_PHASES.contains(&tokens[1]) {
        return false;
    }
    if tokens[1] == "--version" || tokens[1] == "-v" {
        return tokens.len() == 2;
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
        return false;
    }
    true
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "gradle" | "gradlew" => Some(is_safe_gradle(tokens)),
        "mvn" | "mvnw" => Some(is_safe_mvn(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("gradle / gradlew",
            "Subcommands: build, check, dependencies, properties, tasks, test. \
            "),
        CommandDoc::handler("mvn / mvnw",
            "Phases: compile, dependency:list, dependency:tree, help:describe, \
             test, test-compile, validate, verify."),
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        gradle_tasks: "gradle tasks",
        gradle_tasks_all: "gradle tasks --all",
        gradle_dependencies: "gradle dependencies",
        gradle_dependencies_config: "gradle dependencies --configuration implementation",
        gradle_properties: "gradle properties",
        gradle_version: "gradle --version",
        gradle_test: "gradle test",
        gradle_test_info: "gradle test --info",
        gradle_test_stacktrace: "gradle test --stacktrace",
        gradle_build: "gradle build",
        gradle_build_parallel: "gradle build --parallel",
        gradle_build_scan: "gradle build --scan",
        gradle_check: "gradle check",
        gradlew_test: "gradlew test",
        gradlew_version: "gradlew --version",
        gradlew_build_info: "gradlew build --info",
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
        gradle_clean_denied: "gradle clean",
        gradle_publish_denied: "gradle publish",
        gradle_run_denied: "gradle run",
        bare_gradle_denied: "gradle",
        gradle_build_unknown_denied: "gradle build --unknown",
        gradle_tasks_unknown_denied: "gradle tasks --unknown",
        mvn_deploy_denied: "mvn deploy",
        mvn_install_denied: "mvn install",
        mvn_clean_denied: "mvn clean",
        bare_mvn_denied: "mvn",
        mvn_test_unknown_denied: "mvn test --unknown-flag",
        mvn_compile_unknown_denied: "mvn compile --unknown",
    }
}
