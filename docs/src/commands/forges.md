# Code Forges

### `basecamp`
<p class="cmd-url"><a href="https://github.com/basecamp/basecamp-cli">https://github.com/basecamp/basecamp-cli</a></p>

- **auth token**: Positional args accepted
- **cards list**: Positional args accepted
- **commands**: Positional args accepted
- **doctor**: Positional args accepted
- **files list**: Positional args accepted
- **help**: Positional args accepted
- **projects list**: Positional args accepted
- **projects show**: Positional args accepted
- **search**: Positional args accepted
- **todos list**: Positional args accepted
- Allowed standalone flags: --help, --version, -h

### `gh`
<p class="cmd-url"><a href="https://cli.github.com/manual/">https://cli.github.com/manual/</a></p>

- Routing: [[command.matrix]] blocks below cover the sub × action grid (read-only subs and their action verbs → handler_policy by name). The handler's remaining logic is the universal `<sub> --help` shortcut, the `api` sub-handler (REST/GraphQL with method/header/mutation rules), and the dispatch from there into try_sub_dispatch / try_matrix_dispatch.

- **Sub × action matrix:**
- Parents (Inert): alias, attestation, cache, codespace, config, extension, gist, gpg-key, issue, label, org, pr, project, repo, ruleset, secret, ssh-key, variable, workflow
-   - **checks** → policy `checks`
-   - **diff** → policy `diff`
-   - **list** → policy `list`
-   - **status** → policy `status`
-   - **verify** → policy `simple_view`
-   - **view** → policy `view`
-   - **watch** → policy `simple_view`
- Parents (Inert): run
-   - **checks** → policy `simple_view`
-   - **diff** → policy `simple_view`
-   - **list** → policy `run_list`
-   - **status** → policy `simple_view`
-   - **verify** → policy `simple_view`
-   - **view** → policy `run_view`
-   - **watch** → policy `run_watch`
- Parents (SafeWrite): run
-   - **rerun** → policy `run_rerun`
- Parents (Inert): release
-   - **checks** → policy `simple_list`
-   - **diff** → policy `simple_list`
-   - **list** → policy `release_list`
-   - **status** → policy `simple_list`
-   - **verify** → policy `simple_list`
-   - **view** → policy `release_view`
-   - **watch** → policy `simple_list`
- Parents (SafeWrite): release
-   - **download** → policy `release_download` (requires -O/--output)
- Parents (Inert): auth
-   - **status** → policy `simple_list`

- **Fallback grammar (engaged when no sub matches):**
- Allowed standalone flags: --help, --version, -V, -h

- **Handler-side flag policies:**
- **browse**: Flags: --actions, --no-browser, --projects, --releases, --settings, --wiki, -a, -c, -n, -p, -r, -s, -w. Valued: --branch, --commit, --repo, -R, -b
- **checks**: Flags: --fail-fast, --required, --watch, --web, -w. Valued: --interval, --jq, --json, --repo, --template, -R, -i, -q
- **diff**: Flags: --name-only, --patch, --web, -w. Valued: --color, --repo, -R
- **list**: Flags: --all, --archived, --comments, --draft, --fork, --no-archived, --source, --web, -a, -w. Valued: --app, --assignee, --author, --base, --env, --head, --jq, --json, --key, --label, --language, --limit, --mention, --milestone, --order, --org, --ref, --repo, --search, --sort, --state, --template, --topic, --user, --visibility, -B, -H, -L, -O, -R, -S, -e, -k, -l, -o, -q, -r, -u
- **release_download**: Flags: --clobber, --skip-existing. Valued: --archive, --dir, --output, --pattern, --repo, -A, -D, -O, -R, -p
- **release_list**: Flags: --exclude-drafts, --exclude-pre-releases. Valued: --jq, --json, --limit, --order, --repo, --template, -L, -R, -q
- **release_view**: Flags: --web, -w. Valued: --jq, --json, --repo, --template, -R, -q
- **run_list**: Valued: --branch, --commit, --created, --event, --jq, --json, --limit, --repo, --status, --template, --user, --workflow, -L, -R, -b, -q, -u, -w
- **run_rerun**: Flags: --debug, --failed. Valued: --job, --repo, -R, -j
- **run_view**: Flags: --exit-status, --log, --log-failed, --verbose, --web, -v, -w. Valued: --attempt, --job, --jq, --json, --repo, --template, -R, -j, -q
- **run_watch**: Flags: --exit-status. Valued: --interval, --repo, -R, -i
- **search**: Flags: --archived, --draft, --include-forks, --locked, --merged, --no-assignee, --no-label, --no-milestone, --no-project, --web, -w. Valued: --app, --assignee, --author, --checks, --closed, --commenter, --comments, --committer, --created, --filename, --followers, --forks, --good-first-issues, --hash, --help-wanted-issues, --include, --interactions, --involves, --jq, --json, --label, --language, --license, --limit, --match, --mentions, --merged-at, --milestone, --number, --order, --owner, --parent, --project, --reactions, --repo, --review, --review-requested, --reviewed-by, --size, --sort, --stars, --state, --team-mentions, --team-review-requested, --template, --topic, --updated, --visibility, -L, -R, -q
- **simple_list**: Flags: --all, --archived, --fork, --no-archived, --source, --web, -a, -w. Valued: --env, --jq, --json, --key, --language, --limit, --order, --org, --ref, --repo, --search, --sort, --template, --topic, --user, --visibility, -L, -O, -R, -S, -e, -k, -l, -o, -q, -r, -u
- **simple_view**: Flags: --web, --yaml, -w, -y. Valued: --jq, --json, --ref, --repo, --template, -R, -q, -r
- **status**: Flags: --exit-status, --log, --log-failed, --web, -w. Valued: --jq, --json, --repo, --template, -R, -q
- **view**: Flags: --comments, --web, --yaml, -c, -w, -y. Valued: --branch, --jq, --json, --ref, --repo, --template, -R, -b, -q, -r

### `glab`
<p class="cmd-url"><a href="https://gitlab.com/gitlab-org/cli">https://gitlab.com/gitlab-org/cli</a></p>

- Routing: [[command.matrix]] blocks below cover the read-only sub × action grid. Top-level terminal forms `glab version` and `glab check-update` are handled by the handler (no flags allowed). `glab api` delegates to gh's API sub-handler.

- **Sub × action matrix:**
- Parents (Inert): ci, cluster, deploy-key, gpg-key, incident, issue, iteration, label, milestone, mr, release, repo, schedule, snippet, ssh-key, stack, variable
-   - **diff** → policy `diff`
-   - **issues** → policy `list`
-   - **list** → policy `list`
-   - **status** → policy `simple`
-   - **view** → policy `view`
- Parents (Inert): auth
-   - **status** → policy `simple`

- **Fallback grammar (engaged when no sub matches):**
- Allowed standalone flags: --help, --version, -h, -v

- **Handler-side flag policies:**
- **diff**: Flags: --help, --raw, -h. Valued: --color, --repo, -R
- **list**: Flags: --all, --closed, --draft, --help, --merged, -A, -M, -a, -c, -d, -g, -h, -q. Valued: --assignee, --author, --group, --label, --milestone, --not-label, --order, --output, --page, --per-page, --repo, --reviewer, --search, --sort, --source-branch, --state, --target-branch, -F, -P, -R, -S, -a, -g, -l, -m, -o, -p, -r, -s, -t
- **simple**: Flags: --help, -h, -q. Valued: --output, --page, --per-page, --repo, -F, -P, -R, -p
- **view**: Flags: --comments, --help, --resolved, --system-logs, --unresolved, --web, -c, -h, -p, -s, -w. Valued: --output, --page, --per-page, --repo, -F, -P, -R, -p

- **Handler-side data:**
- **always_safe_bare_subs**: check-update, version

### `jjpr`
<p class="cmd-url"><a href="https://github.com/michaeldhopkins/jjpr">https://github.com/michaeldhopkins/jjpr</a></p>

- **auth help**: Positional args accepted
- **auth setup**: Flags: --help, -h
- **auth test**: Flags: --help, -h
- **config help**: Positional args accepted
- **help**: Positional args accepted
- **merge** (requires --dry-run): Flags: --dry-run, --help, --no-ci-check, --no-fetch, --watch, -h. Valued: --base, --merge-method, --reconcile-strategy, --remote, --required-approvals
- **status**: Flags: --dry-run, --help, --no-fetch, -h
- **submit** (requires --dry-run): Flags: --draft, --dry-run, --help, --no-fetch, --ready, -h. Valued: --base, --remote, --reviewer
- Allowed standalone flags: --help, --version, -V, -h
- Bare invocation allowed

### `tea`
<p class="cmd-url"><a href="https://gitea.com/gitea/tea">https://gitea.com/gitea/tea</a></p>

- **b list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **b view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **branch list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **branch view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **branches list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **branches view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **i list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **i view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **issue list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **issue view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **issues list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **issues view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **label list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **label view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **labels list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **labels view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **login list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **logins list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **milestone list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **milestone view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **milestones list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **milestones view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **ms list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **ms view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **n list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **n view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **notification list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **notification view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **notifications list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **notifications view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **org list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **org view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **organization list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **organization view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **organizations list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **organizations view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **pr list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **pr view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **pull list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **pull view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **pulls list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **pulls view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **r list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **r view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **release list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **release view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **releases list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **releases view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **repo list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **repo view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **repos list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **repos view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **t list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **t view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **time list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **time view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **times list**: Flags: --help, -h. Valued: --fields, --limit, --login, --output, --page, --repo, --state, -L, -R, -f, -l, -o, -p, -s
- **times view**: Flags: --comments, --help, -c, -h. Valued: --login, --output, --repo, -R, -l, -o
- **whoami**
- Allowed standalone flags: --version, -v

