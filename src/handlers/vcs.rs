use std::collections::HashSet;
use std::sync::LazyLock;

static GIT_READ_ONLY: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "log",
        "diff",
        "show",
        "status",
        "ls-tree",
        "grep",
        "rev-parse",
        "merge-base",
        "merge-tree",
        "fetch",
        "help",
        "--version",
        "shortlog",
        "describe",
        "blame",
        "reflog",
    ])
});

static GIT_REMOTE_MUTATING: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "add",
        "remove",
        "rename",
        "set-url",
        "set-branches",
        "prune",
    ])
});

static GIT_BRANCH_MUTATING: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "-d",
        "-D",
        "--delete",
        "-m",
        "-M",
        "--move",
        "-c",
        "-C",
        "--copy",
        "-u",
        "--set-upstream-to",
        "--unset-upstream",
        "--edit-description",
    ])
});

static JJ_READ_ONLY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["log", "diff", "show", "status", "st", "help", "--version"]));

static JJ_MULTI: LazyLock<Vec<(&'static str, HashSet<&'static str>)>> = LazyLock::new(|| {
    vec![
        ("op", HashSet::from(["log"])),
        ("file", HashSet::from(["show"])),
        ("config", HashSet::from(["get"])),
    ]
});

pub fn is_safe_git(tokens: &[String]) -> bool {
    let mut args = &tokens[1..];
    while args.len() >= 2 && args[0] == "-C" {
        args = &args[2..];
    }
    if args.is_empty() {
        return false;
    }
    let subcmd = args[0].as_str();
    if GIT_READ_ONLY.contains(subcmd) {
        return true;
    }
    if subcmd == "remote" {
        return args.get(1).is_none_or(|a| !GIT_REMOTE_MUTATING.contains(a.as_str()));
    }
    if subcmd == "branch" {
        return args[1..].iter().all(|a| {
            !GIT_BRANCH_MUTATING.contains(a.as_str())
                && !GIT_BRANCH_MUTATING
                    .iter()
                    .any(|f| f.starts_with("--") && a.starts_with(&format!("{f}=")))
        });
    }
    false
}

pub fn is_safe_jj(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let subcmd = tokens[1].as_str();
    if JJ_READ_ONLY.contains(subcmd) {
        return true;
    }
    for (prefix, actions) in JJ_MULTI.iter() {
        if subcmd == *prefix {
            return tokens.get(2).is_some_and(|a| actions.contains(a.as_str()));
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::is_safe;

    fn check(cmd: &str) -> bool {
        is_safe(cmd)
    }

    #[test]
    fn git_log() {
        assert!(check("git log --oneline -5"));
    }

    #[test]
    fn git_diff() {
        assert!(check("git diff --stat"));
    }

    #[test]
    fn git_show() {
        assert!(check("git show HEAD:some/file.rb"));
    }

    #[test]
    fn git_status() {
        assert!(check("git status --porcelain"));
    }

    #[test]
    fn git_fetch() {
        assert!(check("git fetch origin master"));
    }

    #[test]
    fn git_ls_tree() {
        assert!(check("git ls-tree HEAD"));
    }

    #[test]
    fn git_grep() {
        assert!(check("git grep pattern"));
    }

    #[test]
    fn git_rev_parse() {
        assert!(check("git rev-parse HEAD"));
    }

    #[test]
    fn git_merge_base() {
        assert!(check("git merge-base master HEAD"));
    }

    #[test]
    fn git_merge_tree() {
        assert!(check("git merge-tree HEAD~1 HEAD master"));
    }

    #[test]
    fn git_version() {
        assert!(check("git --version"));
    }

    #[test]
    fn git_help() {
        assert!(check("git help log"));
    }

    #[test]
    fn git_shortlog() {
        assert!(check("git shortlog -s"));
    }

    #[test]
    fn git_describe() {
        assert!(check("git describe --tags"));
    }

    #[test]
    fn git_blame() {
        assert!(check("git blame file.rb"));
    }

    #[test]
    fn git_reflog() {
        assert!(check("git reflog"));
    }

    #[test]
    fn git_c_flag() {
        assert!(check("git -C /some/repo diff --stat"));
    }

    #[test]
    fn git_c_nested() {
        assert!(check("git -C /some/repo -C nested log"));
    }

    #[test]
    fn git_remote_bare() {
        assert!(check("git remote"));
    }

    #[test]
    fn git_remote_v() {
        assert!(check("git remote -v"));
    }

    #[test]
    fn git_remote_get_url() {
        assert!(check("git remote get-url origin"));
    }

    #[test]
    fn git_remote_show() {
        assert!(check("git remote show origin"));
    }

    #[test]
    fn git_push_denied() {
        assert!(!check("git push origin main"));
    }

    #[test]
    fn git_reset_denied() {
        assert!(!check("git reset --hard HEAD~1"));
    }

    #[test]
    fn git_add_denied() {
        assert!(!check("git add ."));
    }

    #[test]
    fn git_commit_denied() {
        assert!(!check("git commit -m 'test'"));
    }

    #[test]
    fn git_checkout_denied() {
        assert!(!check("git checkout -- file.rb"));
    }

    #[test]
    fn git_rebase_denied() {
        assert!(!check("git rebase origin/master"));
    }

    #[test]
    fn git_stash_denied() {
        assert!(!check("git stash"));
    }

    #[test]
    fn git_branch_list() {
        assert!(check("git branch"));
    }

    #[test]
    fn git_branch_list_all() {
        assert!(check("git branch -a"));
    }

    #[test]
    fn git_branch_list_verbose() {
        assert!(check("git branch -v"));
    }

    #[test]
    fn git_branch_contains() {
        assert!(check("git branch --contains abc123"));
    }

    #[test]
    fn git_branch_d_denied() {
        assert!(!check("git branch -D feature"));
    }

    #[test]
    fn git_branch_delete_denied() {
        assert!(!check("git branch --delete feature"));
    }

    #[test]
    fn git_branch_move_denied() {
        assert!(!check("git branch -m old new"));
    }

    #[test]
    fn git_branch_copy_denied() {
        assert!(!check("git branch -c old new"));
    }

    #[test]
    fn git_branch_set_upstream_denied() {
        assert!(!check("git branch --set-upstream-to=origin/main"));
    }

    #[test]
    fn git_rm_denied() {
        assert!(!check("git rm file.rb"));
    }

    #[test]
    fn git_remote_add_denied() {
        assert!(!check(
            "git remote add upstream https://github.com/foo/bar"
        ));
    }

    #[test]
    fn git_remote_remove_denied() {
        assert!(!check("git remote remove upstream"));
    }

    #[test]
    fn git_remote_rename_denied() {
        assert!(!check("git remote rename origin upstream"));
    }

    #[test]
    fn git_config_flag_denied() {
        assert!(!check("git -c user.name=foo log"));
    }

    #[test]
    fn bare_git_denied() {
        assert!(!check("git"));
    }

    #[test]
    fn jj_log() {
        assert!(check("jj log"));
    }

    #[test]
    fn jj_diff() {
        assert!(check("jj diff --stat"));
    }

    #[test]
    fn jj_show() {
        assert!(check("jj show abc123"));
    }

    #[test]
    fn jj_status() {
        assert!(check("jj status"));
    }

    #[test]
    fn jj_st() {
        assert!(check("jj st"));
    }

    #[test]
    fn jj_help() {
        assert!(check("jj help"));
    }

    #[test]
    fn jj_version() {
        assert!(check("jj --version"));
    }

    #[test]
    fn jj_op_log() {
        assert!(check("jj op log"));
    }

    #[test]
    fn jj_file_show() {
        assert!(check("jj file show some/path"));
    }

    #[test]
    fn jj_config_get() {
        assert!(check("jj config get user.name"));
    }

    #[test]
    fn jj_new_denied() {
        assert!(!check("jj new master"));
    }

    #[test]
    fn jj_edit_denied() {
        assert!(!check("jj edit abc123"));
    }

    #[test]
    fn jj_squash_denied() {
        assert!(!check("jj squash"));
    }

    #[test]
    fn jj_describe_denied() {
        assert!(!check("jj describe -m 'test'"));
    }

    #[test]
    fn jj_bookmark_denied() {
        assert!(!check("jj bookmark set my-branch"));
    }

    #[test]
    fn jj_git_push_denied() {
        assert!(!check("jj git push"));
    }

    #[test]
    fn jj_git_fetch_denied() {
        assert!(!check("jj git fetch"));
    }

    #[test]
    fn jj_rebase_denied() {
        assert!(!check("jj rebase -d master"));
    }

    #[test]
    fn jj_restore_denied() {
        assert!(!check("jj restore file.rb"));
    }

    #[test]
    fn jj_abandon_denied() {
        assert!(!check("jj abandon"));
    }

    #[test]
    fn jj_config_set_denied() {
        assert!(!check("jj config set user.name foo"));
    }

    #[test]
    fn bare_jj_denied() {
        assert!(!check("jj"));
    }
}
