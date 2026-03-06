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

pub(in crate::handlers::forges) static API_BODY_FLAGS: &[&str] = &["-f", "-F", "--field", "--raw-field", "--input"];

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

pub(in crate::handlers::forges) fn is_safe_gh_api(tokens: &[Token]) -> bool {
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

pub(in crate::handlers::forges) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "gh" => Some(is_safe_gh(tokens)),
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
    ]
}

#[cfg(test)]
pub(super) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Subcommand { cmd: "gh", subs: &[
        crate::handlers::SubEntry::Nested { name: "issue", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "pr", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
            crate::handlers::SubEntry::Policy { name: "diff" },
            crate::handlers::SubEntry::Policy { name: "checks" },
            crate::handlers::SubEntry::Policy { name: "status" },
        ]},
        crate::handlers::SubEntry::Nested { name: "run", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
            crate::handlers::SubEntry::Policy { name: "watch" },
        ]},
        crate::handlers::SubEntry::Nested { name: "release", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "label", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "cache", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "codespace", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "variable", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "extension", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "attestation", subs: &[
            crate::handlers::SubEntry::Policy { name: "verify" },
        ]},
        crate::handlers::SubEntry::Nested { name: "gpg-key", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "ssh-key", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
        ]},
        crate::handlers::SubEntry::Nested { name: "workflow", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Nested { name: "repo", subs: &[
            crate::handlers::SubEntry::Policy { name: "list" },
            crate::handlers::SubEntry::Policy { name: "view" },
        ]},
        crate::handlers::SubEntry::Custom { name: "search", valid_suffix: Some("issues foo") },
        crate::handlers::SubEntry::Policy { name: "status" },
        crate::handlers::SubEntry::Nested { name: "auth", subs: &[
            crate::handlers::SubEntry::Policy { name: "status" },
        ]},
        crate::handlers::SubEntry::Custom { name: "browse", valid_suffix: Some("--no-browser") },
        crate::handlers::SubEntry::Positional { name: "api" },
    ]},
];

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
    }

    denied! {
        browse_without_flag_denied: "gh browse",
        api_patch_denied: "gh api repos/o/r/pulls/1 -X PATCH -f body=x",
        api_post_denied: "gh api repos/o/r/pulls/1 -X POST",
        api_field_denied: "gh api repos/o/r/issues -f title=x",
        api_method_eq_patch_denied: "gh api repos/o/r/pulls/1 --method=PATCH",
        api_xpost_short_denied: "gh api repos/o/r/pulls -XPOST",
        api_xpatch_short_denied: "gh api repos/o/r/pulls -XPATCH",
    }
}
