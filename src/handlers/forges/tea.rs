use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};
use crate::policy::{self, FlagPolicy, FlagStyle};

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
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[
        "--fields", "--limit", "--login", "--output",
        "--page", "--repo", "--state",
        "-L", "-R", "-f", "-l", "-o", "-p", "-s",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static TEA_VIEW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--comments", "--help",
        "-c", "-h",
    ]),
    valued: WordSet::flags(&[
        "--login", "--output", "--repo",
        "-R", "-l", "-o",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_tea(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Denied;
    }
    let subcmd = &tokens[1];

    if TEA_READ_ONLY_SUBCOMMANDS.contains(subcmd) {
        if tokens.len() == 2 {
            return Verdict::Allowed(SafetyLevel::Inert);
        }
        if !TEA_READ_ONLY_ACTIONS.contains(&tokens[2]) {
            return Verdict::Denied;
        }
        let policy = if tokens[2] == "view" {
            &TEA_VIEW_POLICY
        } else {
            &TEA_LIST_POLICY
        };
        return if policy::check(&tokens[2..], policy) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }

    if TEA_ALWAYS_SAFE.contains(subcmd) {
        return if tokens.len() == 2 { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }

    if subcmd == "logins" || subcmd == "login" {
        if tokens.len() < 3 || !TEA_LOGIN_SAFE.contains(&tokens[2]) {
            return Verdict::Denied;
        }
        return if policy::check(&tokens[2..], &TEA_LIST_POLICY) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }

    Verdict::Denied

}

pub(in crate::handlers::forges) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "tea" => Some(is_safe_tea(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocBuilder, wordset_items};
    vec![
        CommandDoc::handler("tea",
            "https://gitea.com/gitea/tea",
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
pub(super) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Subcommand { cmd: "tea", bare_ok: false, subs: &[
        crate::handlers::SubEntry::Nested { name: "issue", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "issues", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "i", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "pr", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "pull", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "pulls", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "release", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "releases", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "r", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "repo", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "repos", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "branch", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "branches", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "b", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "label", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "labels", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "milestone", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "milestones", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "ms", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "org", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "organization", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "organizations", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "notification", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "notifications", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "n", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "time", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "times", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "t", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "login", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "logins", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
    ]},
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
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
        tea_whoami_with_extra_denied: "tea whoami --extra",
    }
}
