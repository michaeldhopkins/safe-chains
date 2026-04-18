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

- Subcommands alias, attestation, cache, codespace, config, extension, gist, gpg-key, issue, label, org, pr, project, release, repo, ruleset, run, secret, ssh-key, variable, workflow are allowed with actions: checks, diff, list, status, verify, view, watch.
- Always safe: --version, search, status.
- auth status, browse (requires --no-browser), run rerun (SafeWrite), release download (requires --output), api (read-only: implicit GET or explicit -X GET, with --paginate, --slurp, --jq, --template, --cache, --preview, --include, --silent, --verbose, --hostname, -H for Accept and X-GitHub-Api-Version headers).

### `glab`
<p class="cmd-url"><a href="https://glab.readthedocs.io/en/latest/">https://glab.readthedocs.io/en/latest/</a></p>

- Subcommands ci, cluster, deploy-key, gpg-key, incident, issue, iteration, label, milestone, mr, release, repo, schedule, snippet, ssh-key, stack, variable are allowed with actions: diff, issues, list, status, view.
- Always safe: --version, -v, check-update, version.
- auth status, api (GET only).

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

