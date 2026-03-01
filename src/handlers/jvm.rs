use crate::parse::{Token, WordSet};

static GRADLE_SAFE: WordSet = WordSet::new(&[
    "--version", "build", "check", "dependencies", "properties", "tasks", "test",
]);

static MVN_SAFE: WordSet = WordSet::new(&[
    "--version", "-v", "compile", "dependency:list", "dependency:tree",
    "help:describe", "test", "test-compile", "validate", "verify",
]);

pub fn is_safe_gradle(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && GRADLE_SAFE.contains(&tokens[1])
}

pub fn is_safe_mvn(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && MVN_SAFE.contains(&tokens[1])
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::wordset("gradle / gradlew", &GRADLE_SAFE),
        CommandDoc::wordset("mvn / mvnw", &MVN_SAFE),
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
        gradle_dependencies: "gradle dependencies",
        gradle_properties: "gradle properties",
        gradle_version: "gradle --version",
        gradle_test: "gradle test",
        gradle_build: "gradle build",
        gradle_check: "gradle check",
        gradlew_test: "gradlew test",
        gradlew_version: "gradlew --version",
        mvn_version: "mvn --version",
        mvn_version_short: "mvn -v",
        mvn_dependency_tree: "mvn dependency:tree",
        mvn_dependency_list: "mvn dependency:list",
        mvn_help_describe: "mvn help:describe -Dplugin=compiler",
        mvn_validate: "mvn validate",
        mvn_test: "mvn test",
        mvn_compile: "mvn compile",
        mvn_verify: "mvn verify",
        mvn_test_compile: "mvn test-compile",
        mvnw_test: "mvnw test",
    }

    denied! {
        gradle_clean_denied: "gradle clean",
        gradle_publish_denied: "gradle publish",
        gradle_run_denied: "gradle run",
        bare_gradle_denied: "gradle",
        mvn_deploy_denied: "mvn deploy",
        mvn_install_denied: "mvn install",
        mvn_clean_denied: "mvn clean",
        bare_mvn_denied: "mvn",
    }
}
