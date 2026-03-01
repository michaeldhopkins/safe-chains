use crate::parse::{Token, WordSet};

static COMPOSER_SAFE: WordSet = WordSet::new(&[
    "--version", "about", "audit", "check-platform-reqs", "diagnose",
    "fund", "help", "info", "licenses", "outdated", "show", "suggests",
]);

pub fn is_safe_composer(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && COMPOSER_SAFE.contains(&tokens[1])
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![CommandDoc::wordset("composer", &COMPOSER_SAFE)]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        composer_show: "composer show",
        composer_show_package: "composer show laravel/framework",
        composer_info: "composer info",
        composer_diagnose: "composer diagnose",
        composer_outdated: "composer outdated",
        composer_licenses: "composer licenses",
        composer_check_platform_reqs: "composer check-platform-reqs",
        composer_suggests: "composer suggests",
        composer_fund: "composer fund",
        composer_audit: "composer audit",
        composer_version: "composer --version",
        composer_about: "composer about",
        composer_help: "composer help",
    }

    denied! {
        composer_install_denied: "composer install",
        composer_update_denied: "composer update",
        composer_require_denied: "composer require laravel/framework",
        composer_remove_denied: "composer remove laravel/framework",
        composer_run_script_denied: "composer run-script test",
        bare_composer_denied: "composer",
    }
}
