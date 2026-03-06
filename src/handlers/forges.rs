use crate::parse::{Segment, Token, WordSet, has_flag};
use crate::policy::{self, FlagPolicy, FlagStyle};

static GH_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--comments", "--draft", "--web",
    ]),
    standalone_short: b"qw",
    valued: WordSet::new(&[
        "--app", "--assignee", "--author", "--base", "--head",
        "--jq", "--json", "--label", "--limit", "--milestone",
        "--repo", "--search", "--state", "--template",
    ]),
    valued_short: b"BHLRS",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_VIEW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--comments", "--web",
    ]),
    standalone_short: b"cqw",
    valued: WordSet::new(&[
        "--jq", "--json", "--repo", "--template",
    ]),
    valued_short: b"R",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_DIFF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--name-only", "--patch", "--web",
    ]),
    standalone_short: b"w",
    valued: WordSet::new(&[
        "--color", "--repo",
    ]),
    valued_short: b"R",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_CHECKS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--fail-fast", "--required", "--watch", "--web",
    ]),
    standalone_short: b"w",
    valued: WordSet::new(&[
        "--interval", "--jq", "--json", "--repo", "--template",
    ]),
    valued_short: b"iR",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_STATUS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--exit-status", "--log", "--log-failed", "--web",
    ]),
    standalone_short: b"w",
    valued: WordSet::new(&[
        "--jq", "--json", "--repo", "--template",
    ]),
    valued_short: b"R",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_RUN_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"q",
    valued: WordSet::new(&[
        "--branch", "--commit", "--created", "--event",
        "--jq", "--json", "--limit", "--repo",
        "--status", "--template", "--user", "--workflow",
    ]),
    valued_short: b"bLRuw",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_RUN_VIEW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--exit-status", "--log", "--log-failed", "--verbose", "--web",
    ]),
    standalone_short: b"vw",
    valued: WordSet::new(&[
        "--attempt", "--job", "--jq", "--json", "--repo", "--template",
    ]),
    valued_short: b"jR",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_RUN_WATCH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--exit-status",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "--interval", "--repo",
    ]),
    valued_short: b"iR",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_RELEASE_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--exclude-drafts", "--exclude-pre-releases",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "--jq", "--json", "--limit", "--order", "--repo", "--template",
    ]),
    valued_short: b"LR",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_RELEASE_VIEW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--web",
    ]),
    standalone_short: b"w",
    valued: WordSet::new(&[
        "--jq", "--json", "--repo", "--template",
    ]),
    valued_short: b"R",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_SEARCH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--archived", "--include-forks", "--web",
    ]),
    standalone_short: b"w",
    valued: WordSet::new(&[
        "--assignee", "--author", "--closed", "--committer",
        "--created", "--filename", "--followers", "--forks",
        "--good-first-issues", "--hash", "--help-wanted-issues",
        "--include", "--interactions", "--jq", "--json",
        "--label", "--language", "--license", "--limit",
        "--match", "--mentions", "--merged", "--milestone",
        "--no-assignee", "--number", "--order", "--owner",
        "--parent", "--reactions", "--repo", "--review-requested",
        "--reviewed-by", "--size", "--sort", "--stars",
        "--state", "--team-review-requested", "--template",
        "--topic", "--updated", "--visibility",
    ]),
    valued_short: b"LR",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_SIMPLE_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"q",
    valued: WordSet::new(&[
        "--jq", "--json", "--limit", "--repo", "--template",
    ]),
    valued_short: b"LR",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_SIMPLE_VIEW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--web"]),
    standalone_short: b"w",
    valued: WordSet::new(&[
        "--jq", "--json", "--repo", "--template",
    ]),
    valued_short: b"R",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static READ_ONLY_SUBCOMMANDS: WordSet = WordSet::new(&[
    "attestation", "cache", "codespace", "extension", "gpg-key",
    "issue", "label", "pr", "release", "repo", "run",
    "ssh-key", "variable", "workflow",
]);

static READ_ONLY_ACTIONS: WordSet =
    WordSet::new(&["checks", "diff", "list", "status", "verify", "view", "watch"]);

static ALWAYS_SAFE_SUBCOMMANDS: WordSet =
    WordSet::new(&["--version", "search", "status"]);

static GH_BROWSE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--actions", "--no-browser", "--projects",
        "--releases", "--settings", "--wiki",
    ]),
    standalone_short: b"acnprsw",
    valued: WordSet::new(&["--branch", "--commit", "--repo"]),
    valued_short: b"bR",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static API_BODY_FLAGS: &[&str] = &["-f", "-F", "--field", "--raw-field", "--input"];

fn gh_action_policy(action: &str) -> &'static FlagPolicy {
    match action {
        "list" => &GH_LIST_POLICY,
        "view" => &GH_VIEW_POLICY,
        "diff" => &GH_DIFF_POLICY,
        "checks" => &GH_CHECKS_POLICY,
        "status" => &GH_STATUS_POLICY,
        "verify" | "watch" => &GH_SIMPLE_VIEW_POLICY,
        _ => &GH_SIMPLE_LIST_POLICY,
    }
}

fn gh_run_action_policy(action: &str) -> &'static FlagPolicy {
    match action {
        "list" => &GH_RUN_LIST_POLICY,
        "view" => &GH_RUN_VIEW_POLICY,
        "watch" => &GH_RUN_WATCH_POLICY,
        _ => &GH_SIMPLE_VIEW_POLICY,
    }
}

fn gh_release_action_policy(action: &str) -> &'static FlagPolicy {
    match action {
        "list" => &GH_RELEASE_LIST_POLICY,
        "view" => &GH_RELEASE_VIEW_POLICY,
        _ => &GH_SIMPLE_LIST_POLICY,
    }
}

pub fn is_safe_gh(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let subcmd = &tokens[1];

    if subcmd == "search" {
        return tokens.len() >= 3 && policy::check(&tokens[2..], &GH_SEARCH_POLICY);
    }

    if subcmd == "status" {
        return policy::check(&tokens[1..], &GH_SIMPLE_LIST_POLICY);
    }

    if READ_ONLY_SUBCOMMANDS.contains(subcmd) {
        if tokens.len() < 3 || !READ_ONLY_ACTIONS.contains(&tokens[2]) {
            return false;
        }
        let action = tokens[2].as_str();
        let policy = if subcmd == "run" {
            gh_run_action_policy(action)
        } else if subcmd == "release" {
            gh_release_action_policy(action)
        } else {
            gh_action_policy(action)
        };
        return policy::check(&tokens[2..], policy);
    }

    if subcmd == "auth" {
        if tokens.len() < 3 {
            return false;
        }
        if tokens[2] == "status" {
            return policy::check(&tokens[2..], &GH_SIMPLE_LIST_POLICY);
        }
        return false;
    }

    if subcmd == "browse" {
        return has_flag(&tokens[1..], Some("-n"), Some("--no-browser"))
            && policy::check(&tokens[1..], &GH_BROWSE_POLICY);
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

static GLAB_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--closed", "--draft", "--merged",
    ]),
    standalone_short: b"AacdgMq",
    valued: WordSet::new(&[
        "--assignee", "--author", "--group", "--label",
        "--milestone", "--not-label", "--order", "--output",
        "--page", "--per-page", "--repo", "--reviewer",
        "--search", "--sort", "--source-branch", "--state",
        "--target-branch",
    ]),
    valued_short: b"aFglmoPpRrSst",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GLAB_VIEW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--comments", "--resolved", "--system-logs",
        "--unresolved", "--web",
    ]),
    standalone_short: b"cpsw",
    valued: WordSet::new(&[
        "--output", "--page", "--per-page", "--repo",
    ]),
    valued_short: b"FPpR",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GLAB_DIFF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--raw",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "--color", "--repo",
    ]),
    valued_short: b"R",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GLAB_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"q",
    valued: WordSet::new(&[
        "--output", "--page", "--per-page", "--repo",
    ]),
    valued_short: b"FPpR",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn glab_action_policy(action: &str) -> &'static FlagPolicy {
    match action {
        "list" | "issues" => &GLAB_LIST_POLICY,
        "view" => &GLAB_VIEW_POLICY,
        "diff" => &GLAB_DIFF_POLICY,
        "status" => &GLAB_SIMPLE_POLICY,
        _ => &GLAB_SIMPLE_POLICY,
    }
}

pub fn is_safe_glab(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let subcmd = &tokens[1];

    if GLAB_READ_ONLY_SUBCOMMANDS.contains(subcmd) {
        if tokens.len() < 3 || !GLAB_READ_ONLY_ACTIONS.contains(&tokens[2]) {
            return false;
        }
        let policy = glab_action_policy(tokens[2].as_str());
        return policy::check(&tokens[2..], policy);
    }

    if GLAB_ALWAYS_SAFE.contains(subcmd) {
        return tokens.len() == 2;
    }

    if subcmd == "auth" {
        if tokens.len() < 3 || !GLAB_AUTH_SAFE.contains(&tokens[2]) {
            return false;
        }
        return policy::check(&tokens[2..], &GLAB_SIMPLE_POLICY);
    }

    if subcmd == "api" {
        return is_safe_gh_api(tokens);
    }

    false
}

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

static TEA_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "--fields", "--limit", "--login", "--output",
        "--page", "--repo", "--state",
    ]),
    valued_short: b"flLoopRs",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TEA_VIEW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--comments",
    ]),
    standalone_short: b"c",
    valued: WordSet::new(&[
        "--login", "--output", "--repo",
    ]),
    valued_short: b"loR",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_tea(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let subcmd = &tokens[1];

    if TEA_READ_ONLY_SUBCOMMANDS.contains(subcmd) {
        if tokens.len() == 2 {
            return true;
        }
        if !TEA_READ_ONLY_ACTIONS.contains(&tokens[2]) {
            return false;
        }
        let policy = if tokens[2] == "view" {
            &TEA_VIEW_POLICY
        } else {
            &TEA_LIST_POLICY
        };
        return policy::check(&tokens[2..], policy);
    }

    if TEA_ALWAYS_SAFE.contains(subcmd) {
        return tokens.len() == 2;
    }

    if subcmd == "logins" || subcmd == "login" {
        if tokens.len() < 3 || !TEA_LOGIN_SAFE.contains(&tokens[2]) {
            return false;
        }
        return policy::check(&tokens[2..], &TEA_LIST_POLICY);
    }

    false
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "gh" => Some(is_safe_gh(tokens)),
        "glab" => Some(is_safe_glab(tokens)),
        "tea" => Some(is_safe_tea(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocBuilder, wordset_items};
    vec![
        CommandDoc::handler("gh",
            DocBuilder::new()
                .section(format!("Subcommands {} are allowed with actions: {}.",
                    wordset_items(&READ_ONLY_SUBCOMMANDS),
                    wordset_items(&READ_ONLY_ACTIONS)))
                .section(format!("Always safe: {}.",
                    wordset_items(&ALWAYS_SAFE_SUBCOMMANDS)))
                .section("auth status, browse (requires --no-browser), \
                          api (GET only).")
                .section("")
                .build()),
        CommandDoc::handler("glab",
            DocBuilder::new()
                .section(format!("Subcommands {} are allowed with actions: {}.",
                    wordset_items(&GLAB_READ_ONLY_SUBCOMMANDS),
                    wordset_items(&GLAB_READ_ONLY_ACTIONS)))
                .section(format!("Always safe: {}.",
                    wordset_items(&GLAB_ALWAYS_SAFE)))
                .section("auth status, api (GET only).")
                .section("")
                .build()),
        CommandDoc::handler("tea",
            DocBuilder::new()
                .section(format!("Subcommands {} are allowed with actions: {} or bare invocation.",
                    wordset_items(&TEA_READ_ONLY_SUBCOMMANDS),
                    wordset_items(&TEA_READ_ONLY_ACTIONS)))
                .section(format!("Always safe: {}.",
                    wordset_items(&TEA_ALWAYS_SAFE)))
                .section("logins/login (list only).")
                .section("")
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
        pr_view_json: "gh pr view 123 --json title,body",
        pr_view_web: "gh pr view 123 --web",
        pr_view_comments: "gh pr view 123 --comments",
        pr_list: "gh pr list",
        pr_list_state: "gh pr list --state open",
        pr_list_label: "gh pr list --label bug",
        pr_list_author: "gh pr list --author user",
        pr_list_json: "gh pr list --json number,title --jq '.[].title'",
        pr_list_limit: "gh pr list --limit 50",
        pr_list_search: "gh pr list --search 'is:draft'",
        pr_diff: "gh pr diff 123",
        pr_diff_color: "gh pr diff 123 --color always",
        pr_diff_name_only: "gh pr diff 123 --name-only",
        pr_checks: "gh pr checks 123",
        pr_checks_watch: "gh pr checks 123 --watch",
        pr_checks_required: "gh pr checks 123 --required",
        issue_view: "gh issue view 456",
        issue_list: "gh issue list",
        issue_list_state: "gh issue list --state closed",
        auth_status: "gh auth status",
        search_issues: "gh search issues foo",
        search_issues_state: "gh search issues foo --state open",
        search_issues_json: "gh search issues foo --json number,title",
        search_prs: "gh search prs bar",
        search_repos: "gh search repos baz --language rust",
        run_view: "gh run view 789",
        run_view_log: "gh run view 789 --log",
        run_view_log_failed: "gh run view 789 --log-failed",
        run_view_exit_status: "gh run view 789 --exit-status",
        run_view_json: "gh run view 789 --json conclusion",
        run_watch: "gh run watch 123",
        run_watch_repo: "gh run watch 123 --repo owner/repo",
        run_watch_exit: "gh run watch 123 --exit-status",
        run_list: "gh run list",
        run_list_workflow: "gh run list --workflow ci.yml",
        run_list_branch: "gh run list --branch main",
        run_list_status: "gh run list --status completed",
        release_list: "gh release list",
        release_list_limit: "gh release list --limit 10",
        release_view: "gh release view v1.0",
        release_view_web: "gh release view v1.0 --web",
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
        glab_mr_list_state: "glab mr list --state opened",
        glab_mr_list_author: "glab mr list --author user",
        glab_mr_list_label: "glab mr list --label bug",
        glab_mr_list_output: "glab mr list --output json",
        glab_mr_view: "glab mr view 123",
        glab_mr_view_web: "glab mr view 123 --web",
        glab_mr_view_comments: "glab mr view 123 --comments",
        glab_mr_diff: "glab mr diff 123",
        glab_mr_diff_color: "glab mr diff 123 --color always",
        glab_mr_diff_raw: "glab mr diff 123 --raw",
        glab_issue_list: "glab issue list",
        glab_issue_list_state: "glab issue list --state opened",
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
        tea_issue_view_comments: "tea issue view 1 --comments",
        tea_pull_list: "tea pull list",
        tea_pull_list_state: "tea pull list --state open",
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
        auth_token_denied: "gh auth token",
        bare_gh_denied: "gh",
        gh_pr_list_unknown_denied: "gh pr list --unknown",
        gh_pr_view_unknown_denied: "gh pr view 123 --unknown",
        gh_pr_diff_unknown_denied: "gh pr diff 123 --unknown",
        gh_issue_list_unknown_denied: "gh issue list --unknown",
        gh_run_view_unknown_denied: "gh run view 789 --unknown",
        gh_search_unknown_denied: "gh search issues foo --unknown",
        gh_status_unknown_denied: "gh status --unknown",
        gh_auth_status_unknown_denied: "gh auth status --unknown",
        glab_mr_create_denied: "glab mr create --title test",
        glab_mr_merge_denied: "glab mr merge 123",
        glab_issue_create_denied: "glab issue create --title test",
        glab_auth_login_denied: "glab auth login",
        glab_api_post_denied: "glab api projects/1/issues -X POST",
        glab_api_field_denied: "glab api projects/1/issues -f title=x",
        bare_glab_denied: "glab",
        glab_mr_list_unknown_denied: "glab mr list --unknown",
        glab_mr_view_unknown_denied: "glab mr view 123 --unknown",
        glab_issue_list_unknown_denied: "glab issue list --unknown",
        glab_version_with_extra_denied: "glab version --extra",
        glab_check_update_with_extra_denied: "glab check-update --extra",
        tea_issue_create_denied: "tea issue create --title test",
        tea_pull_create_denied: "tea pull create",
        tea_login_add_denied: "tea login add",
        tea_logout_denied: "tea logout",
        bare_tea_denied: "tea",
        tea_issue_list_unknown_denied: "tea issue list --unknown",
        tea_issue_view_unknown_denied: "tea issue view 1 --unknown",
        tea_login_list_unknown_denied: "tea login list --unknown",
        tea_whoami_with_extra_denied: "tea whoami --extra",
    }
}
