use proptest::prelude::*;

fn in_project<T>(run: impl FnOnce(&str) -> T) -> T {
    let dir = tempfile::tempdir().expect("scratch project");
    let root = dir.path().canonicalize().expect("canonical root").to_str().expect("utf-8 root").to_string();
    std::fs::create_dir_all(format!("{root}/.git/hooks")).expect("metadata");
    std::fs::create_dir_all(format!("{root}/sub/.git")).expect("nested metadata");
    std::fs::create_dir_all(format!("{root}/src")).expect("source");
    let _ctx = crate::pathctx::enter(crate::pathctx::PathCtx { cwd: Some(root.clone()), root: Some(root.clone()), session_id: None });
    run(&root)
}

#[test]
fn a_printed_path_is_judged_where_it_may_be_written() {
    in_project(|root| {
        std::os::unix::fs::symlink("/etc", format!("{root}/-etc")).expect("link");
        for line in [
            "echo /etc/hosts | xargs rm",
            "echo .git | xargs chmod 755",
            "echo .git/hooks/x | xargs touch",
            "echo . | xargs chmod -R 755",
            "echo src . | xargs chmod -R 755",
            "echo sub | xargs rm -r",
            "echo src sub | xargs chmod -R 755",
            "echo 'src /etc/hosts' | xargs touch",
            "echo \"'/etc/hosts'\" | xargs touch",
            "printf '%s%s' /et c/hosts | xargs touch",
            "printf '%s\\n' src /etc/hosts | xargs touch",
            "echo src/.gitx/hooks | xargs -d x touch",
            "ls -d src /etc/hosts | xargs touch",
            "echo -x/../.git | xargs rm -rf --",
            "printf -- -x/../.git | xargs rm -rf --",
            "ls -d -- -etc/hosts | xargs rm -f --",
            "ls -d | xargs chmod -R 755",
            "find / -name $(echo a -delete)",
            "find . -name $(echo \"a -delete\")",
            "find / -name $(printf '%s\\n' a -delete)",
            "git commit -m $(echo a --amend)",
            "for x in \"$(echo \"a -delete\")\"; do find / -name $x; done",
        ] {
            assert!(!crate::is_safe_command(line), "{line} was approved");
        }
        for line in [
            "echo src | xargs chmod 755", "echo -n src | xargs chmod 755 --", "echo src/a src/b | xargs touch",
            "echo /etc/hosts | xargs cat",
        ] {
            assert!(crate::is_safe_command(line), "{line} was refused");
        }
    });
}

const PIECES: [&str; 9] = ["src", "src/a", ".", "sub", ".git", ".git/hooks/x", "/etc/hosts", "~/.zshrc", "/tmp/x"];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    fn printed_items_are_written_only_where_each_may_be(picks in prop::collection::vec(0..PIECES.len(), 1..4), write in prop::sample::select(vec!["touch", "chmod -R 755", "rm -r"])) {
        in_project(|_| {
            let words: Vec<&str> = picks.iter().map(|i| PIECES[*i]).collect();
            let line = format!("echo {} | xargs {write}", words.join(" "));
            if crate::is_safe_command(&line) {
                for piece in &words {
                    let alone = format!("{write} {piece}");
                    assert!(crate::is_safe_command(&alone), "{line} was approved, {alone} is not");
                }
            }
        });
    }
}
