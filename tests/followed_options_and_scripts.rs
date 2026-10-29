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
