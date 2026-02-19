use std::collections::HashSet;
use std::sync::LazyLock;

static READ_ONLY_SUBCOMMANDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "pr",
        "issue",
        "repo",
        "release",
        "run",
        "workflow",
        "label",
        "codespace",
        "variable",
        "extension",
        "cache",
        "attestation",
        "gpg-key",
        "ssh-key",
    ])
});

static READ_ONLY_ACTIONS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["view", "list", "status", "diff", "checks", "verify"]));

static ALWAYS_SAFE_SUBCOMMANDS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["search", "status"]));

static AUTH_SAFE_ACTIONS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["status", "token"]));

static API_BODY_FLAGS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["-f", "-F", "--field", "--raw-field", "--input"]));

pub fn is_safe_gh(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let subcmd = &tokens[1];

    if READ_ONLY_SUBCOMMANDS.contains(subcmd.as_str()) {
        return tokens.len() >= 3 && READ_ONLY_ACTIONS.contains(tokens[2].as_str());
    }

    if ALWAYS_SAFE_SUBCOMMANDS.contains(subcmd.as_str()) {
        return true;
    }

    if subcmd == "auth" {
        return tokens.len() >= 3 && AUTH_SAFE_ACTIONS.contains(tokens[2].as_str());
    }

    if subcmd == "browse" {
        return tokens[2..].iter().any(|t| t == "--no-browser");
    }

    if subcmd == "api" {
        return is_safe_gh_api(tokens);
    }

    false
}

fn is_safe_gh_api(tokens: &[String]) -> bool {
    for (i, token) in tokens[2..].iter().enumerate() {
        let abs_i = i + 2;

        if token == "-X" || token == "--method" {
            return tokens
                .get(abs_i + 1)
                .is_some_and(|m| m.eq_ignore_ascii_case("GET"));
        }
        if token.starts_with("-X") && token.len() > 2 && !token.starts_with("-X=") {
            return token[2..].eq_ignore_ascii_case("GET");
        }
        if token.starts_with("-X=") || token.starts_with("--method=") {
            let val = token.split_once('=').map(|(_, v)| v).unwrap_or("");
            return val.eq_ignore_ascii_case("GET");
        }

        for flag in API_BODY_FLAGS.iter() {
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

#[cfg(test)]
mod tests {
    use crate::is_safe;

    fn check(cmd: &str) -> bool {
        is_safe(cmd)
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
    fn bare_gh_denied() {
        assert!(!check("gh"));
    }
}
