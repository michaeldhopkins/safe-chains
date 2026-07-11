# Code Forges

### `basecamp`
<p class="cmd-url"><a href="https://github.com/basecamp/basecamp-cli">https://github.com/basecamp/basecamp-cli</a></p>

- **auth token**
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

- **api**: Read-only REST/GraphQL: implicit GET or explicit `-X GET`. Allowed flags: --paginate, --slurp, --silent, --include, --verbose, --jq, --json, --template, --cache, --preview, --hostname. Headers via -H/--header limited to Accept and X-GitHub-Api-Version. -f/-F/--field/--raw-field require -X GET on REST endpoints; on the graphql endpoint, mutation queries are denied.
- **browse** (requires --no-browser, -n): Flags: --actions, --no-browser, --projects, --releases, --settings, --wiki, -a, -c, -n, -p, -r, -s, -w. Valued: --branch, --commit, --repo, -R, -b
- **completion**: Flags: --help, --no-descriptions, -h. Valued: --shell, -s
- **search**: Flags: --archived, --draft, --include-forks, --locked, --merged, --no-assignee, --no-label, --no-milestone, --no-project, --web, -w. Valued: --app, --assignee, --author, --checks, --closed, --commenter, --comments, --committer, --created, --filename, --followers, --forks, --good-first-issues, --hash, --help-wanted-issues, --include, --interactions, --involves, --jq, --json, --label, --language, --license, --limit, --match, --mentions, --merged-at, --milestone, --number, --order, --owner, --parent, --project, --reactions, --repo, --review, --review-requested, --reviewed-by, --size, --sort, --stars, --state, --team-mentions, --team-review-requested, --template, --topic, --updated, --visibility, -L, -R, -q
- **status** — see `simple_list` below

- **Without a subcommand:**
- Allowed standalone flags: --help, --version, -V, -h

- **Subcommands by action verb:**
- **alias, attestation, cache, codespace, config, extension, gist, gpg-key, issue, label, org, pr, project, repo, ruleset, secret, ssh-key, variable, workflow** (Inert)
  - **checks**: Flags: --fail-fast, --required, --watch, --web, -w. Valued: --interval, --jq, --json, --repo, --template, -R, -i, -q
  - **diff**: Flags: --name-only, --patch, --web, -w. Valued: --color, --repo, -R
  - **list**: Flags: --all, --archived, --comments, --draft, --fork, --no-archived, --source, --web, -a, -w. Valued: --app, --assignee, --author, --base, --env, --head, --jq, --json, --key, --label, --language, --limit, --mention, --milestone, --order, --org, --ref, --repo, --search, --sort, --state, --template, --topic, --user, --visibility, -B, -H, -L, -O, -R, -S, -e, -k, -l, -o, -q, -r, -u
  - **status**: Flags: --exit-status, --log, --log-failed, --web, -w. Valued: --jq, --json, --repo, --template, -R, -q
  - **verify** — see `simple_view` below
  - **view**: Flags: --comments, --web, --yaml, -c, -w, -y. Valued: --branch, --jq, --json, --ref, --repo, --template, -R, -b, -q, -r
  - **watch** — see `simple_view` below
- **run** (Inert)
  - **checks** — see `simple_view` below
  - **diff** — see `simple_view` below
  - **list**: Valued: --branch, --commit, --created, --event, --jq, --json, --limit, --repo, --status, --template, --user, --workflow, -L, -R, -b, -q, -u, -w
  - **status** — see `simple_view` below
  - **verify** — see `simple_view` below
  - **view**: Flags: --exit-status, --log, --log-failed, --verbose, --web, -v, -w. Valued: --attempt, --job, --jq, --json, --repo, --template, -R, -j, -q
  - **watch**: Flags: --exit-status. Valued: --interval, --repo, -R, -i
- **run** (SafeWrite)
  - **rerun**: Flags: --debug, --failed. Valued: --job, --repo, -R, -j
- **release** (Inert)
  - **checks** — see `simple_list` below
  - **diff** — see `simple_list` below
  - **list**: Flags: --exclude-drafts, --exclude-pre-releases. Valued: --jq, --json, --limit, --order, --repo, --template, -L, -R, -q
  - **status** — see `simple_list` below
  - **verify** — see `simple_list` below
  - **view**: Flags: --web, -w. Valued: --jq, --json, --repo, --template, -R, -q
  - **watch** — see `simple_list` below
- **release** (SafeWrite)
  - **download** (requires -O/--output): Flags: --clobber, --skip-existing. Valued: --archive, --dir, --output, --pattern, --repo, -A, -D, -O, -R, -p
- **auth** (Inert)
  - **status** — see `simple_list` below

- **Shared flag sets:**
- **simple_list**: Flags: --all, --archived, --fork, --no-archived, --source, --web, -a, -w. Valued: --env, --jq, --json, --key, --language, --limit, --order, --org, --ref, --repo, --search, --sort, --template, --topic, --user, --visibility, -L, -O, -R, -S, -e, -k, -l, -o, -q, -r, -u
- **simple_view**: Flags: --web, --yaml, -w, -y. Valued: --jq, --json, --ref, --repo, --template, -R, -q, -r

### `glab`
<p class="cmd-url"><a href="https://gitlab.com/gitlab-org/cli">https://gitlab.com/gitlab-org/cli</a></p>

- **api**: Read-only REST/GraphQL — same rules as `gh api`.
- **check-update**
- **version**

- **Without a subcommand:**
- Allowed standalone flags: --help, --version, -h, -v

- **Subcommands by action verb:**
- **ci, cluster, deploy-key, gpg-key, incident, issue, iteration, label, milestone, mr, release, repo, schedule, snippet, ssh-key, stack, variable** (Inert)
  - **diff**: Flags: --help, --raw, -h. Valued: --color, --repo, -R
  - **issues** — see `list` below
  - **list** — see `list` below
  - **status** — see `simple` below
  - **view**: Flags: --comments, --help, --resolved, --system-logs, --unresolved, --web, -c, -h, -p, -s, -w. Valued: --output, --page, --per-page, --repo, -F, -P, -R, -p
- **auth** (Inert)
  - **status** — see `simple` below

- **Shared flag sets:**
- **list**: Flags: --all, --closed, --draft, --help, --merged, -A, -M, -a, -c, -d, -g, -h, -q. Valued: --assignee, --author, --group, --label, --milestone, --not-label, --order, --output, --page, --per-page, --repo, --reviewer, --search, --sort, --source-branch, --state, --target-branch, -F, -P, -R, -S, -a, -g, -l, -m, -o, -p, -r, -s, -t
- **simple**: Flags: --help, -h, -q. Valued: --output, --page, --per-page, --repo, -F, -P, -R, -p

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

