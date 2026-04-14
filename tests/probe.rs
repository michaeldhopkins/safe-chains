#![allow(clippy::unwrap_used)]

use safe_chains::is_safe_command;

fn check(cmd: &str) -> bool {
    is_safe_command(cmd)
}

#[test]
fn env_prefix_quoted_safe_command() {
    assert!(check("FOO='bar baz' ls -la"));
}

#[test]
fn env_prefix_quoted_unsafe_denied() {
    assert!(!check("FOO='bar baz' rm -rf /"));
}

#[test]
fn awk_dev_null_redirect() {
    assert!(check("awk '{print $1}' file.txt > /dev/null"));
}

#[test]
fn sed_dev_null_redirect() {
    assert!(check("sed 's/foo/bar/' file.txt > /dev/null"));
}

#[test]
fn grep_dev_null_redirect() {
    assert!(check("grep pattern file > /dev/null"));
}

#[test]
fn git_log_dev_null_redirect() {
    assert!(check("git log > /dev/null"));
}

#[test]
fn echo_dev_null_redirect() {
    assert!(check("echo hello > /dev/null"));
}

#[test]
fn echo_stderr_dev_null() {
    assert!(check("echo hello 2> /dev/null"));
}

#[test]
fn multi_digit_fd_redirect() {
    assert!(check("ls 10>&1"));
    assert!(check("cargo clippy 255>&2"));
}

#[test]
fn multi_digit_dev_null_redirect() {
    assert!(check("echo hello 10>/dev/null"));
    assert!(check("ls 255>/dev/null"));
}

#[test]
fn numeric_arg_not_swallowed_by_redirect_filter() {
    assert!(check("head -n 42 /dev/null"));
    assert!(check("head -42 /dev/null"));
    assert!(check("tail -100 /dev/null"));
}

#[test]
fn command_with_path_traversal() {
    assert!(!check("/usr/bin/../../../etc/shadow"));
}

#[test]
fn command_with_simple_path() {
    assert!(check("/usr/bin/ls -la"));
}

#[test]
fn has_flag_unicode_no_panic() {
    let _result = check("sed -é 's/foo/bar/'");
}

#[test]
fn heredoc_safe() {
    assert!(check("cat << EOF"));
    assert!(check("cat <<EOF\nhello world\nEOF"));
    assert!(check("cat <<'EOF'\nhello world\nEOF"));
    assert!(check("cat <<-EOF\n\thello\nEOF"));
}

// ── gh api: safe patterns ────────────────────────────────────────────

#[test]
fn gh_api_bare_endpoint() {
    for endpoint in GH_API_BARE_ENDPOINTS {
        let cmd = format!("gh api {endpoint}");
        assert!(check(&cmd), "expected safe: {cmd}");
    }
}

const GH_API_BARE_ENDPOINTS: &[&str] = &[
    "repos/owner/repo/pulls",
    "repos/owner/repo/issues",
    "repos/owner/repo/commits",
    "repos/owner/repo/releases",
    "repos/owner/repo/branches",
    "repos/owner/repo/tags",
    "repos/owner/repo/contributors",
    "repos/owner/repo/languages",
    "repos/owner/repo/topics",
    "repos/owner/repo/readme",
    "repos/owner/repo/license",
    "repos/owner/repo/contents/src/main.rs",
    "repos/owner/repo/git/refs",
    "repos/owner/repo/git/trees/main",
    "repos/owner/repo/actions/runs",
    "repos/owner/repo/actions/workflows",
    "repos/owner/repo/check-runs/123",
    "repos/owner/repo/check-suites/456",
    "repos/owner/repo/deployments",
    "repos/owner/repo/milestones",
    "repos/owner/repo/labels",
    "repos/owner/repo/stargazers",
    "repos/owner/repo/forks",
    "repos/owner/repo/collaborators",
    "user",
    "user/repos",
    "users/octocat",
    "users/octocat/repos",
    "orgs/github",
    "orgs/github/repos",
    "orgs/github/members",
    "search/repositories?q=rust",
    "rate_limit",
    "notifications",
    "gists",
];

#[test]
fn gh_api_pr_review_patterns() {
    assert!(check("gh api repos/o/r/pulls/1"));
    assert!(check("gh api repos/o/r/pulls/1/reviews"));
    assert!(check("gh api repos/o/r/pulls/1/comments"));
    assert!(check("gh api repos/o/r/pulls/1/commits"));
    assert!(check("gh api repos/o/r/pulls/1/files"));
    assert!(check("gh api repos/o/r/pulls/1/requested_reviewers"));
    assert!(check("gh api repos/o/r/pulls/1/merge"));
}

#[test]
fn gh_api_issue_patterns() {
    assert!(check("gh api repos/o/r/issues/123"));
    assert!(check("gh api repos/o/r/issues/123/comments"));
    assert!(check("gh api repos/o/r/issues/123/labels"));
    assert!(check("gh api repos/o/r/issues/123/events"));
    assert!(check("gh api repos/o/r/issues/123/timeline"));
}

#[test]
fn gh_api_jq_variations() {
    assert!(check("gh api repos/o/r/pulls --jq '.[].title'"));
    assert!(check("gh api repos/o/r/pulls --jq '.[].number'"));
    assert!(check("gh api repos/o/r/pulls --jq '.[] | {number, title}'"));
    assert!(check("gh api repos/o/r/pulls --jq 'length'"));
    assert!(check("gh api repos/o/r/pulls -q '.[0]'"));
    assert!(check("gh api repos/o/r/pulls -q '.[] | select(.draft)'"));
}

#[test]
fn gh_api_template_variations() {
    assert!(check("gh api repos/o/r/pulls --template '{{.title}}'"));
    assert!(check("gh api repos/o/r/pulls -t '{{range .}}{{.title}}{{end}}'"));
    assert!(check("gh api repos/o/r/releases -t '{{.tag_name}}'"));
}

#[test]
fn gh_api_pagination() {
    assert!(check("gh api repos/o/r/pulls --paginate"));
    assert!(check("gh api repos/o/r/issues --paginate --jq '.[].title'"));
    assert!(check("gh api repos/o/r/pulls --paginate --slurp"));
    assert!(check("gh api repos/o/r/pulls --paginate --slurp --jq 'flatten | length'"));
    assert!(check("gh api repos/o/r/stargazers --paginate --silent"));
    assert!(check("gh api graphql --paginate -q '.data.viewer.repositories.nodes[].nameWithOwner'"));
}

#[test]
fn gh_api_cache() {
    assert!(check("gh api repos/o/r/pulls --cache 3600s"));
    assert!(check("gh api repos/o/r/pulls --cache 60m"));
    assert!(check("gh api repos/o/r/pulls --cache 1h"));
    assert!(check("gh api rate_limit --cache 30s"));
}

#[test]
fn gh_api_hostname() {
    assert!(check("gh api repos/o/r/pulls --hostname github.example.com"));
    assert!(check("gh api repos/o/r/pulls --hostname enterprise.corp.com --jq '.[].title'"));
}

#[test]
fn gh_api_preview() {
    assert!(check("gh api repos/o/r/pulls -p corsair"));
    assert!(check("gh api repos/o/r/pulls --preview corsair"));
    assert!(check("gh api repos/o/r/pulls -p corsair -p nebula"));
    assert!(check("gh api repos/o/r/pulls --preview corsair --preview nebula"));
}

#[test]
fn gh_api_include() {
    assert!(check("gh api repos/o/r/pulls -i"));
    assert!(check("gh api repos/o/r/pulls --include"));
    assert!(check("gh api repos/o/r/pulls -i --jq '.[].title'"));
}

#[test]
fn gh_api_verbose_silent() {
    assert!(check("gh api repos/o/r/pulls --verbose"));
    assert!(check("gh api repos/o/r/pulls --silent"));
}

#[test]
fn gh_api_explicit_get() {
    assert!(check("gh api repos/o/r/pulls -X GET"));
    assert!(check("gh api repos/o/r/pulls -XGET"));
    assert!(check("gh api repos/o/r/pulls --method GET"));
    assert!(check("gh api repos/o/r/pulls --method=GET"));
    assert!(check("gh api repos/o/r/pulls -X GET --jq '.[].title'"));
    assert!(check("gh api repos/o/r/pulls -X GET --paginate"));
    assert!(check("gh api search/issues -X GET --jq '.items[].title'"));
}

#[test]
fn gh_api_combined_flags() {
    assert!(check("gh api repos/o/r/pulls --paginate --slurp --jq '.[].title' --cache 60s"));
    assert!(check("gh api repos/o/r/pulls --paginate --jq '.[] | {number, title}' -i"));
    assert!(check("gh api repos/o/r/issues --hostname enterprise.corp.com --paginate --jq '.[].title'"));
    assert!(check("gh api repos/o/r/pulls -p corsair --cache 1h --jq 'length'"));
    assert!(check("gh api repos/o/r/pulls --verbose --paginate --slurp"));
    assert!(check("gh api repos/o/r/commits -t '{{range .}}{{.sha}}{{end}}' --cache 300s"));
    assert!(check("gh api repos/o/r/releases --paginate --slurp --template '{{range .}}{{.tag_name}}{{end}}'"));
}

#[test]
fn gh_api_in_pipelines() {
    assert!(check("gh api repos/o/r/pulls --jq '.[].title' | head -5"));
    assert!(check("gh api repos/o/r/contents/f --jq '.content' | base64 -d"));
    assert!(check("gh api repos/o/r/pulls --paginate --slurp | jq 'flatten | length'"));
    assert!(check("gh api repos/o/r/pulls --jq '.[].number' | sort -n | tail -1"));
    assert!(check("gh api repos/o/r/pulls --paginate --jq '.[].title' | grep -c pattern"));
    assert!(check("gh api repos/o/r/readme --jq '.content' | base64 -d | head -50"));
}

#[test]
fn gh_api_with_redirects() {
    assert!(check("gh api repos/o/r/pulls 2>&1"));
    assert!(check("gh api repos/o/r/pulls --jq '.[].title' 2>/dev/null"));
    assert!(check("gh api repos/o/r/pulls --silent 2>/dev/null"));
}

#[test]
fn gh_api_graphql_safe_readonly() {
    assert!(check("gh api graphql --jq '.data.viewer.login'"));
    assert!(check("gh api graphql --paginate --jq '.data.viewer.repositories.nodes[].name'"));
    assert!(check("gh api graphql --paginate --slurp --jq '.[].data'"));
    assert!(check("gh api graphql -q '.data'"));
}

// ── gh api: denied patterns ──────────────────────────────────────────

#[test]
fn gh_api_mutation_methods() {
    assert!(!check("gh api repos/o/r/pulls -X POST"));
    assert!(!check("gh api repos/o/r/pulls -XPOST"));
    assert!(!check("gh api repos/o/r/pulls -X PATCH"));
    assert!(!check("gh api repos/o/r/pulls -XPATCH"));
    assert!(!check("gh api repos/o/r/pulls -X PUT"));
    assert!(!check("gh api repos/o/r/pulls -XPUT"));
    assert!(!check("gh api repos/o/r/pulls -X DELETE"));
    assert!(!check("gh api repos/o/r/pulls -XDELETE"));
    assert!(!check("gh api repos/o/r/pulls --method POST"));
    assert!(!check("gh api repos/o/r/pulls --method PATCH"));
    assert!(!check("gh api repos/o/r/pulls --method=POST"));
    assert!(!check("gh api repos/o/r/pulls --method=PATCH"));
    assert!(!check("gh api repos/o/r/pulls --method=PUT"));
    assert!(!check("gh api repos/o/r/pulls --method=DELETE"));
}

#[test]
fn gh_api_body_flags() {
    assert!(!check("gh api repos/o/r/issues -f title=bug"));
    assert!(!check("gh api repos/o/r/issues -f body='description'"));
    assert!(!check("gh api repos/o/r/issues -F title=bug"));
    assert!(!check("gh api repos/o/r/issues -F 'labels[]=bug'"));
    assert!(!check("gh api repos/o/r/issues --field title=bug"));
    assert!(!check("gh api repos/o/r/issues --raw-field body=text"));
    assert!(!check("gh api repos/o/r/rulesets --input file.json"));
    assert!(!check("gh api repos/o/r/rulesets --input -"));
    assert!(!check("gh api graphql -f query='{viewer{login}}'"));
    assert!(!check("gh api graphql -F owner=octocat -f query='query{}'"));
}

#[test]
fn gh_api_header_flag() {
    assert!(check("gh api repos/o/r/pulls -H 'Accept: application/json'"));
    assert!(check("gh api repos/o/r/pulls --header 'Accept: application/json'"));
    assert!(check("gh api repos/o/r/pulls -H 'X-GitHub-Api-Version: 2022-11-28'"));
    assert!(!check("gh api repos/o/r/pulls -H 'Content-Type: application/json'"));
    assert!(!check("gh api repos/o/r/pulls --header 'X-Custom: value'"));
    assert!(!check("gh api repos/o/r/pulls -H 'Authorization: token ghp_xxx'"));
}

#[test]
fn gh_api_unknown_flags() {
    assert!(!check("gh api repos/o/r/pulls --unknown-flag"));
    assert!(!check("gh api repos/o/r/pulls -Z"));
    assert!(!check("gh api repos/o/r/pulls --foo=bar"));
}

#[test]
fn gh_api_mixed_safe_and_unsafe() {
    assert!(!check("gh api repos/o/r/issues --jq '.[].title' -f title=bug"));
    assert!(!check("gh api repos/o/r/pulls --paginate -X POST"));
    assert!(!check("gh api repos/o/r/pulls --cache 60s -H 'Authorization: Bearer x'"));
    assert!(!check("gh api repos/o/r/pulls --jq '.[]' --input data.json"));
    assert!(!check("gh api repos/o/r/pulls --paginate --field key=val"));
}
