use crate::parse::{Token, WordSet};

static GIT_READ_ONLY: WordSet = WordSet::new(&[
    "--version", "blame", "cat-file", "count-objects", "describe",
    "diff", "diff-tree", "fetch", "for-each-ref", "grep", "help",
    "log", "ls-files", "ls-remote", "ls-tree", "merge-base", "merge-tree",
    "name-rev", "reflog", "rev-parse", "shortlog", "show", "status",
    "verify-commit", "verify-tag",
]);

static GIT_REMOTE_MUTATING: WordSet = WordSet::new(&[
    "add", "prune", "remove", "rename", "set-branches", "set-url",
]);

static GIT_BRANCH_MUTATING: WordSet = WordSet::new(&[
    "--copy", "--delete", "--edit-description", "--move",
    "--set-upstream-to", "--unset-upstream",
    "-C", "-D", "-M", "-c", "-d", "-m", "-u",
]);

static GIT_STASH_SAFE: WordSet =
    WordSet::new(&["list", "show"]);

static GIT_TAG_MUTATING: WordSet = WordSet::new(&[
    "--annotate", "--delete", "--force", "--sign",
    "-a", "-d", "-f", "-s",
]);

static GIT_CONFIG_SAFE: WordSet =
    WordSet::new(&["--get", "--get-all", "--get-regexp", "--list", "-l"]);

static GIT_NOTES_SAFE: WordSet =
    WordSet::new(&["list", "show"]);

static JJ_GLOBAL_STANDALONE: WordSet = WordSet::new(&[
    "--debug", "--ignore-immutable", "--ignore-working-copy",
    "--no-pager", "--quiet", "--verbose",
]);

static JJ_GLOBAL_VALUED: WordSet =
    WordSet::new(&["--at-op", "--at-operation", "--color", "--repository", "-R"]);

static JJ_READ_ONLY: WordSet =
    WordSet::new(&["--version", "diff", "help", "log", "show", "st", "status"]);

static JJ_MULTI: &[(&str, WordSet)] = &[
    ("bookmark", WordSet::new(&["list"])),
    ("config", WordSet::new(&["get", "list"])),
    ("file", WordSet::new(&["show"])),
    ("op", WordSet::new(&["log"])),
];

static JJ_TRIPLE: &[(&str, &str, WordSet)] =
    &[("git", "remote", WordSet::new(&["list"]))];

pub fn is_safe_git(tokens: &[Token]) -> bool {
    let mut args = &tokens[1..];
    while args.len() >= 2 && args[0] == "-C" {
        args = &args[2..];
    }
    if args.is_empty() {
        return false;
    }
    let subcmd = args[0].command_name();
    if GIT_READ_ONLY.contains(subcmd) {
        let has_dangerous = args[1..].iter().any(|a| {
            a.starts_with("--upload-p") || a.starts_with("--receive-p")
        });
        return !has_dangerous;
    }
    if subcmd == "remote" {
        return args.get(1).is_none_or(|a| !GIT_REMOTE_MUTATING.contains(a));
    }
    if subcmd == "branch" {
        return args[1..].iter().all(|a| {
            !GIT_BRANCH_MUTATING.contains(a)
                && !GIT_BRANCH_MUTATING
                    .iter()
                    .any(|f| f.starts_with("--") && a.starts_with(&format!("{f}=")))
        });
    }
    if subcmd == "stash" {
        return args.get(1).is_some_and(|a| GIT_STASH_SAFE.contains(a));
    }
    if subcmd == "tag" {
        if args.len() == 1 {
            return true;
        }
        return args[1..].iter().all(|a| !GIT_TAG_MUTATING.contains(a));
    }
    if subcmd == "config" {
        return args.get(1).is_some_and(|a| GIT_CONFIG_SAFE.contains(a));
    }
    if subcmd == "worktree" {
        return args.get(1).is_some_and(|a| a == "list");
    }
    if subcmd == "notes" {
        return args.get(1).is_some_and(|a| GIT_NOTES_SAFE.contains(a));
    }
    false
}

pub fn is_safe_jj(tokens: &[Token]) -> bool {
    let mut args = &tokens[1..];
    loop {
        if args.is_empty() {
            return false;
        }
        if JJ_GLOBAL_STANDALONE.contains(&args[0]) {
            args = &args[1..];
        } else if JJ_GLOBAL_VALUED.contains(&args[0]) {
            if args.len() < 2 {
                return false;
            }
            args = &args[2..];
        } else if let Some(prefix) = args[0].split_value("=") {
            let flag = args[0].as_str().split_once('=').unwrap().0;
            if JJ_GLOBAL_VALUED.contains(flag) && !prefix.is_empty() {
                args = &args[1..];
            } else {
                break;
            }
        } else {
            break;
        }
    }
    if args.is_empty() {
        return false;
    }
    if JJ_READ_ONLY.contains(&args[0]) {
        return true;
    }
    for (prefix, actions) in JJ_MULTI.iter() {
        if args[0] == *prefix {
            return args.get(1).is_some_and(|a| actions.contains(a));
        }
    }
    for (first, second, actions) in JJ_TRIPLE.iter() {
        if args[0] == *first && args.get(1).is_some_and(|t| t == *second) {
            return args.get(2).is_some_and(|a| actions.contains(a));
        }
    }
    false
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("git",
            "Read-only: log, diff, show, status, ls-tree, grep, rev-parse, merge-base, merge-tree, fetch, help, shortlog, describe, blame, reflog, ls-files, ls-remote, diff-tree, cat-file, name-rev, for-each-ref, count-objects, verify-commit, verify-tag. \
             Guarded: remote (deny add/remove/rename/set-url/prune), branch (deny -d/-m/-c/--delete/--move/--copy), stash (list, show only), tag (list only, deny -d/-a/-s/-f), config (--list/--get/--get-all/--get-regexp/-l only), worktree (list only), notes (show, list only). \
             Supports `-C <dir>` prefix."),
        CommandDoc::handler("jj",
            "Read-only: log, diff, show, status, st, help, --version. \
             Multi-word: op log, file show, config get/list, bookmark list, git remote list. \
             Skips global flags: --ignore-working-copy, --no-pager, --quiet, --verbose, --debug, --ignore-immutable, --color, -R/--repository, --at-op/--at-operation."),
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
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
    fn git_ls_files() {
        assert!(check("git ls-files"));
    }

    #[test]
    fn git_ls_remote() {
        assert!(check("git ls-remote origin"));
    }

    #[test]
    fn git_diff_tree() {
        assert!(check("git diff-tree --no-commit-id -r HEAD"));
    }

    #[test]
    fn git_cat_file() {
        assert!(check("git cat-file -p HEAD"));
    }

    #[test]
    fn git_name_rev() {
        assert!(check("git name-rev HEAD"));
    }

    #[test]
    fn git_for_each_ref() {
        assert!(check("git for-each-ref refs/heads"));
    }

    #[test]
    fn git_count_objects() {
        assert!(check("git count-objects -v"));
    }

    #[test]
    fn git_verify_commit() {
        assert!(check("git verify-commit HEAD"));
    }

    #[test]
    fn git_verify_tag() {
        assert!(check("git verify-tag v1.0"));
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
    fn git_stash_list() {
        assert!(check("git stash list"));
    }

    #[test]
    fn git_stash_show() {
        assert!(check("git stash show -p"));
    }

    #[test]
    fn git_stash_bare_denied() {
        assert!(!check("git stash"));
    }

    #[test]
    fn git_stash_push_denied() {
        assert!(!check("git stash push"));
    }

    #[test]
    fn git_stash_pop_denied() {
        assert!(!check("git stash pop"));
    }

    #[test]
    fn git_stash_drop_denied() {
        assert!(!check("git stash drop"));
    }

    #[test]
    fn git_tag_list() {
        assert!(check("git tag"));
    }

    #[test]
    fn git_tag_list_pattern() {
        assert!(check("git tag -l 'v1.*'"));
    }

    #[test]
    fn git_tag_list_long() {
        assert!(check("git tag --list"));
    }

    #[test]
    fn git_tag_delete_denied() {
        assert!(!check("git tag -d v1.0"));
    }

    #[test]
    fn git_tag_annotate_denied() {
        assert!(!check("git tag -a v1.0 -m 'release'"));
    }

    #[test]
    fn git_tag_sign_denied() {
        assert!(!check("git tag -s v1.0"));
    }

    #[test]
    fn git_tag_force_denied() {
        assert!(!check("git tag -f v1.0"));
    }

    #[test]
    fn git_config_list() {
        assert!(check("git config --list"));
    }

    #[test]
    fn git_config_get() {
        assert!(check("git config --get user.name"));
    }

    #[test]
    fn git_config_get_all() {
        assert!(check("git config --get-all remote.origin.url"));
    }

    #[test]
    fn git_config_get_regexp() {
        assert!(check("git config --get-regexp 'remote.*'"));
    }

    #[test]
    fn git_config_l() {
        assert!(check("git config -l"));
    }

    #[test]
    fn git_config_set_denied() {
        assert!(!check("git config user.name foo"));
    }

    #[test]
    fn git_config_unset_denied() {
        assert!(!check("git config --unset user.name"));
    }

    #[test]
    fn git_worktree_list() {
        assert!(check("git worktree list"));
    }

    #[test]
    fn git_worktree_add_denied() {
        assert!(!check("git worktree add ../new-branch"));
    }

    #[test]
    fn git_notes_show() {
        assert!(check("git notes show HEAD"));
    }

    #[test]
    fn git_notes_list() {
        assert!(check("git notes list"));
    }

    #[test]
    fn git_notes_add_denied() {
        assert!(!check("git notes add -m 'note'"));
    }

    #[test]
    fn git_ls_remote_upload_pack_denied() {
        assert!(!check("git ls-remote --upload-pack=malicious origin"));
    }

    #[test]
    fn git_ls_remote_upload_pack_space_denied() {
        assert!(!check("git ls-remote --upload-pack malicious origin"));
    }

    #[test]
    fn git_fetch_upload_pack_denied() {
        assert!(!check("git fetch --upload-pack=malicious origin"));
    }

    #[test]
    fn git_fetch_receive_pack_denied() {
        assert!(!check("git fetch --receive-pack=malicious origin"));
    }

    #[test]
    fn git_ls_remote_upload_pack_abbreviated_denied() {
        assert!(!check("git ls-remote --upload-pa=malicious origin"));
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
    fn jj_config_list() {
        assert!(check("jj config list"));
    }

    #[test]
    fn jj_bookmark_list() {
        assert!(check("jj bookmark list"));
    }

    #[test]
    fn jj_git_remote_list() {
        assert!(check("jj git remote list"));
    }

    #[test]
    fn jj_ignore_working_copy_diff() {
        assert!(check("jj --ignore-working-copy diff --from 'trunk()' --to '@' --summary"));
    }

    #[test]
    fn jj_no_pager_log() {
        assert!(check("jj --no-pager log"));
    }

    #[test]
    fn jj_repository_flag() {
        assert!(check("jj -R /some/repo status"));
    }

    #[test]
    fn jj_color_valued() {
        assert!(check("jj --color auto log"));
    }

    #[test]
    fn jj_color_eq() {
        assert!(check("jj --color=auto log"));
    }

    #[test]
    fn jj_at_op() {
        assert!(check("jj --at-op @- diff"));
    }

    #[test]
    fn jj_multiple_global_flags() {
        assert!(check("jj --no-pager --ignore-working-copy --color=auto diff"));
    }

    #[test]
    fn jj_global_flag_no_subcommand_denied() {
        assert!(!check("jj --ignore-working-copy"));
    }

    #[test]
    fn jj_global_flag_mutating_denied() {
        assert!(!check("jj --ignore-working-copy new master"));
    }

    #[test]
    fn jj_global_flag_multi_word() {
        assert!(check("jj --no-pager bookmark list"));
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
