use crate::parse::{Token, WordSet, has_flag};
use crate::verdict::{SafetyLevel, Verdict};
use crate::policy::{self, FlagPolicy, FlagStyle};

static GH_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--archived", "--comments", "--draft",
        "--fork", "--no-archived", "--source", "--web",
        "-a", "-w",
    ]),
    valued: WordSet::flags(&[
        "--app", "--assignee", "--author", "--base",
        "--env", "--head", "--jq", "--json",
        "--key", "--label", "--language", "--limit",
        "--mention", "--milestone", "--order", "--org",
        "--ref", "--repo", "--search", "--sort",
        "--state", "--template", "--topic", "--user", "--visibility",
        "-B", "-H", "-L", "-O", "-R", "-S",
        "-e", "-k", "-l", "-o", "-q", "-r", "-u",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_VIEW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--comments", "--web", "--yaml",
        "-c", "-w", "-y",
    ]),
    valued: WordSet::flags(&[
        "--branch", "--jq", "--json", "--ref", "--repo", "--template",
        "-R", "-b", "-q", "-r",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_DIFF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--name-only", "--patch", "--web",
        "-w",
    ]),
    valued: WordSet::flags(&[
        "--color", "--repo",
        "-R",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_CHECKS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--fail-fast", "--required", "--watch", "--web",
        "-w",
    ]),
    valued: WordSet::flags(&[
        "--interval", "--jq", "--json", "--repo", "--template",
        "-R", "-i", "-q",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_STATUS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--exit-status", "--log", "--log-failed", "--web",
        "-w",
    ]),
    valued: WordSet::flags(&[
        "--jq", "--json", "--repo", "--template",
        "-R", "-q",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_RUN_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[
        "--branch", "--commit", "--created", "--event",
        "--jq", "--json", "--limit", "--repo",
        "--status", "--template", "--user", "--workflow",
        "-L", "-R", "-b", "-q", "-u", "-w",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_RUN_VIEW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--exit-status", "--log", "--log-failed", "--verbose", "--web",
        "-v", "-w",
    ]),
    valued: WordSet::flags(&[
        "--attempt", "--job", "--jq", "--json", "--repo", "--template",
        "-R", "-j", "-q",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_RUN_WATCH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--exit-status",
    ]),
    valued: WordSet::flags(&[
        "--interval", "--repo",
        "-R", "-i",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_RELEASE_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--exclude-drafts", "--exclude-pre-releases",
    ]),
    valued: WordSet::flags(&[
        "--jq", "--json", "--limit", "--order", "--repo", "--template",
        "-L", "-R", "-q",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_RELEASE_VIEW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--web",
        "-w",
    ]),
    valued: WordSet::flags(&[
        "--jq", "--json", "--repo", "--template",
        "-R", "-q",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_SEARCH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--archived", "--include-forks", "--web",
        "-w",
    ]),
    valued: WordSet::flags(&[
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
        "-L", "-R", "-q",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_RELEASE_DOWNLOAD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--clobber", "--skip-existing"]),
    valued: WordSet::flags(&[
        "--archive", "--dir", "--output", "--pattern", "--repo",
        "-A", "-D", "-O", "-R", "-p",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_SIMPLE_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--archived", "--fork", "--no-archived", "--source", "--web",
        "-a", "-w",
    ]),
    valued: WordSet::flags(&[
        "--env", "--jq", "--json", "--key", "--language", "--limit",
        "--order", "--org", "--ref", "--repo", "--search",
        "--sort", "--template", "--topic", "--user", "--visibility",
        "-L", "-O", "-R", "-S", "-e", "-k", "-l", "-o", "-q", "-r", "-u",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_SIMPLE_VIEW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--web", "--yaml", "-w", "-y"]),
    valued: WordSet::flags(&[
        "--jq", "--json", "--ref", "--repo", "--template",
        "-R", "-q", "-r",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static GH_RUN_RERUN_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--debug", "--failed"]),
    valued: WordSet::flags(&["--job", "--repo", "-R", "-j"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static READ_ONLY_SUBCOMMANDS: WordSet = WordSet::new(&[
    "alias", "attestation", "cache", "codespace", "config",
    "extension", "gist", "gpg-key",
    "issue", "label", "org", "pr", "project", "release",
    "repo", "ruleset", "run", "secret",
    "ssh-key", "variable", "workflow",
]);

static READ_ONLY_ACTIONS: WordSet =
    WordSet::new(&["checks", "diff", "list", "status", "verify", "view", "watch"]);

static ALWAYS_SAFE_SUBCOMMANDS: WordSet =
    WordSet::new(&["--version", "search", "status"]);

static GH_BROWSE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--actions", "--no-browser", "--projects",
        "--releases", "--settings", "--wiki",
        "-a", "-c", "-n", "-p", "-r", "-s", "-w",
    ]),
    valued: WordSet::flags(&["--branch", "--commit", "--repo", "-R", "-b"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static API_STANDALONE: WordSet = WordSet::new(&[
    "--include", "--paginate", "--silent", "--slurp", "--verbose",
    "-i",
]);

static API_VALUED: WordSet = WordSet::new(&[
    "--cache", "--hostname", "--jq", "--json", "--preview", "--template",
    "-p", "-q", "-t",
]);

fn is_safe_api_header(value: &str) -> bool {
    let Some((name, _)) = value.split_once(':') else {
        return false;
    };
    let trimmed = name.trim();
    trimmed.eq_ignore_ascii_case("Accept")
        || trimmed.eq_ignore_ascii_case("X-GitHub-Api-Version")
}

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
        "download" => &GH_RELEASE_DOWNLOAD_POLICY,
        "list" => &GH_RELEASE_LIST_POLICY,
        "view" => &GH_RELEASE_VIEW_POLICY,
        _ => &GH_SIMPLE_LIST_POLICY,
    }
}

pub fn is_safe_gh(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Denied;
    }
    if tokens.len() == 2 && matches!(tokens[1].as_str(), "--help" | "-h" | "--version" | "-V") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let subcmd = &tokens[1];

    if tokens.len() == 3 && (tokens[2] == "--help" || tokens[2] == "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }

    if subcmd == "search" {
        return if tokens.len() >= 3 && policy::check(&tokens[2..], &GH_SEARCH_POLICY) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }

    if subcmd == "status" {
        return if policy::check(&tokens[1..], &GH_SIMPLE_LIST_POLICY) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }

    if subcmd == "run" && tokens.len() >= 3 && tokens[2] == "rerun" {
        return if policy::check(&tokens[2..], &GH_RUN_RERUN_POLICY)
        { Verdict::Allowed(SafetyLevel::SafeWrite) }
        else { Verdict::Denied };
    }

    if subcmd == "release" && tokens.len() >= 3 && tokens[2] == "download" {
        return if has_flag(&tokens[2..], Some("-O"), Some("--output"))
            && policy::check(&tokens[2..], &GH_RELEASE_DOWNLOAD_POLICY) { Verdict::Allowed(SafetyLevel::SafeWrite) } else { Verdict::Denied };
    }

    if READ_ONLY_SUBCOMMANDS.contains(subcmd) {
        if tokens.len() < 3 || !READ_ONLY_ACTIONS.contains(&tokens[2]) {
            return Verdict::Denied;
        }
        let action = tokens[2].as_str();
        let policy = if subcmd == "run" {
            gh_run_action_policy(action)
        } else if subcmd == "release" {
            gh_release_action_policy(action)
        } else {
            gh_action_policy(action)
        };
        return if policy::check(&tokens[2..], policy) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }

    if subcmd == "auth" {
        if tokens.len() < 3 {
            return Verdict::Denied;
        }
        if tokens[2] == "status" {
            return if policy::check(&tokens[2..], &GH_SIMPLE_LIST_POLICY) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
        }
        return Verdict::Denied;
    }

    if subcmd == "browse" {
        return if has_flag(&tokens[1..], Some("-n"), Some("--no-browser"))
            && policy::check(&tokens[1..], &GH_BROWSE_POLICY)
        { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }

    if subcmd == "api" {
        return is_safe_gh_api(tokens);
    }

    Verdict::Denied

}

pub(in crate::handlers::forges) fn is_safe_gh_api(tokens: &[Token]) -> Verdict {
    let mut i = 2;
    while i < tokens.len() {
        let token = &tokens[i];

        if token == "-X" || token == "--method" {
            return if tokens
                .get(i + 1)
                .is_some_and(|m| m.eq_ignore_ascii_case("GET")) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
        }
        if token.starts_with("-X") && token.len() > 2 && !token.starts_with("-X=") {
            return if token
                .get(2..)
                .is_some_and(|s| s.eq_ignore_ascii_case("GET"))
            { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
        }
        if token.starts_with("-X=") || token.starts_with("--method=") {
            let val = token.split_value("=").unwrap_or("");
            return if val.eq_ignore_ascii_case("GET") { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
        }

        if token == "-H" || token == "--header" {
            match tokens.get(i + 1) {
                Some(val) if is_safe_api_header(val.as_str()) => {
                    i += 2;
                    continue;
                }
                _ => return Verdict::Denied,
            }
        }
        if let Some(rest) = token.as_str().strip_prefix("-H=").or_else(|| token.as_str().strip_prefix("--header=")) {
            if is_safe_api_header(rest) {
                i += 1;
                continue;
            }
            return Verdict::Denied;
        }

        if token.starts_with('-') {
            if API_STANDALONE.contains(token) {
                i += 1;
                continue;
            }
            if API_VALUED.contains(token) {
                i += 2;
                continue;
            }
            if let Some((_flag, _val)) = token.split_once('=') {
                let flag_part = Token::from_raw(_flag.to_string());
                if API_VALUED.contains(&flag_part) {
                    i += 1;
                    continue;
                }
            }
            return Verdict::Denied;
        }

        i += 1;
    }
    Verdict::Allowed(SafetyLevel::Inert)
}

pub(in crate::handlers::forges) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "gh" => Some(is_safe_gh(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocBuilder, wordset_items};
    vec![
        CommandDoc::handler("gh",
            "https://cli.github.com/manual/",
            DocBuilder::new()
                .section(format!("Subcommands {} are allowed with actions: {}.",
                    wordset_items(&READ_ONLY_SUBCOMMANDS),
                    wordset_items(&READ_ONLY_ACTIONS)))
                .section(format!("Always safe: {}.",
                    wordset_items(&ALWAYS_SAFE_SUBCOMMANDS)))
                .section("auth status, browse (requires --no-browser), \
                          run rerun (SafeWrite), \
                          release download (requires --output), \
                          api (read-only: implicit GET or explicit -X GET, \
                          with --paginate, --slurp, --jq, --template, \
                          --cache, --preview, --include, --silent, --verbose, --hostname, \
                          -H for Accept and X-GitHub-Api-Version headers).")
                .section("")
                .build()),
    ]
}

#[cfg(test)]
pub(super) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Paths { cmd: "gh", bare_ok: false, paths: &[
        "gh alias list",
        "gh attestation verify artifact",
        "gh auth status",
        "gh browse --no-browser",
        "gh cache list",
        "gh codespace list",
        "gh config list",
        "gh extension list",
        "gh gist list",
        "gh gist view 123",
        "gh gpg-key list",
        "gh issue list",
        "gh issue view 456",
        "gh label list",
        "gh org list",
        "gh pr list",
        "gh pr view 123",
        "gh pr diff 123",
        "gh pr checks 123",
        "gh pr status 123",
        "gh project list",
        "gh project view 1",
        "gh release list",
        "gh release view v1.0",
        "gh release download v1.0 --output -",
        "gh repo list",
        "gh repo view owner/repo",
        "gh ruleset list",
        "gh ruleset view 1",
        "gh run list",
        "gh run view 789",
        "gh run watch 123",
        "gh run rerun 12345",
        "gh search issues foo",
        "gh secret list",
        "gh ssh-key list",
        "gh status",
        "gh variable list",
        "gh workflow list",
        "gh workflow view ci",
        "gh api repos/o/r",
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
        run_view_json_jq: "gh run view 789 --json status,conclusion -q '.status'",
        run_view_json_jq_repo: "gh run view 789 --repo owner/repo --json status,conclusion -q '.status'",
        pr_list_jq_short: "gh pr list --json number -q '.[].number'",
        issue_list_jq_short: "gh issue list -q '.[] | .title' --json title",
        pr_help: "gh pr --help",
        issue_help: "gh issue --help",
        run_help: "gh run --help",
        search_help: "gh search --help",
        auth_help: "gh auth --help",
        browse_help: "gh browse --help",
        api_help: "gh api --help",
        release_help: "gh release --help",
        run_watch: "gh run watch 123",
        run_watch_repo: "gh run watch 123 --repo owner/repo",
        run_watch_exit: "gh run watch 123 --exit-status",
        run_rerun: "gh run rerun 12345",
        run_rerun_failed: "gh run rerun 12345 --failed",
        run_rerun_debug: "gh run rerun 12345 --debug",
        run_rerun_job: "gh run rerun 12345 --job job-id",
        run_rerun_repo: "gh run rerun 12345 --repo owner/repo",
        run_list: "gh run list",
        run_list_workflow: "gh run list --workflow ci.yml",
        run_list_branch: "gh run list --branch main",
        run_list_status: "gh run list --status completed",
        release_list: "gh release list",
        release_list_limit: "gh release list --limit 10",
        release_view: "gh release view v1.0",
        release_view_web: "gh release view v1.0 --web",
        release_download_stdout: "gh release download v1.0 --output -",
        release_download_pattern: "gh release download v1.0 --repo o/r --pattern 'SHA256SUMS.txt' --output -",
        release_download_short: "gh release download v1.0 -O - -p '*.tar.gz'",
        release_download_archive: "gh release download v1.0 --archive tar.gz --output out.tar.gz",
        release_download_dir: "gh release download v1.0 --output /tmp/out --dir /tmp",
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
        api_paginate_slurp: "gh api repos/o/r/pulls --paginate --slurp",
        api_paginate_jq: "gh api repos/o/r/pulls --paginate --jq '.[].title'",
        api_xget_short: "gh api repos/o/r/pulls -XGET",
        api_include: "gh api repos/o/r/pulls -i",
        api_silent: "gh api repos/o/r/pulls --silent",
        api_verbose: "gh api repos/o/r/pulls --verbose",
        api_cache: "gh api repos/o/r/pulls --cache 3600s",
        api_hostname: "gh api repos/o/r/pulls --hostname github.example.com",
        api_template: "gh api repos/o/r/pulls -t '{{.title}}'",
        api_preview: "gh api repos/o/r/pulls -p corsair",
        api_method_eq_get: "gh api repos/o/r/pulls --method=GET",
        api_combined: "gh api repos/o/r/pulls --paginate --slurp --jq '.[].title' --cache 60s",
        api_header_accept_raw: "gh api repos/o/r/contents/f?ref=branch -H 'Accept: application/vnd.github.raw'",
        api_header_accept_json: "gh api repos/o/r/contents/f -H 'Accept: application/vnd.github.v3+json'",
        api_header_api_version: "gh api repos/o/r/pulls -H 'X-GitHub-Api-Version: 2022-11-28'",
        api_header_long_form: "gh api repos/o/r/contents/f --header 'Accept: application/vnd.github.raw'",
        api_header_multiple: "gh api repos/o/r/contents/f -H 'Accept: application/vnd.github.raw' -H 'X-GitHub-Api-Version: 2022-11-28'",
        api_header_with_jq: "gh api repos/o/r/contents/f -H 'Accept: application/vnd.github.raw' --jq '.content'",
        gh_version: "gh --version",
        gist_list: "gh gist list",
        gist_view: "gh gist view abc123",
        org_list: "gh org list",
        project_list: "gh project list",
        project_view: "gh project view 1",
        ruleset_list: "gh ruleset list",
        ruleset_view: "gh ruleset view 1",
        config_list: "gh config list",
        alias_list: "gh alias list",
        secret_list: "gh secret list",
        issue_list_mention: "gh issue list --mention user",
        label_list_search: "gh label list --search bug",
        label_list_sort: "gh label list --sort name --order asc",
        cache_list_key: "gh cache list --key prefix",
        repo_list_language: "gh repo list --language rust",
        repo_list_archived: "gh repo list --archived",
        repo_view_branch: "gh repo view owner/repo --branch dev",
        workflow_list_all: "gh workflow list --all",
        workflow_view_yaml: "gh workflow view ci --yaml",
        workflow_view_ref: "gh workflow view ci --ref main",
        codespace_list_repo: "gh codespace list --repo owner/repo",
        variable_list_env: "gh variable list --env production",
        variable_list_org: "gh variable list --org myorg",
    }

    denied! {
        run_rerun_bare_denied: "gh run rerun",
        run_rerun_unknown_flag_denied: "gh run rerun 12345 --unknown",
        browse_without_flag_denied: "gh browse",
        issue_download_denied: "gh issue download",
        release_download_no_output_denied: "gh release download v1.0",
        release_download_pattern_no_output_denied: "gh release download v1.0 --pattern '*.tar.gz'",
        api_patch_denied: "gh api repos/o/r/pulls/1 -X PATCH -f body=x",
        api_post_denied: "gh api repos/o/r/pulls/1 -X POST",
        api_field_denied: "gh api repos/o/r/issues -f title=x",
        api_raw_field_denied: "gh api repos/o/r/issues --raw-field body=x",
        api_big_field_denied: "gh api repos/o/r/issues -F title=x",
        api_input_denied: "gh api repos/o/r/rulesets --input file.json",
        api_header_authorization_denied: "gh api repos/o/r/pulls -H 'Authorization: token ghp_xxx'",
        api_header_content_type_denied: "gh api repos/o/r/pulls -H 'Content-Type: application/json'",
        api_header_long_auth_denied: "gh api repos/o/r/pulls --header 'Authorization: Bearer xxx'",
        api_header_missing_value_denied: "gh api repos/o/r/pulls -H",
        api_header_no_colon_denied: "gh api repos/o/r/pulls -H 'Accept'",
        api_header_compact_denied: "gh api repos/o/r/pulls -HAuthorization:token",
        api_method_eq_patch_denied: "gh api repos/o/r/pulls/1 --method=PATCH",
        api_xpost_short_denied: "gh api repos/o/r/pulls -XPOST",
        api_xpatch_short_denied: "gh api repos/o/r/pulls -XPATCH",
        api_unknown_flag_denied: "gh api repos/o/r/pulls --some-unknown-flag",
    }
}
