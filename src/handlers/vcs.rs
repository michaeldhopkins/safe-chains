use crate::parse::{Token, WordSet};

static GIT_READ_ONLY: WordSet = WordSet::new(&[
    "--version", "blame", "cat-file", "check-ignore", "count-objects", "describe",
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
    ("git", WordSet::new(&["fetch"])),
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
        if args[0] == *prefix && args.get(1).is_some_and(|a| actions.contains(a)) {
            return true;
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
    use crate::docs::{CommandDoc, doc, doc_multi, wordset_items};
    vec![
        CommandDoc::handler("git",
            doc(&GIT_READ_ONLY)
                .section(format!(
                    "Guarded: remote (deny {}), branch (deny {}), stash ({} only), \
                     tag (list only, deny {}), config ({} only), worktree (list only), \
                     notes ({} only). Supports `-C <dir>` prefix.",
                    wordset_items(&GIT_REMOTE_MUTATING),
                    wordset_items(&GIT_BRANCH_MUTATING),
                    wordset_items(&GIT_STASH_SAFE),
                    wordset_items(&GIT_TAG_MUTATING),
                    wordset_items(&GIT_CONFIG_SAFE),
                    wordset_items(&GIT_NOTES_SAFE),
                ))
                .build()),
        CommandDoc::handler("jj",
            doc_multi(&JJ_READ_ONLY, JJ_MULTI)
                .triple_word(JJ_TRIPLE)
                .section(format!(
                    "Skips global flags: standalone ({}), valued ({}).",
                    wordset_items(&JJ_GLOBAL_STANDALONE),
                    wordset_items(&JJ_GLOBAL_VALUED),
                ))
                .build()),
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        git_log: "git log --oneline -5",
        git_diff: "git diff --stat",
        git_show: "git show HEAD:some/file.rb",
        git_status: "git status --porcelain",
        git_fetch: "git fetch origin master",
        git_ls_tree: "git ls-tree HEAD",
        git_grep: "git grep pattern",
        git_rev_parse: "git rev-parse HEAD",
        git_merge_base: "git merge-base master HEAD",
        git_merge_tree: "git merge-tree HEAD~1 HEAD master",
        git_version: "git --version",
        git_help: "git help log",
        git_shortlog: "git shortlog -s",
        git_describe: "git describe --tags",
        git_blame: "git blame file.rb",
        git_reflog: "git reflog",
        git_ls_files: "git ls-files",
        git_ls_remote: "git ls-remote origin",
        git_diff_tree: "git diff-tree --no-commit-id -r HEAD",
        git_cat_file: "git cat-file -p HEAD",
        git_check_ignore: "git check-ignore .jj/",
        git_name_rev: "git name-rev HEAD",
        git_for_each_ref: "git for-each-ref refs/heads",
        git_count_objects: "git count-objects -v",
        git_verify_commit: "git verify-commit HEAD",
        git_verify_tag: "git verify-tag v1.0",
        git_c_flag: "git -C /some/repo diff --stat",
        git_c_nested: "git -C /some/repo -C nested log",
        git_remote_bare: "git remote",
        git_remote_v: "git remote -v",
        git_remote_get_url: "git remote get-url origin",
        git_remote_show: "git remote show origin",
        git_branch_list: "git branch",
        git_branch_list_all: "git branch -a",
        git_branch_list_verbose: "git branch -v",
        git_branch_contains: "git branch --contains abc123",
        git_stash_list: "git stash list",
        git_stash_show: "git stash show -p",
        git_tag_list: "git tag",
        git_tag_list_pattern: "git tag -l 'v1.*'",
        git_tag_list_long: "git tag --list",
        git_config_list: "git config --list",
        git_config_get: "git config --get user.name",
        git_config_get_all: "git config --get-all remote.origin.url",
        git_config_get_regexp: "git config --get-regexp 'remote.*'",
        git_config_l: "git config -l",
        git_worktree_list: "git worktree list",
        git_notes_show: "git notes show HEAD",
        git_notes_list: "git notes list",
        jj_log: "jj log",
        jj_diff: "jj diff --stat",
        jj_show: "jj show abc123",
        jj_status: "jj status",
        jj_st: "jj st",
        jj_help: "jj help",
        jj_version: "jj --version",
        jj_op_log: "jj op log",
        jj_file_show: "jj file show some/path",
        jj_config_get: "jj config get user.name",
        jj_config_list: "jj config list",
        jj_bookmark_list: "jj bookmark list",
        jj_git_remote_list: "jj git remote list",
        jj_ignore_working_copy_diff: "jj --ignore-working-copy diff --from 'trunk()' --to '@' --summary",
        jj_no_pager_log: "jj --no-pager log",
        jj_repository_flag: "jj -R /some/repo status",
        jj_color_valued: "jj --color auto log",
        jj_color_eq: "jj --color=auto log",
        jj_at_op: "jj --at-op @- diff",
        jj_multiple_global_flags: "jj --no-pager --ignore-working-copy --color=auto diff",
        jj_global_flag_multi_word: "jj --no-pager bookmark list",
        jj_git_fetch: "jj git fetch",
        jj_git_fetch_with_global_flags: "jj --no-pager git fetch",
    }

    denied! {
        git_stash_bare_denied: "git stash",
        git_stash_push_denied: "git stash push",
        git_stash_pop_denied: "git stash pop",
        git_stash_drop_denied: "git stash drop",
        git_tag_delete_denied: "git tag -d v1.0",
        git_tag_annotate_denied: "git tag -a v1.0 -m 'release'",
        git_tag_sign_denied: "git tag -s v1.0",
        git_tag_force_denied: "git tag -f v1.0",
        git_config_set_denied: "git config user.name foo",
        git_config_unset_denied: "git config --unset user.name",
        git_worktree_add_denied: "git worktree add ../new-branch",
        git_notes_add_denied: "git notes add -m 'note'",
        git_ls_remote_upload_pack_denied: "git ls-remote --upload-pack=malicious origin",
        git_ls_remote_upload_pack_space_denied: "git ls-remote --upload-pack malicious origin",
        git_fetch_upload_pack_denied: "git fetch --upload-pack=malicious origin",
        git_fetch_receive_pack_denied: "git fetch --receive-pack=malicious origin",
        git_ls_remote_upload_pack_abbreviated_denied: "git ls-remote --upload-pa=malicious origin",
        git_push_denied: "git push origin main",
        git_reset_denied: "git reset --hard HEAD~1",
        git_add_denied: "git add .",
        git_commit_denied: "git commit -m 'test'",
        git_checkout_denied: "git checkout -- file.rb",
        git_rebase_denied: "git rebase origin/master",
        git_stash_denied: "git stash",
        git_branch_d_denied: "git branch -D feature",
        git_branch_delete_denied: "git branch --delete feature",
        git_branch_move_denied: "git branch -m old new",
        git_branch_copy_denied: "git branch -c old new",
        git_branch_set_upstream_denied: "git branch --set-upstream-to=origin/main",
        git_rm_denied: "git rm file.rb",
        git_remote_add_denied: "git remote add upstream https://github.com/foo/bar",
        git_remote_remove_denied: "git remote remove upstream",
        git_remote_rename_denied: "git remote rename origin upstream",
        git_config_flag_denied: "git -c user.name=foo log",
        bare_git_denied: "git",
        jj_global_flag_no_subcommand_denied: "jj --ignore-working-copy",
        jj_global_flag_mutating_denied: "jj --ignore-working-copy new master",
        jj_new_denied: "jj new master",
        jj_edit_denied: "jj edit abc123",
        jj_squash_denied: "jj squash",
        jj_describe_denied: "jj describe -m 'test'",
        jj_bookmark_denied: "jj bookmark set my-branch",
        jj_git_push_denied: "jj git push",
        jj_rebase_denied: "jj rebase -d master",
        jj_restore_denied: "jj restore file.rb",
        jj_abandon_denied: "jj abandon",
        jj_config_set_denied: "jj config set user.name foo",
        bare_jj_denied: "jj",
    }
}
