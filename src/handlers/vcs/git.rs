use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};

static GIT_REMOTE_MUTATING: WordSet = WordSet::new(&[
    "add", "prune", "remove", "rename", "set-branches", "set-url",
]);

pub fn check_git_remote(tokens: &[Token]) -> Verdict {
    if tokens.get(1).is_none_or(|a| !GIT_REMOTE_MUTATING.contains(a)) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }
}

static GIT_C_KV_EXACT: WordSet = WordSet::new(&[
    "core.askPass=",
    "core.askPass=false",
    "core.askpass=",
    "core.askpass=false",
    "core.pager=cat",
    "core.pager=less",
    "credential.helper=",
    "http.sslVerify=false",
    "http.sslVerify=true",
    "http.sslverify=false",
    "http.sslverify=true",
    "init.defaultBranch=main",
    "init.defaultBranch=master",
    "init.defaultBranch=trunk",
]);

fn is_safe_git_c_kv(kv: &str) -> bool {
    if GIT_C_KV_EXACT.contains(kv) {
        return true;
    }
    let Some((key, _)) = kv.split_once('=') else {
        return false;
    };
    let key_lc = key.to_ascii_lowercase();
    matches!(key_lc.as_str(), "safe.directory")
        || key_lc.starts_with("advice.")
        || key_lc.starts_with("color.")
}

pub fn is_safe_git(tokens: &[Token]) -> Verdict {
    let mut filtered: Vec<Token> = Vec::with_capacity(tokens.len());
    if let Some(t) = tokens.first() {
        filtered.push(t.clone());
    } else {
        return Verdict::Denied;
    }
    let mut i = 1;
    let mut saw_c = false;
    while i < tokens.len() {
        let t = &tokens[i];
        if t == "-c" {
            let Some(next) = tokens.get(i + 1) else {
                return Verdict::Denied;
            };
            if !is_safe_git_c_kv(next.as_str()) {
                return Verdict::Denied;
            }
            saw_c = true;
            i += 2;
            continue;
        }
        filtered.push(t.clone());
        i += 1;
    }
    if !saw_c {
        return crate::registry::toml_dispatch(tokens).unwrap_or(Verdict::Denied);
    }
    crate::registry::toml_dispatch(&filtered).unwrap_or(Verdict::Denied)
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        git_log: "git log --oneline -5",
        git_log_graph: "git log --graph --all --oneline",
        git_log_author: "git log --author=foo --since=2024-01-01",
        git_log_format: "git log --format='%H %s' -n 10",
        git_log_stat: "git log --stat --no-merges",
        git_log_grep_i: "git log --all --oneline --grep=pattern -i",
        git_log_bare: "git log",
        git_diff: "git diff --stat",
        git_diff_cached: "git diff --cached --name-only",
        git_diff_algorithm: "git diff --diff-algorithm=patience",
        git_show: "git show HEAD:some/file.rb",
        git_show_stat: "git show --stat HEAD",
        git_status: "git status --porcelain",
        git_status_short: "git status -sb",
        git_fetch: "git fetch origin master",
        git_fetch_all: "git fetch --all --prune",
        git_ls_tree: "git ls-tree HEAD",
        git_ls_tree_r: "git ls-tree -r --name-only HEAD",
        git_grep: "git grep pattern",
        git_grep_flags: "git grep -n -i --count pattern",
        git_rev_parse: "git rev-parse HEAD",
        git_rev_parse_toplevel: "git rev-parse --show-toplevel",
        git_merge_base: "git merge-base master HEAD",
        git_merge_tree: "git merge-tree HEAD~1 HEAD master",
        git_version: "git --version",
        git_help: "git help log",
        git_shortlog: "git shortlog -s",
        git_shortlog_ne: "git shortlog -sne",
        git_describe: "git describe --tags",
        git_describe_always: "git describe --always --abbrev=7",
        git_blame: "git blame file.rb",
        git_blame_flags: "git blame -L 10,20 -w file.rb",
        git_reflog: "git reflog",
        git_reflog_n: "git reflog -n 10",
        git_ls_files: "git ls-files",
        git_ls_files_others: "git ls-files --others --exclude-standard",
        git_ls_remote: "git ls-remote origin",
        git_ls_remote_tags: "git ls-remote --tags origin",
        git_diff_tree: "git diff-tree --no-commit-id -r HEAD",
        git_cat_file: "git cat-file -p HEAD",
        git_cat_file_t: "git cat-file -t HEAD",
        git_check_ignore: "git check-ignore .jj/",
        git_check_ignore_v: "git check-ignore -v .gitignore",
        git_name_rev: "git name-rev HEAD",
        git_for_each_ref: "git for-each-ref refs/heads",
        git_for_each_ref_format: "git for-each-ref --format='%(refname)' --sort=-committerdate",
        git_count_objects: "git count-objects -v",
        git_verify_commit: "git verify-commit HEAD",
        git_verify_tag: "git verify-tag v1.0",
        git_merge_base_all: "git merge-base --all HEAD main",
        git_c_flag: "git -C /some/repo diff --stat",
        git_c_nested: "git -C /some/repo -C nested log",
        git_config_askpass_disable: "git -c core.askPass=false ls-remote https://github.com/foo/bar",
        git_config_askpass_empty: "git -c core.askPass= ls-remote https://github.com/foo/bar",
        git_config_askpass_lower: "git -c core.askpass=false fetch origin",
        git_config_credential_helper_empty: "git -c credential.helper= ls-remote origin",
        git_config_safe_directory: "git -c safe.directory=/repo status",
        git_config_safe_directory_star: "git -c safe.directory=* log",
        git_config_advice: "git -c advice.detachedHead=false log HEAD",
        git_config_color_ui: "git -c color.ui=never status",
        git_config_pager_cat: "git -c core.pager=cat log",
        git_config_sslverify_false: "git -c http.sslVerify=false fetch origin",
        git_config_init_default_branch: "git -c init.defaultBranch=main ls-remote origin",
        git_config_multiple_c: "git -c core.askPass=false -c credential.helper= ls-remote origin",
        git_config_lower_c_with_upper_c: "git -C /repo -c core.askPass=false log",
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
        git_config_local_list: "git config --local --list",
        git_config_global_list: "git config --global --list",
        git_config_system_l: "git config --system -l",
        git_config_local_get: "git config --local --get user.name",
        git_config_file_list: "git config --file .gitconfig --list",
        git_config_f_list: "git config -f .gitconfig -l",
        git_config_show_origin_list: "git config --show-origin --list",
        git_config_show_scope_local_list: "git config --show-scope --local --list",
        git_config_name_only_list: "git config --name-only --list",
        git_config_worktree_list: "git config --worktree -l",
        git_config_blob_get: "git config --blob HEAD:.gitmodules --get submodule.foo.url",
        git_config_local_help: "git config --local --help",
        git_worktree_list: "git worktree list",
        git_worktree_list_porcelain: "git worktree list --porcelain",
        git_worktree_list_verbose: "git worktree list -v",
        git_notes_show: "git notes show HEAD",
        git_notes_list: "git notes list",
        git_worktree_help: "git worktree --help",
        git_worktree_help_h: "git worktree -h",
    }

    denied! {
        git_rebase_help_denied: "git rebase --help",
        git_push_help_denied: "git push --help",
        git_fetch_upload_pack_denied: "git fetch --upload-pack=malicious origin",
        git_ls_remote_upload_pack_denied: "git ls-remote --upload-pack malicious origin",
        git_tag_delete_denied: "git tag -d v1.0",
        git_tag_annotate_denied: "git tag -a v1.0 -m 'release'",
        git_tag_sign_denied: "git tag -s v1.0",
        git_tag_force_denied: "git tag -f v1.0",
        git_config_set_denied: "git config user.name foo",
        git_config_unset_denied: "git config --unset user.name",
        git_config_local_set_denied: "git config --local user.name foo",
        git_config_local_unset_denied: "git config --local --unset user.name",
        git_config_local_evil_list_denied: "git config --local --evil --list",
        git_config_list_trailing_flag_denied: "git config --list --evil",
        git_config_get_trailing_flag_denied: "git config --get user.name --evil",
        git_worktree_add_denied: "git worktree add ../new-branch",
        git_worktree_list_trailing_denied: "git worktree list --evil",
        git_notes_list_trailing_flag_denied: "git notes list --evil",
        git_branch_d_denied: "git branch -D feature",
        git_branch_delete_denied: "git branch --delete feature",
        git_branch_move_denied: "git branch -m old new",
        git_branch_copy_denied: "git branch -c old new",
        git_branch_set_upstream_denied: "git branch --set-upstream-to=origin/main",
        git_remote_add_denied: "git remote add upstream https://github.com/foo/bar",
        git_remote_remove_denied: "git remote remove upstream",
        git_remote_rename_denied: "git remote rename origin upstream",
        git_config_flag_denied: "git -c user.name=foo log",
        git_config_editor_denied: "git -c core.editor=vim commit",
        git_config_ssh_command_denied: "git -c core.sshCommand=evil ls-remote origin",
        git_config_alias_denied: "git -c alias.evil=push log",
        git_config_hooks_path_denied: "git -c core.hooksPath=/tmp log",
        git_config_pager_arbitrary_denied: "git -c core.pager=evil log",
        git_config_no_value_denied: "git -c log",
        git_help_bypass_denied: "git push -- --help",
    }
}
