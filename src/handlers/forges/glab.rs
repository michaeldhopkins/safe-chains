//! GitLab CLI dispatch. All flag-policy data and sub × action
//! allowlists live in `commands/forges/glab.toml`. The handler is the
//! matrix routing logic plus the always-safe-bare-subs check and
//! delegation to gh's API sub-handler for `glab api`.
use crate::parse::Token;
use crate::registry;
use crate::verdict::Verdict;

pub fn is_safe_glab(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Denied;
    }
    if let Some(v @ Verdict::Allowed(_)) = registry::try_fallback_grammar("glab", tokens) {
        return v;
    }
    if let Some(v) = registry::try_sub_dispatch("glab", tokens) {
        return v;
    }
    registry::try_matrix_dispatch("glab", tokens).unwrap_or(Verdict::Denied)
}

pub(in crate::handlers::forges) fn dispatch(_cmd: &str, _tokens: &[Token]) -> Option<Verdict> {
    // glab is dispatched through the TOML registry now
    // (handler = "glab" in commands/forges/glab.toml).
    None
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    // glab's docs come from the TOML registry's auto-render.
    Vec::new()
}

#[cfg(test)]
pub(super) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Paths { cmd: "glab", bare_ok: false, paths: &[
        "glab mr list",
        "glab mr view 123",
        "glab mr diff 123",
        "glab issue list",
        "glab issue view 456",
        "glab ci list",
        "glab ci status",
        "glab release list",
        "glab label list",
        "glab milestone list",
        "glab snippet view 1",
        "glab variable list",
        "glab repo list",
        "glab repo view owner/repo",
        "glab cluster list",
        "glab deploy-key list",
        "glab gpg-key list",
        "glab incident list",
        "glab iteration list",
        "glab schedule list",
        "glab ssh-key list",
        "glab stack list",
        "glab auth status",
        "glab api projects/1/merge_requests",
    ]},
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
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
    }

    denied! {
        glab_api_post_denied: "glab api projects/1/issues -X POST",
        glab_api_field_denied: "glab api projects/1/issues -f title=x",
        glab_version_with_extra_denied: "glab version --extra",
        glab_check_update_with_extra_denied: "glab check-update --extra",
    }
}
