//! GitHub CLI dispatch. All flag-policy data and the sub × action
//! allowlists live in `commands/forges/gh.toml`, including the `gh api`
//! flag, header and field sets and the flags naming where
//! `gh release download` writes. The handler is the sub × action matrix
//! routing plus the `gh api` walk (REST-vs-GraphQL routing, the
//! explicit-GET-for-fields rule, the file-reading field veto and the
//! GraphQL mutation veto) and the write-target gate on downloads.
use crate::parse::Token;
use crate::registry;
use crate::verdict::{SafetyLevel, Verdict};

fn policy(key: &str) -> (&'static [String], &'static [String]) {
    registry::handler_policy_flag_sets("gh", key).unwrap_or((&[], &[]))
}

fn listed(set: &[String], word: &str) -> bool {
    set.iter().any(|f| f == word)
}

fn flag_value_at<'a>(tokens: &'a [Token], i: usize, flags: &'static [String]) -> Option<(&'static str, &'a str, usize)> {
    let word = tokens[i].as_str();
    for flag in flags {
        if word == flag {
            return tokens.get(i + 1).map(|next| (flag.as_str(), next.as_str(), 2));
        }
        let Some(rest) = word.strip_prefix(flag.as_str()) else {
            continue;
        };
        if let Some(value) = rest.strip_prefix('=') {
            return Some((flag.as_str(), value, 1));
        }
        if !flag.starts_with("--") && !rest.is_empty() {
            return Some((flag.as_str(), rest, 1));
        }
    }
    None
}

fn is_safe_api_header(value: &str) -> bool {
    let Some((name, _)) = value.split_once(':') else {
        return false;
    };
    let name = name.trim();
    policy("api_header_names").0.iter().any(|allowed| allowed.eq_ignore_ascii_case(name))
}

fn names_graphql_mutation(query: &str) -> bool {
    let is_name_byte = |b: u8| b == b'_' || b.is_ascii_alphanumeric();
    let bytes = query.as_bytes();
    query.match_indices("mutation").any(|(at, word)| {
        let before = at.checked_sub(1).and_then(|p| bytes.get(p)).is_some_and(|&b| is_name_byte(b));
        let after = bytes.get(at + word.len()).is_some_and(|&b| is_name_byte(b));
        !before && !after
    })
}

pub fn is_safe_gh(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Denied;
    }
    if let Some(v @ Verdict::Allowed(_)) = registry::try_fallback_grammar("gh", tokens) {
        return v;
    }
    if tokens.len() == 3 && matches!(tokens[2].as_str(), "--help" | "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    if let Some(v) = registry::try_sub_dispatch("gh", tokens) {
        return v;
    }
    let verdict = registry::try_matrix_dispatch("gh", tokens).unwrap_or(Verdict::Denied);
    if verdict.is_allowed() && tokens[1] == "release" && tokens[2] == "download" {
        return verdict.combine(download_targets_verdict(&tokens[3..]));
    }
    verdict
}

fn download_targets_verdict(args: &[Token]) -> Verdict {
    let (_, targets) = policy("release_download_targets");
    let mut verdict = Verdict::Allowed(SafetyLevel::Inert);
    let mut i = 0;
    while i < args.len() {
        let Some((_, value, consumed)) = flag_value_at(args, i, targets) else {
            i += 1;
            continue;
        };
        if value != "-" {
            verdict = verdict.combine(crate::engine::resolve::write_target_verdict(value));
        }
        i += consumed;
    }
    verdict
}

/// Sub-handler for `gh api ...` / `glab api ...`. Called with tokens
/// in sub-dispatch convention: `tokens[0] = "api"`, `tokens[1] =
/// endpoint`, `tokens[2..] = args`. glab dispatches with
/// `super::gh::is_safe_gh_api(&tokens[1..])`.
pub fn is_safe_gh_api(tokens: &[Token]) -> Verdict {
    let graphql = tokens.get(1).is_some_and(|t| t == "graphql");
    let (standalone, valued) = policy("api");
    let (_, method_flags) = policy("api_method");
    let (_, header_flags) = policy("api_header");
    let (_, field_flags) = policy("api_fields");
    let (_, file_field_flags) = policy("api_file_fields");
    let mut has_fields = false;
    let mut has_explicit_get = false;
    let mut i = 1;
    while i < tokens.len() {
        let word = tokens[i].as_str();
        if let Some((_, method, consumed)) = flag_value_at(tokens, i, method_flags) {
            if !graphql && !method.eq_ignore_ascii_case("GET") {
                return Verdict::Denied;
            }
            has_explicit_get = true;
            i += consumed;
            continue;
        }
        if let Some((_, header, consumed)) = flag_value_at(tokens, i, header_flags) {
            if !is_safe_api_header(header) {
                return Verdict::Denied;
            }
            i += consumed;
            continue;
        }
        if let Some((flag, pair, consumed)) = flag_value_at(tokens, i, field_flags) {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            if listed(file_field_flags, flag) && value.starts_with('@') {
                return Verdict::Denied;
            }
            if graphql && key == "query" && names_graphql_mutation(value) {
                return Verdict::Denied;
            }
            has_fields = true;
            i += consumed;
            continue;
        }
        if word.starts_with('-') {
            if listed(standalone, word) {
                i += 1;
                continue;
            }
            if let Some((_, _, consumed)) = flag_value_at(tokens, i, valued) {
                i += consumed;
                continue;
            }
            return Verdict::Denied;
        }
        if i > 1 && word == "graphql" {
            return Verdict::Denied;
        }
        i += 1;
    }
    if !graphql && has_fields && !has_explicit_get {
        return Verdict::Denied;
    }
    Verdict::Allowed(SafetyLevel::Inert)
}

#[cfg(test)]
pub(super) const REGISTRY: &[crate::handlers::CommandEntry] = &[crate::handlers::CommandEntry::Paths {
    cmd: "gh",
    bare_ok: false,
    paths: &[
        "gh alias list", "gh attestation verify artifact", "gh auth status", "gh browse --no-browser", "gh cache list",
        "gh codespace list", "gh config list", "gh extension list", "gh gist list", "gh gist view 123", "gh gpg-key list", "gh issue list",
        "gh issue view 456", "gh label list", "gh org list", "gh pr list", "gh pr view 123", "gh pr diff 123", "gh pr checks 123",
        "gh pr status 123", "gh project list", "gh project view 1", "gh release list", "gh release view v1.0",
        "gh release download v1.0 --output -", "gh repo list", "gh repo view owner/repo", "gh ruleset list", "gh ruleset view 1",
        "gh run list", "gh run view 789", "gh run watch 123", "gh run rerun 12345", "gh search issues foo", "gh secret list",
        "gh ssh-key list", "gh status", "gh variable list", "gh workflow list", "gh workflow view ci", "gh api repos/o/r",
    ],
}];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        pr_view: "gh pr view 123",
        pr_view_json: "gh pr view 123 --json title,body",
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
        api_field_with_get: "gh api repos/o/r/contents/f -X GET -f ref=abc --jq '.content'",
        api_field_with_get_eq: "gh api repos/o/r/contents/f --method=GET -f ref=abc",
        api_raw_field_with_get: "gh api repos/o/r/contents/f -X GET -F ref=abc",
        api_graphql_query: "gh api graphql -f query='{viewer{login}}' --jq '.data.viewer.login'",
        api_graphql_field: "gh api graphql --field query='{repository(owner:\"o\",name:\"r\"){name}}'",
        api_graphql_jq: "gh api graphql -f query='{viewer{login}}' -q '.data'",
        api_graphql_named_query: "gh api graphql -f query='query GetViewer{viewer{login}}'",
        api_graphql_with_vars: "gh api graphql -f query='{repository(owner:\"o\",name:\"r\"){name}}' -F owner=o",
        api_graphql_paginate: "gh api graphql --paginate -f query='{viewer{repositories(first:100){nodes{name}}}}'",
        api_raw_field_at_literal: "gh api -X GET search/issues -f q=@octocat",
        api_graphql_mutation_inside_a_name: "gh api graphql -f query='{viewer{mutationCount: login}}'",
        release_download_worktree_output: "gh release download v1.0 -p '*.tar.gz' -O dist/app.tar.gz",
        release_download_worktree_dir: "gh release download v1.0 -O - -D dist",
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
        api_field_no_get_denied: "gh api repos/o/r/issues -f title=x",
        api_raw_field_no_get_denied: "gh api repos/o/r/issues --raw-field body=x",
        api_big_field_no_get_denied: "gh api repos/o/r/issues -F title=x",
        api_field_with_post_denied: "gh api repos/o/r/issues -X POST -f title=x",
        api_field_with_patch_denied: "gh api repos/o/r/issues -X PATCH -f title=x",
        api_graphql_mutation_denied: "gh api graphql -f query=mutation",
        api_graphql_mutation_brace_denied: "gh api graphql -f query='mutation{addStar}'",
        api_graphql_mutation_named_denied: "gh api graphql -f query='mutation AddStar{addStar}'",
        api_graphql_mutation_whitespace_denied: "gh api graphql -f query='  mutation{addStar}'",
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
        api_field_file_denied: "gh api -X GET search/code -F q=@/etc/passwd",
        api_field_stdin_denied: "gh api -X GET search/code -F q=@-",
        api_field_eq_file_denied: "gh api -X GET search/code --field=q=@notes.txt",
        api_field_attached_file_denied: "gh api -X GET search/code -Fq=@notes.txt",
        api_field_short_eq_file_denied: "gh api -X GET search/code -F=q=@notes.txt",
        api_graphql_query_file_denied: "gh api graphql -F query=@q.graphql",
        api_graphql_short_eq_mutation_denied: "gh api graphql -F=query='mutation{addStar}'",
        api_graphql_raw_short_eq_mutation_denied: "gh api graphql -f=query='mutation{addStar}'",
        api_graphql_fragment_then_mutation_denied: "gh api graphql -f query='fragment F on User{login} mutation{addStar}'",
        api_graphql_second_operation_mutation_denied: "gh api graphql -f query='query A{viewer{login}} mutation B{addStar}' -f operationName=B",
        api_graphql_endpoint_after_flags_denied: "gh api -X GET graphql -f query='mutation{addStar}'",
        api_json_not_an_api_flag_denied: "gh api repos/o/r/pulls --json title",
        release_download_home_denied: "gh release download v1.0 -O ~/.bashrc --clobber",
        release_download_hook_denied: "gh release download v1.0 -O .git/hooks/pre-commit",
        release_download_output_eq_denied: "gh release download v1.0 --output=$HOME/.zshrc",
        release_download_dir_config_denied: "gh release download v1.0 -O - --dir ~/.config/fish",
        pr_list_web_denied: "gh pr list --web",
        pr_list_w_denied: "gh pr list -w",
        repo_view_web_denied: "gh repo view --web",
        search_web_denied: "gh search issues bug --web",
    }

    #[test]
    fn no_form_that_opens_a_browser_approves() {
        let mut forms: Vec<String> = Vec::new();
        for parent in [
            "alias", "attestation", "cache", "codespace", "config", "extension", "gist", "gpg-key", "issue", "label", "org", "pr",
            "project", "repo", "ruleset", "secret", "ssh-key", "variable", "workflow", "run", "release",
        ] {
            for action in ["checks", "diff", "list", "status", "verify", "view", "watch"] {
                forms.push(format!("gh {parent} {action}"));
                forms.push(format!("gh {parent} {action} 123"));
            }
        }
        for search in ["code", "commits", "issues", "prs", "repos"] {
            forms.push(format!("gh search {search} x"));
        }
        forms.push("gh status".into());
        forms.push("gh auth status".into());
        for parent in ["mr", "issue", "ci", "release", "repo", "snippet", "label", "milestone"] {
            for action in ["list", "view", "status", "diff"] {
                forms.push(format!("glab {parent} {action} 123"));
            }
        }
        for form in &forms {
            for flag in ["--web", "-w"] {
                if flag == "-w" && form.starts_with("gh run list") {
                    continue;
                }
                let cmd = format!("{form} {flag}");
                assert!(!check(&cmd), "{cmd} was approved");
            }
        }
        assert!(check("gh pr view 123"), "the plain form must still approve");
        assert!(check("gh run list -w ci.yml"), "-w names a workflow on run list");
    }

    proptest::proptest! {
        #[test]
        fn a_typed_field_reading_a_file_never_approves(
            key in "[a-z_]{1,8}",
            path in "[A-Za-z0-9_./~-]{0,20}",
            form in 0usize..5,
            graphql in proptest::bool::ANY,
        ) {
            let pair = format!("{key}=@{path}");
            let field = match form {
                0 => format!("-F '{pair}'"),
                1 => format!("--field '{pair}'"),
                2 => format!("'--field={pair}'"),
                3 => format!("'-F{pair}'"),
                _ => format!("'-F={pair}'"),
            };
            let cmd = if graphql {
                format!("gh api graphql -f query='{{viewer{{login}}}}' {field}")
            } else {
                format!("gh api -X GET search/code {field}")
            };
            proptest::prop_assert!(!check(&cmd), "{cmd}");
        }

        #[test]
        fn a_graphql_query_naming_mutation_anywhere_never_approves(
            before in "[a-z{}() ]{0,16}",
            after in "[a-z{}() ]{0,16}",
            left in "[ {}(),\n]",
            right in "[ {}(),\n]",
            form in 0usize..6,
        ) {
            let query = format!("{before}{left}mutation{right}{after}");
            let field = match form {
                0 => format!("-f 'query={query}'"),
                1 => format!("-F 'query={query}'"),
                2 => format!("'--raw-field=query={query}'"),
                3 => format!("'-f=query={query}'"),
                4 => format!("'-fquery={query}'"),
                _ => format!("--field 'query={query}'"),
            };
            let cmd = format!("gh api graphql {field}");
            proptest::prop_assert!(!check(&cmd), "{cmd}");
        }

        #[test]
        fn a_download_into_a_home_dotfile_never_approves(
            name in "[a-z]{1,10}",
            form in 0usize..4,
        ) {
            let target = format!("~/.{name}");
            let flag = match form {
                0 => format!("-O {target}"),
                1 => format!("--output={target}"),
                2 => format!("-O - -D {target}"),
                _ => format!("-O - --dir={target}"),
            };
            let cmd = format!("gh release download v1.0 {flag}");
            proptest::prop_assert!(!check(&cmd), "{cmd}");
        }
    }
}
