//! A mise setting that loads a file, a grep option spelled by any unambiguous prefix, and a script
//! run from installed packages or a frozen folder are judged as what they run or read.

use safe_chains::pathctx::{PathCtx, enter};

fn approved(command: &str) -> bool {
    let root = env!("CARGO_MANIFEST_DIR").to_string();
    let _ctx = enter(PathCtx { cwd: Some(root.clone()), root: Some(root), session_id: None });
    safe_chains::is_safe_command(command)
}

#[test]
fn a_mise_setting_that_loads_a_file_is_refused() {
    for command in ["mise set _.source=/tmp/evil.sh", "mise set _.file=x", "mise set _.path=/tmp", "mise set -g _.source=./x.sh"] {
        assert!(!approved(command), "`{command}` was approved");
    }
}

#[test]
fn a_long_token_of_three_characters_is_a_search_pattern_not_an_option_prefix() {
    for command in ["grep --d x README.md", "grep --f x README.md", "grep --x x README.md", "grep --- x README.md"] {
        assert!(approved(command), "`{command}` was refused");
    }
}

#[test]
fn every_prefix_of_a_grep_option_that_reads_a_file_or_follows_links_is_judged_as_the_option() {
    for option in ["--file=~/.aws/credentials", "--dereference-recursive"] {
        let (name, value) = option.split_once('=').map_or((option, ""), |(n, v)| (n, v));
        for end in 4..=name.len() {
            let prefix = &name[..end];
            let spelled = if value.is_empty() { prefix.to_string() } else { format!("{prefix}={value}") };
            if prefix == "--de" || prefix == "--d" {
                continue;
            }
            let command = format!("grep {spelled} x README.md");
            assert!(!approved(&command), "`{command}` was approved");
        }
    }
    for command in ["grep --deref '' .", "grep --dereference x .", "grep --fil=/etc/passwd x README.md"] {
        assert!(!approved(command), "`{command}` was approved");
    }
    for command in ["grep --count x README.md", "grep --ignore-case x README.md", "grep --fixed-strings x README.md"] {
        assert!(approved(command), "`{command}` should stay approved");
    }
}

#[test]
fn a_script_from_installed_packages_or_a_frozen_folder_is_refused() {
    for command in [
        "node_modules/.bin/x", "./node_modules/.bin/x", "bash node_modules/.bin/x", "./.envrc", ".git/HEAD", "bash .git/hooks/pre-commit",
        "sh ./.git/hooks/pre-commit", "./.jj/x",
    ] {
        assert!(!approved(command), "`{command}` was approved");
    }
}

#[test]
fn a_cd_into_installed_packages_or_a_frozen_folder_judges_what_runs_there() {
    for command in [
        "cd node_modules/pkg && bash a.sh",
        "cd node_modules/pkg && ./a.sh",
        "cd node_modules/pkg && sh ./a.sh",
        "cd node_modules && bash pkg/a.sh",
        "cd src && bash ../node_modules/pkg/a.sh",
        "cd node_modules/pkg && mvn validate",
        "cd node_modules/pkg && task -l",
        "cd node_modules/pkg && ansible-inventory --list",
        "cd node_modules/pkg && mise exec -- ls",
        "cd .git/hooks && mvn validate",
        "cd .git/hooks && task -l",
        "cd .git/hooks && ./pre-commit",
        "cd src && ./a.sh",
    ] {
        assert!(!approved(command), "`{command}` was approved");
    }
    for command in ["cd src && bash a.sh", "cd src && task -l", "bash src/a.sh"] {
        assert!(approved(command), "`{command}` should stay approved");
    }
}

#[test]
fn a_package_or_frozen_folder_spelled_with_a_folding_character_is_refused() {
    for command in [
        "bash node_module\u{17F}/pkg/a.sh", "sh node_module\u{17F}/pkg/a.sh", "./node_module\u{17F}/pkg/a.sh",
        "cd node_module\u{17F}/pkg && bash a.sh", "cd node_module\u{17F}/pkg && task -l", "bash .g\u{131}t/hooks/pre-commit",
        "cd .g\u{131}t/hooks && ./pre-commit",
    ] {
        assert!(!approved(command), "`{command}` was approved");
    }
    assert!(approved("bash r\u{E9}sum\u{E9}/a.sh"), "a name outside ASCII that cannot be node_modules stays approved");
}

proptest::proptest! {
    /// Every spelling of `node_modules` APFS opens as that folder, each letter in either case and its
    /// `s` as the long s that folds to it, is refused as the folder a script runs from.
    #[test]
    fn every_folded_spelling_of_node_modules_is_refused(upper in proptest::collection::vec(proptest::bool::ANY, 12)) {
        let name: String = "node_modules"
            .chars()
            .zip(upper)
            .map(|(c, up)| if c == 's' { '\u{17F}' } else if up { c.to_ascii_uppercase() } else { c })
            .collect();
        for command in [format!("bash {name}/pkg/a.sh"), format!("./{name}/pkg/a.sh"), format!("cd {name}/pkg && bash a.sh")] {
            proptest::prop_assert!(!approved(&command), "`{}` was approved", command);
        }
    }
}
