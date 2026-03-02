use crate::parse::{FlagCheck, Token, WordSet};

static READ_ONLY_SUBCOMMANDS: WordSet = WordSet::new(&[
    "attestation", "cache", "codespace", "extension", "gpg-key",
    "issue", "label", "pr", "release", "repo", "run",
    "ssh-key", "variable", "workflow",
]);

static READ_ONLY_ACTIONS: WordSet =
    WordSet::new(&["checks", "diff", "list", "status", "verify", "view", "watch"]);

static ALWAYS_SAFE_SUBCOMMANDS: WordSet =
    WordSet::new(&["--version", "search", "status"]);

static AUTH_SAFE_ACTIONS: WordSet =
    WordSet::new(&["status", "token"]);

static GH_BROWSE: FlagCheck =
    FlagCheck::new(&["--no-browser"], &[]);

static API_BODY_FLAGS: &[&str] = &["-f", "-F", "--field", "--raw-field", "--input"];

static GLAB_READ_ONLY_SUBCOMMANDS: WordSet = WordSet::new(&[
    "ci", "cluster", "deploy-key", "gpg-key", "incident", "issue",
    "iteration", "label", "milestone", "mr", "release", "repo",
    "schedule", "snippet", "ssh-key", "stack", "variable",
]);

static GLAB_READ_ONLY_ACTIONS: WordSet =
    WordSet::new(&["diff", "issues", "list", "status", "view"]);

static GLAB_ALWAYS_SAFE: WordSet =
    WordSet::new(&["--version", "-v", "check-update", "version"]);

static GLAB_AUTH_SAFE: WordSet =
    WordSet::new(&["status"]);

static TEA_READ_ONLY_SUBCOMMANDS: WordSet = WordSet::new(&[
    "b", "branch", "branches",
    "i", "issue", "issues",
    "label", "labels",
    "milestone", "milestones", "ms",
    "n", "notification", "notifications",
    "org", "organization", "organizations",
    "pr", "pull", "pulls",
    "r", "release", "releases",
    "repo", "repos",
    "t", "time", "times",
]);

static TEA_READ_ONLY_ACTIONS: WordSet =
    WordSet::new(&["list", "view"]);

static TEA_ALWAYS_SAFE: WordSet =
    WordSet::new(&["--version", "-v", "whoami"]);

static TEA_LOGIN_SAFE: WordSet =
    WordSet::new(&["list"]);

pub fn is_safe_gh(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let subcmd = &tokens[1];

    if READ_ONLY_SUBCOMMANDS.contains(subcmd) {
        return tokens.len() >= 3 && READ_ONLY_ACTIONS.contains(&tokens[2]);
    }

    if ALWAYS_SAFE_SUBCOMMANDS.contains(subcmd) {
        return true;
    }

    if subcmd == "auth" {
        return tokens.len() >= 3 && AUTH_SAFE_ACTIONS.contains(&tokens[2]);
    }

    if subcmd == "browse" {
        return GH_BROWSE.is_safe(&tokens[2..]);
    }

    if subcmd == "api" {
        return is_safe_gh_api(tokens);
    }

    false
}

fn is_safe_gh_api(tokens: &[Token]) -> bool {
    for (i, token) in tokens[2..].iter().enumerate() {
        let abs_i = i + 2;

        if token == "-X" || token == "--method" {
            return tokens
                .get(abs_i + 1)
                .is_some_and(|m| m.eq_ignore_ascii_case("GET"));
        }
        if token.starts_with("-X") && token.len() > 2 && !token.starts_with("-X=") {
            return token
                .get(2..)
                .is_some_and(|s| s.eq_ignore_ascii_case("GET"));
        }
        if token.starts_with("-X=") || token.starts_with("--method=") {
            let val = token.split_value("=").unwrap_or("");
            return val.eq_ignore_ascii_case("GET");
        }

        for flag in API_BODY_FLAGS {
            if token == *flag {
                return false;
            }
            if flag.len() == 2 && token.len() > 2 && token.starts_with(flag) {
                return false;
            }
            if flag.starts_with("--") && token.starts_with(&format!("{flag}=")) {
                return false;
            }
        }
    }
    true
}

pub fn is_safe_glab(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let subcmd = &tokens[1];

    if GLAB_READ_ONLY_SUBCOMMANDS.contains(subcmd) {
        return tokens.len() >= 3 && GLAB_READ_ONLY_ACTIONS.contains(&tokens[2]);
    }

    if GLAB_ALWAYS_SAFE.contains(subcmd) {
        return true;
    }

    if subcmd == "auth" {
        return tokens.len() >= 3 && GLAB_AUTH_SAFE.contains(&tokens[2]);
    }

    if subcmd == "api" {
        return is_safe_gh_api(tokens);
    }

    false
}

pub fn is_safe_tea(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let subcmd = &tokens[1];

    if TEA_READ_ONLY_SUBCOMMANDS.contains(subcmd) {
        if tokens.len() == 2 {
            return true;
        }
        return TEA_READ_ONLY_ACTIONS.contains(&tokens[2]);
    }

    if TEA_ALWAYS_SAFE.contains(subcmd) {
        return true;
    }

    if subcmd == "logins" || subcmd == "login" {
        return tokens.len() >= 3 && TEA_LOGIN_SAFE.contains(&tokens[2]);
    }

    false
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocBuilder, describe_flagcheck, wordset_items};
    vec![
        CommandDoc::handler("gh",
            DocBuilder::new()
                .section(format!("Subcommands {} are allowed with actions: {}.",
                    wordset_items(&READ_ONLY_SUBCOMMANDS),
                    wordset_items(&READ_ONLY_ACTIONS)))
                .section(format!("Always safe: {}.",
                    wordset_items(&ALWAYS_SAFE_SUBCOMMANDS)))
                .section(format!("Guarded: auth ({} only), browse ({}), api (GET only, no body flags).",
                    wordset_items(&AUTH_SAFE_ACTIONS),
                    describe_flagcheck(&GH_BROWSE).trim_end_matches('.').to_lowercase()))
                .build()),
        CommandDoc::handler("glab",
            DocBuilder::new()
                .section(format!("Subcommands {} are allowed with actions: {}.",
                    wordset_items(&GLAB_READ_ONLY_SUBCOMMANDS),
                    wordset_items(&GLAB_READ_ONLY_ACTIONS)))
                .section(format!("Always safe: {}.",
                    wordset_items(&GLAB_ALWAYS_SAFE)))
                .section(format!("Guarded: auth ({} only), api (GET only, no body flags).",
                    wordset_items(&GLAB_AUTH_SAFE)))
                .build()),
        CommandDoc::handler("tea",
            DocBuilder::new()
                .section(format!("Subcommands {} are allowed with actions: {}. \
                    Bare subcommand (no action) is also safe.",
                    wordset_items(&TEA_READ_ONLY_SUBCOMMANDS),
                    wordset_items(&TEA_READ_ONLY_ACTIONS)))
                .section(format!("Always safe: {}.",
                    wordset_items(&TEA_ALWAYS_SAFE)))
                .section(format!("Guarded: logins/login ({} only).",
                    wordset_items(&TEA_LOGIN_SAFE)))
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
        pr_view: "gh pr view 123",
        pr_list: "gh pr list",
        pr_diff: "gh pr diff 123",
        pr_checks: "gh pr checks 123",
        issue_view: "gh issue view 456",
        issue_list: "gh issue list",
        auth_status: "gh auth status",
        search_issues: "gh search issues foo",
        search_prs: "gh search prs bar",
        run_view: "gh run view 789",
        run_watch: "gh run watch 123 --repo owner/repo",
        release_list: "gh release list",
        label_list: "gh label list",
        codespace_list: "gh codespace list",
        variable_list: "gh variable list",
        extension_list: "gh extension list",
        cache_list: "gh cache list",
        attestation_verify: "gh attestation verify artifact.tar.gz",
        gpg_key_list: "gh gpg-key list",
        ssh_key_list: "gh ssh-key list",
        status_safe: "gh status",
        browse_no_browser: "gh browse --no-browser",
        browse_no_browser_with_path: "gh browse src/main.rs --no-browser",
        api_get_implicit: "gh api repos/o/r/pulls/1",
        api_jq: "gh api repos/o/r/contents/f --jq '.content'",
        api_explicit_get: "gh api repos/o/r/pulls -X GET",
        api_paginate: "gh api repos/o/r/pulls --paginate",
        api_xget_short: "gh api repos/o/r/pulls -XGET",
        gh_version: "gh --version",
        glab_mr_list: "glab mr list",
        glab_mr_view: "glab mr view 123",
        glab_mr_diff: "glab mr diff 123",
        glab_issue_list: "glab issue list",
        glab_issue_view: "glab issue view 456",
        glab_ci_status: "glab ci status",
        glab_ci_list: "glab ci list",
        glab_release_list: "glab release list",
        glab_label_list: "glab label list",
        glab_milestone_list: "glab milestone list",
        glab_snippet_view: "glab snippet view 1",
        glab_variable_list: "glab variable list",
        glab_auth_status: "glab auth status",
        glab_version: "glab --version",
        glab_version_subcommand: "glab version",
        glab_check_update: "glab check-update",
        glab_api_get_implicit: "glab api projects/1/merge_requests",
        glab_api_explicit_get: "glab api projects/1/issues -X GET",
        tea_issue_list: "tea issue list",
        tea_issues_list: "tea issues list",
        tea_issue_view: "tea issue view 1",
        tea_pull_list: "tea pull list",
        tea_pr_view: "tea pr view 1",
        tea_release_list: "tea release list",
        tea_repo_bare: "tea repo",
        tea_repo_list: "tea repos list",
        tea_branch_list: "tea branch list",
        tea_label_list: "tea labels list",
        tea_milestone_list: "tea milestones list",
        tea_org_list: "tea org list",
        tea_notifications_bare: "tea notifications",
        tea_times_list: "tea times list",
        tea_whoami: "tea whoami",
        tea_version: "tea --version",
        tea_login_list: "tea login list",
        tea_logins_list: "tea logins list",
    }

    denied! {
        browse_without_flag_denied: "gh browse",
        pr_create_denied: "gh pr create --title test",
        pr_merge_denied: "gh pr merge 123",
        api_patch_denied: "gh api repos/o/r/pulls/1 -X PATCH -f body=x",
        api_post_denied: "gh api repos/o/r/pulls/1 -X POST",
        api_field_denied: "gh api repos/o/r/issues -f title=x",
        api_method_eq_patch_denied: "gh api repos/o/r/pulls/1 --method=PATCH",
        api_xpost_short_denied: "gh api repos/o/r/pulls -XPOST",
        api_xpatch_short_denied: "gh api repos/o/r/pulls -XPATCH",
        auth_login_denied: "gh auth login",
        bare_gh_denied: "gh",
        glab_mr_create_denied: "glab mr create --title test",
        glab_mr_merge_denied: "glab mr merge 123",
        glab_issue_create_denied: "glab issue create --title test",
        glab_auth_login_denied: "glab auth login",
        glab_api_post_denied: "glab api projects/1/issues -X POST",
        glab_api_field_denied: "glab api projects/1/issues -f title=x",
        bare_glab_denied: "glab",
        tea_issue_create_denied: "tea issue create --title test",
        tea_pull_create_denied: "tea pull create",
        tea_login_add_denied: "tea login add",
        tea_logout_denied: "tea logout",
        bare_tea_denied: "tea",
    }
}
