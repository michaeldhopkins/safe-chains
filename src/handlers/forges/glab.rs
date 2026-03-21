use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};
use crate::policy::{self, FlagPolicy, FlagStyle};

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
    standalone: WordSet::flags(&[
        "--all", "--closed", "--draft", "--help", "--merged",
        "-A", "-M", "-a", "-c", "-d", "-g", "-h", "-q",
    ]),
    valued: WordSet::flags(&[
        "--assignee", "--author", "--group", "--label",
        "--milestone", "--not-label", "--order", "--output",
        "--page", "--per-page", "--repo", "--reviewer",
        "--search", "--sort", "--source-branch", "--state",
        "--target-branch",
        "-F", "-P", "-R", "-S", "-a", "-g", "-l", "-m", "-o", "-p", "-r", "-s", "-t",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GLAB_VIEW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--comments", "--help", "--resolved", "--system-logs",
        "--unresolved", "--web",
        "-c", "-h", "-p", "-s", "-w",
    ]),
    valued: WordSet::flags(&[
        "--output", "--page", "--per-page", "--repo",
        "-F", "-P", "-R", "-p",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GLAB_DIFF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--raw",
        "-h",
    ]),
    valued: WordSet::flags(&[
        "--color", "--repo",
        "-R",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GLAB_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help",
        "-h", "-q",
    ]),
    valued: WordSet::flags(&[
        "--output", "--page", "--per-page", "--repo",
        "-F", "-P", "-R", "-p",
    ]),
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

pub fn is_safe_glab(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Denied;
    }
    let subcmd = &tokens[1];

    if GLAB_READ_ONLY_SUBCOMMANDS.contains(subcmd) {
        if tokens.len() < 3 || !GLAB_READ_ONLY_ACTIONS.contains(&tokens[2]) {
            return Verdict::Denied;
        }
        let policy = glab_action_policy(tokens[2].as_str());
        return if policy::check(&tokens[2..], policy) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }

    if GLAB_ALWAYS_SAFE.contains(subcmd) {
        return if tokens.len() == 2 { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }

    if subcmd == "auth" {
        if tokens.len() < 3 || !GLAB_AUTH_SAFE.contains(&tokens[2]) {
            return Verdict::Denied;
        }
        return if policy::check(&tokens[2..], &GLAB_SIMPLE_POLICY) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }

    if subcmd == "api" {
        return super::gh::is_safe_gh_api(tokens);
    }

    Verdict::Denied

}

pub(in crate::handlers::forges) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "glab" => Some(is_safe_glab(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocBuilder, wordset_items};
    vec![
        CommandDoc::handler("glab",
            "https://glab.readthedocs.io/en/latest/",
            DocBuilder::new()
                .section(format!("Subcommands {} are allowed with actions: {}.",
                    wordset_items(&GLAB_READ_ONLY_SUBCOMMANDS),
                    wordset_items(&GLAB_READ_ONLY_ACTIONS)))
                .section(format!("Always safe: {}.",
                    wordset_items(&GLAB_ALWAYS_SAFE)))
                .section("auth status, api (GET only).")
                .section("")
                .build()),
    ]
}

#[cfg(test)]
pub(super) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Subcommand { cmd: "glab", bare_ok: false, subs: &[
        crate::handlers::SubEntry::Nested { name: "mr", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
            crate::handlers::SubEntry::Policy { name: "diff" },
        ]},
        crate::handlers::SubEntry::Nested { name: "issue", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "ci", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "status" },
        ]},
        crate::handlers::SubEntry::Nested { name: "release", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "label", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "milestone", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "snippet", subs: &[
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "variable", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "repo", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "cluster", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "deploy-key", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "gpg-key", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "incident", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "iteration", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "schedule", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "ssh-key", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "stack", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "auth", subs: &[
            crate::handlers::SubEntry::Policy { name: "status" },
        ]},
        crate::handlers::SubEntry::Positional,
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
