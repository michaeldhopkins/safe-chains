//! `mise use`, `mise set` and `mise unset` pass with `-g` or `--global`, which writes the global
//! config instead of the project's, while a variable that changes what later shells run stays
//! refused there as everywhere.

fn approved(command: &str) -> bool {
    safe_chains::is_safe_command(command)
}

#[test]
fn mise_use_passes_with_global() {
    for command in ["mise use -g node@20", "mise use --global node@20", "mise u -g node@20", "mise use -gf node@20"] {
        assert!(approved(command), "{command} must pass");
    }
}

#[test]
fn mise_set_passes_with_global() {
    for command in ["mise set -g FOO=bar", "mise set --global FOO=bar", "mise set -g NODE_ENV=production"] {
        assert!(approved(command), "{command} must pass");
    }
}

#[test]
fn mise_unset_passes_with_global() {
    for command in ["mise unset -g FOO", "mise unset --global FOO", "mise unset FOO"] {
        assert!(approved(command), "{command} must pass");
    }
}

#[test]
fn global_is_judged_as_the_project_file_is() {
    for assignment in [
        "FOO=bar", "RUSTC_WRAPPER=./w", "RUSTC_WRAPPER=/tmp/w", "LD_PRELOAD=/tmp/x.so", "PATH=/tmp/x", "BASH_ENV=./x.sh",
        "FOO='{{ exec(command=\"id\") }}'", "_.source=./x.sh",
    ] {
        let local = approved(&format!("mise set {assignment}"));
        assert_eq!(approved(&format!("mise set -g {assignment}")), local, "-g changes the verdict of {assignment}");
        assert_eq!(approved(&format!("mise set --global {assignment}")), local, "--global changes the verdict of {assignment}");
    }
}

#[test]
fn global_listing_the_values_is_refused() {
    for command in ["mise set -g", "mise set --global", "mise set -g OPENAI_API_KEY", "mise set -g LD_PRELOAD=/tmp/x.so"] {
        assert!(!approved(command), "{command} must not pass");
    }
}

#[test]
fn environment_names_are_letters_only() {
    for command in ["mise set -E staging FOO=bar", "mise use -E production node@20", "mise use -e sandbox node@20", "mise set --env=dev FOO=bar"] {
        assert!(approved(command), "{command} must pass");
    }
    for command in [
        "mise set -E staging1 FOO=bar", "mise set -E ../x FOO=bar", "mise use -E pro_d node@20", "mise set --env=de.v FOO=bar",
        "mise use -E 'a b' node@20",
    ] {
        assert!(!approved(command), "{command} must not pass");
    }
}
