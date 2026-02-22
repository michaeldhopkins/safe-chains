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

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocKind};
    vec![CommandDoc {
        name: "gh",
        kind: DocKind::Handler,
        description: "Read-only subcommands (view/list/status/diff/checks/verify): pr, issue, repo, release, run, workflow, label, codespace, variable, extension, cache, attestation, gpg-key, ssh-key. \
                      Always safe: search, status. \
                      Guarded: auth (status/token only), browse (requires --no-browser), api (GET only, no body flags).",
    }]
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
}
