use crate::parse::{Token, WordSet};

static SWIFT_SAFE: WordSet =
    WordSet::new(&["--version", "build", "test"]);

static SWIFT_PACKAGE_SAFE: WordSet =
    WordSet::new(&["describe", "dump-package", "show-dependencies"]);

pub fn is_safe_swift(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if SWIFT_SAFE.contains(&tokens[1]) {
        return true;
    }
    if tokens[1] == "package" {
        return tokens
            .get(2)
            .is_some_and(|a| SWIFT_PACKAGE_SAFE.contains(a));
    }
    false
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, doc};
    vec![CommandDoc::handler("swift",
        doc(&SWIFT_SAFE)
            .multi_word(&[("package", SWIFT_PACKAGE_SAFE)])
            .build())]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        swift_version: "swift --version",
        swift_test: "swift test",
        swift_build: "swift build",
        swift_package_describe: "swift package describe",
        swift_package_dump_package: "swift package dump-package",
        swift_package_show_dependencies: "swift package show-dependencies",
    }

    denied! {
        swift_run_denied: "swift run",
        swift_package_init_denied: "swift package init",
        swift_package_update_denied: "swift package update",
        swift_package_resolve_denied: "swift package resolve",
        bare_swift_denied: "swift",
    }
}
