use crate::parse::{FlagCheck, Token, WordSet};

static READ_ONLY_SUBCOMMANDS: WordSet = WordSet::new(&[
    "attestation", "cache", "codespace", "extension", "gpg-key",
    "issue", "label", "pr", "release", "repo", "run",
    "ssh-key", "variable", "workflow",
]);

static READ_ONLY_ACTIONS: WordSet =
    WordSet::new(&["checks", "diff", "list", "status", "verify", "view"]);

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
    use crate::docs::{CommandDoc, describe_wordset, describe_flagcheck};
    let actions = describe_wordset(&READ_ONLY_ACTIONS).trim_start_matches("Allowed: ").trim_end_matches('.').to_string();
    vec![
        CommandDoc::handler("gh", format!(
            "Read-only subcommands ({actions}): {}. \
             Always safe: {}. \
             Guarded: auth ({} only), browse ({}), api (GET only, no body flags).",
            describe_wordset(&READ_ONLY_SUBCOMMANDS).trim_start_matches("Allowed: ").trim_end_matches('.'),
            describe_wordset(&ALWAYS_SAFE_SUBCOMMANDS).trim_start_matches("Allowed: ").trim_end_matches('.'),
            describe_wordset(&AUTH_SAFE_ACTIONS).trim_start_matches("Allowed: ").trim_end_matches('.'),
            describe_flagcheck(&GH_BROWSE).trim_end_matches('.').to_lowercase(),
        )),
        CommandDoc::handler("glab", format!(
            "Read-only subcommands ({}): {}. \
             Always safe: {}. \
             Guarded: auth ({} only), api (GET only, no body flags).",
            describe_wordset(&GLAB_READ_ONLY_ACTIONS).trim_start_matches("Allowed: ").trim_end_matches('.'),
            describe_wordset(&GLAB_READ_ONLY_SUBCOMMANDS).trim_start_matches("Allowed: ").trim_end_matches('.'),
            describe_wordset(&GLAB_ALWAYS_SAFE).trim_start_matches("Allowed: ").trim_end_matches('.'),
            describe_wordset(&GLAB_AUTH_SAFE).trim_start_matches("Allowed: ").trim_end_matches('.'),
        )),
        CommandDoc::handler("tea", format!(
            "Read-only subcommands ({}): {}. \
             Bare subcommand (no action) also safe for read-only subcommands. \
             Always safe: {}. Guarded: logins/login ({} only).",
            describe_wordset(&TEA_READ_ONLY_ACTIONS).trim_start_matches("Allowed: ").trim_end_matches('.'),
            describe_wordset(&TEA_READ_ONLY_SUBCOMMANDS).trim_start_matches("Allowed: ").trim_end_matches('.'),
            describe_wordset(&TEA_ALWAYS_SAFE).trim_start_matches("Allowed: ").trim_end_matches('.'),
            describe_wordset(&TEA_LOGIN_SAFE).trim_start_matches("Allowed: ").trim_end_matches('.'),
        )),
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    #[test]
    fn pr_view() {
        assert!(check("gh pr view 123"));
    }

    #[test]
    fn pr_list() {
        assert!(check("gh pr list"));
    }

    #[test]
    fn pr_diff() {
        assert!(check("gh pr diff 123"));
    }

    #[test]
    fn pr_checks() {
        assert!(check("gh pr checks 123"));
    }

    #[test]
    fn issue_view() {
        assert!(check("gh issue view 456"));
    }

    #[test]
    fn issue_list() {
        assert!(check("gh issue list"));
    }

    #[test]
    fn auth_status() {
        assert!(check("gh auth status"));
    }

    #[test]
    fn search_issues() {
        assert!(check("gh search issues foo"));
    }

    #[test]
    fn search_prs() {
        assert!(check("gh search prs bar"));
    }

    #[test]
    fn run_view() {
        assert!(check("gh run view 789"));
    }

    #[test]
    fn release_list() {
        assert!(check("gh release list"));
    }

    #[test]
    fn label_list() {
        assert!(check("gh label list"));
    }

    #[test]
    fn codespace_list() {
        assert!(check("gh codespace list"));
    }

    #[test]
    fn variable_list() {
        assert!(check("gh variable list"));
    }

    #[test]
    fn extension_list() {
        assert!(check("gh extension list"));
    }

    #[test]
    fn cache_list() {
        assert!(check("gh cache list"));
    }

    #[test]
    fn attestation_verify() {
        assert!(check("gh attestation verify artifact.tar.gz"));
    }

    #[test]
    fn gpg_key_list() {
        assert!(check("gh gpg-key list"));
    }

    #[test]
    fn ssh_key_list() {
        assert!(check("gh ssh-key list"));
    }

    #[test]
    fn status_safe() {
        assert!(check("gh status"));
    }

    #[test]
    fn browse_no_browser() {
        assert!(check("gh browse --no-browser"));
    }

    #[test]
    fn browse_no_browser_with_path() {
        assert!(check("gh browse src/main.rs --no-browser"));
    }

    #[test]
    fn browse_without_flag_denied() {
        assert!(!check("gh browse"));
    }

    #[test]
    fn api_get_implicit() {
        assert!(check("gh api repos/o/r/pulls/1"));
    }

    #[test]
    fn api_jq() {
        assert!(check("gh api repos/o/r/contents/f --jq '.content'"));
    }

    #[test]
    fn api_explicit_get() {
        assert!(check("gh api repos/o/r/pulls -X GET"));
    }

    #[test]
    fn api_paginate() {
        assert!(check("gh api repos/o/r/pulls --paginate"));
    }

    #[test]
    fn api_xget_short() {
        assert!(check("gh api repos/o/r/pulls -XGET"));
    }

    #[test]
    fn pr_create_denied() {
        assert!(!check("gh pr create --title test"));
    }

    #[test]
    fn pr_merge_denied() {
        assert!(!check("gh pr merge 123"));
    }

    #[test]
    fn api_patch_denied() {
        assert!(!check("gh api repos/o/r/pulls/1 -X PATCH -f body=x"));
    }

    #[test]
    fn api_post_denied() {
        assert!(!check("gh api repos/o/r/pulls/1 -X POST"));
    }

    #[test]
    fn api_field_denied() {
        assert!(!check("gh api repos/o/r/issues -f title=x"));
    }

    #[test]
    fn api_method_eq_patch_denied() {
        assert!(!check("gh api repos/o/r/pulls/1 --method=PATCH"));
    }

    #[test]
    fn api_xpost_short_denied() {
        assert!(!check("gh api repos/o/r/pulls -XPOST"));
    }

    #[test]
    fn api_xpatch_short_denied() {
        assert!(!check("gh api repos/o/r/pulls -XPATCH"));
    }

    #[test]
    fn auth_login_denied() {
        assert!(!check("gh auth login"));
    }

    #[test]
    fn gh_version() {
        assert!(check("gh --version"));
    }

    #[test]
    fn bare_gh_denied() {
        assert!(!check("gh"));
    }

    #[test]
    fn glab_mr_list() {
        assert!(check("glab mr list"));
    }

    #[test]
    fn glab_mr_view() {
        assert!(check("glab mr view 123"));
    }

    #[test]
    fn glab_mr_diff() {
        assert!(check("glab mr diff 123"));
    }

    #[test]
    fn glab_issue_list() {
        assert!(check("glab issue list"));
    }

    #[test]
    fn glab_issue_view() {
        assert!(check("glab issue view 456"));
    }

    #[test]
    fn glab_ci_status() {
        assert!(check("glab ci status"));
    }

    #[test]
    fn glab_ci_list() {
        assert!(check("glab ci list"));
    }

    #[test]
    fn glab_release_list() {
        assert!(check("glab release list"));
    }

    #[test]
    fn glab_label_list() {
        assert!(check("glab label list"));
    }

    #[test]
    fn glab_milestone_list() {
        assert!(check("glab milestone list"));
    }

    #[test]
    fn glab_snippet_view() {
        assert!(check("glab snippet view 1"));
    }

    #[test]
    fn glab_variable_list() {
        assert!(check("glab variable list"));
    }

    #[test]
    fn glab_auth_status() {
        assert!(check("glab auth status"));
    }

    #[test]
    fn glab_version() {
        assert!(check("glab --version"));
    }

    #[test]
    fn glab_version_subcommand() {
        assert!(check("glab version"));
    }

    #[test]
    fn glab_check_update() {
        assert!(check("glab check-update"));
    }

    #[test]
    fn glab_api_get_implicit() {
        assert!(check("glab api projects/1/merge_requests"));
    }

    #[test]
    fn glab_api_explicit_get() {
        assert!(check("glab api projects/1/issues -X GET"));
    }

    #[test]
    fn glab_mr_create_denied() {
        assert!(!check("glab mr create --title test"));
    }

    #[test]
    fn glab_mr_merge_denied() {
        assert!(!check("glab mr merge 123"));
    }

    #[test]
    fn glab_issue_create_denied() {
        assert!(!check("glab issue create --title test"));
    }

    #[test]
    fn glab_auth_login_denied() {
        assert!(!check("glab auth login"));
    }

    #[test]
    fn glab_api_post_denied() {
        assert!(!check("glab api projects/1/issues -X POST"));
    }

    #[test]
    fn glab_api_field_denied() {
        assert!(!check("glab api projects/1/issues -f title=x"));
    }

    #[test]
    fn bare_glab_denied() {
        assert!(!check("glab"));
    }

    #[test]
    fn tea_issue_list() {
        assert!(check("tea issue list"));
    }

    #[test]
    fn tea_issues_list() {
        assert!(check("tea issues list"));
    }

    #[test]
    fn tea_issue_view() {
        assert!(check("tea issue view 1"));
    }

    #[test]
    fn tea_pull_list() {
        assert!(check("tea pull list"));
    }

    #[test]
    fn tea_pr_view() {
        assert!(check("tea pr view 1"));
    }

    #[test]
    fn tea_release_list() {
        assert!(check("tea release list"));
    }

    #[test]
    fn tea_repo_bare() {
        assert!(check("tea repo"));
    }

    #[test]
    fn tea_repo_list() {
        assert!(check("tea repos list"));
    }

    #[test]
    fn tea_branch_list() {
        assert!(check("tea branch list"));
    }

    #[test]
    fn tea_label_list() {
        assert!(check("tea labels list"));
    }

    #[test]
    fn tea_milestone_list() {
        assert!(check("tea milestones list"));
    }

    #[test]
    fn tea_org_list() {
        assert!(check("tea org list"));
    }

    #[test]
    fn tea_notifications_bare() {
        assert!(check("tea notifications"));
    }

    #[test]
    fn tea_times_list() {
        assert!(check("tea times list"));
    }

    #[test]
    fn tea_whoami() {
        assert!(check("tea whoami"));
    }

    #[test]
    fn tea_version() {
        assert!(check("tea --version"));
    }

    #[test]
    fn tea_login_list() {
        assert!(check("tea login list"));
    }

    #[test]
    fn tea_logins_list() {
        assert!(check("tea logins list"));
    }

    #[test]
    fn tea_issue_create_denied() {
        assert!(!check("tea issue create --title test"));
    }

    #[test]
    fn tea_pull_create_denied() {
        assert!(!check("tea pull create"));
    }

    #[test]
    fn tea_login_add_denied() {
        assert!(!check("tea login add"));
    }

    #[test]
    fn tea_logout_denied() {
        assert!(!check("tea logout"));
    }

    #[test]
    fn bare_tea_denied() {
        assert!(!check("tea"));
    }
}
