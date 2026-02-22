use std::collections::HashSet;
use std::sync::LazyLock;

use crate::parse::Token;

static GRADLE_SAFE: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "tasks",
        "dependencies",
        "properties",
        "--version",
        "test",
        "build",
        "check",
    ])
});

static MVN_SAFE: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "--version",
        "-v",
        "dependency:tree",
        "dependency:list",
        "help:describe",
        "validate",
        "test",
        "compile",
        "verify",
        "test-compile",
    ])
});

pub fn is_safe_gradle(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && GRADLE_SAFE.contains(tokens[1].as_str())
}

pub fn is_safe_mvn(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && MVN_SAFE.contains(tokens[1].as_str())
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocKind};
    vec![
        CommandDoc {
            name: "gradle / gradlew",
            kind: DocKind::Handler,
            description: "Allowed: tasks, dependencies, properties, --version, test, build, check.",
        },
        CommandDoc {
            name: "mvn / mvnw",
            kind: DocKind::Handler,
            description: "Allowed: --version, -v, dependency:tree, dependency:list, help:describe, validate, test, compile, verify, test-compile.",
        },
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    #[test]
    fn gradle_tasks() {
        assert!(check("gradle tasks"));
    }

    #[test]
    fn gradle_dependencies() {
        assert!(check("gradle dependencies"));
    }

    #[test]
    fn gradle_properties() {
        assert!(check("gradle properties"));
    }

    #[test]
    fn gradle_version() {
        assert!(check("gradle --version"));
    }

    #[test]
    fn gradle_test() {
        assert!(check("gradle test"));
    }

    #[test]
    fn gradle_build() {
        assert!(check("gradle build"));
    }

    #[test]
    fn gradle_check() {
        assert!(check("gradle check"));
    }

    #[test]
    fn gradlew_test() {
        assert!(check("gradlew test"));
    }

    #[test]
    fn gradlew_version() {
        assert!(check("gradlew --version"));
    }

    #[test]
    fn gradle_clean_denied() {
        assert!(!check("gradle clean"));
    }

    #[test]
    fn gradle_publish_denied() {
        assert!(!check("gradle publish"));
    }

    #[test]
    fn gradle_run_denied() {
        assert!(!check("gradle run"));
    }

    #[test]
    fn bare_gradle_denied() {
        assert!(!check("gradle"));
    }

    #[test]
    fn mvn_version() {
        assert!(check("mvn --version"));
    }

    #[test]
    fn mvn_version_short() {
        assert!(check("mvn -v"));
    }

    #[test]
    fn mvn_dependency_tree() {
        assert!(check("mvn dependency:tree"));
    }

    #[test]
    fn mvn_dependency_list() {
        assert!(check("mvn dependency:list"));
    }

    #[test]
    fn mvn_help_describe() {
        assert!(check("mvn help:describe -Dplugin=compiler"));
    }

    #[test]
    fn mvn_validate() {
        assert!(check("mvn validate"));
    }

    #[test]
    fn mvn_test() {
        assert!(check("mvn test"));
    }

    #[test]
    fn mvn_compile() {
        assert!(check("mvn compile"));
    }

    #[test]
    fn mvn_verify() {
        assert!(check("mvn verify"));
    }

    #[test]
    fn mvn_test_compile() {
        assert!(check("mvn test-compile"));
    }

    #[test]
    fn mvnw_test() {
        assert!(check("mvnw test"));
    }

    #[test]
    fn mvn_deploy_denied() {
        assert!(!check("mvn deploy"));
    }

    #[test]
    fn mvn_install_denied() {
        assert!(!check("mvn install"));
    }

    #[test]
    fn mvn_clean_denied() {
        assert!(!check("mvn clean"));
    }

    #[test]
    fn bare_mvn_denied() {
        assert!(!check("mvn"));
    }
}
