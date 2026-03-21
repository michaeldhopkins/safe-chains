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
             test, test-compile, validate, verify."),
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
}
