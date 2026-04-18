# Python

### `bandit`
<p class="cmd-url"><a href="https://bandit.readthedocs.io/">https://bandit.readthedocs.io/</a></p>

- Allowed standalone flags: --help, --ignore-nosec, --number, --one-line, --quiet, --recursive, --verbose, --version, -h, -i, -n, -q, -r, -v
- Allowed valued flags: --aggregate, --baseline, --config, --configfile, --exclude, --format, --output, --profile, --severity-level, --skip, --tests, -b, -c, -f, -l, -o, -p, -s, -t

### `conda`
<p class="cmd-url"><a href="https://docs.conda.io/projects/conda/en/stable/commands/index.html">https://docs.conda.io/projects/conda/en/stable/commands/index.html</a></p>

- **config** (requires --show, --show-sources): Flags: --help, --json, --quiet, --show, --show-sources, --verbose, -h, -q, -v. Valued: --env, --file, --name, --prefix, -f, -n, -p
- **info**: Flags: --all, --envs, --help, --json, --verbose, -a, -e, -h, -v
- **list**: Flags: --explicit, --export, --full-name, --help, --json, --no-pip, --revisions, -e, -f, -h. Valued: --name, --prefix, -n, -p
- Allowed standalone flags: --help, --version, -V, -h

### `coverage`
<p class="cmd-url"><a href="https://coverage.readthedocs.io/">https://coverage.readthedocs.io/</a></p>

- **combine**: Flags: --append, --help, --keep, -a, -h. Valued: --data-file
- **html**: Flags: --help, --ignore-errors, --include, --omit, --show-contexts, --skip-covered, --skip-empty, -h. Valued: --data-file, --directory, --fail-under, --precision, --title, -d
- **json**: Flags: --help, --ignore-errors, --include, --omit, --pretty-print, -h. Valued: --data-file, --fail-under, -o
- **report**: Flags: --help, --ignore-errors, --include, --no-skip-covered, --omit, --show-missing, --skip-covered, --skip-empty, -h, -m. Valued: --data-file, --fail-under, --precision, --sort
- **run**: Flags: --append, --branch, --concurrency, --help, --parallel, --source, --timid, -a, -h, -p. Valued: --context, --data-file, --include, --omit, --rcfile, --source. Positional args accepted
- Allowed standalone flags: --help, --version, -h

### `nox`
<p class="cmd-url"><a href="https://nox.thea.codes/">https://nox.thea.codes/</a></p>

- Allowed standalone flags: --error-on-external-run, --error-on-missing-interpreters, --help, --list, --no-color, --no-error-on-external-run, --no-error-on-missing-interpreters, --no-install, --no-venv, --reuse-existing-virtualenvs, --stop-on-first-error, --version, -R, -h, -l, -r, -x
- Allowed valued flags: --default-venv-backend, --envdir, --extra-pythons, --force-pythons, --noxfile, --pythons, --sessions, --tags, -e, -f, -p, -s, -t
- Bare invocation allowed

### `pdm`
<p class="cmd-url"><a href="https://pdm-project.org/">https://pdm-project.org/</a></p>

- **config**: Flags: --delete, --global, --help, --local, --project, -d, -g, -h, -l
- **info**: Flags: --env, --help, --json, --packages, --python, --where, -h
- **list**: Flags: --csv, --freeze, --graph, --help, --json, --markdown, --reverse, --tree, -h. Valued: --exclude, --fields, --include, --resolve, --sort
- **search**: Flags: --help, -h
- **show**: Flags: --help, --json, --keywords, --name, --platform, --summary, --version, -h
- Allowed standalone flags: --help, --version, -V, -h

### `pip`
<p class="cmd-url"><a href="https://pip.pypa.io/en/stable/cli/">https://pip.pypa.io/en/stable/cli/</a></p>

Aliases: `pip3`

- **check**: Flags: --help, -h
- **config get**: Flags: --help, -h
- **config list**: Flags: --help, -h
- **debug**: Flags: --help, -h
- **download**: Flags: --help, --no-deps, --pre, --quiet, --require-hashes, --verbose, -h, -q, -v. Valued: --constraint, --dest, --extra-index-url, --find-links, --index-url, --no-binary, --only-binary, --platform, --python-version, --requirement, -c, -d, -f, -i, -r
- **freeze**: Flags: --all, --exclude-editable, --help, --local, --user, -h, -l. Valued: --exclude, --path
- **help**: Flags: --help, -h
- **index**: Flags: --help, -h
- **inspect**: Flags: --help, -h
- **install** (requires --dry-run): Flags: --dry-run, --help, --no-deps, --pre, --quiet, --require-hashes, --user, --verbose, -U, -h, -q, -v. Valued: --constraint, --extra-index-url, --find-links, --index-url, --no-binary, --only-binary, --platform, --python-version, --requirement, --target, --upgrade-strategy, -c, -f, -i, -r, -t
- **list**: Flags: --editable, --exclude-editable, --help, --include-editable, --local, --not-required, --outdated, --pre, --uptodate, --user, -e, -h, -i, -l, -o. Valued: --exclude, --format, --index-url, --path
- **show**: Flags: --files, --help, --verbose, -f, -h, -v
- Allowed standalone flags: --help, --version, -V, -h

### `pip-audit`
<p class="cmd-url"><a href="https://github.com/pypa/pip-audit">https://github.com/pypa/pip-audit</a></p>

- Allowed standalone flags: --desc, --dry-run, --help, --json, --local, --no-deps, --skip-editable, --strict, --verbose, --version, -S, -h, -l, -s, -v
- Allowed valued flags: --cache-dir, --exclude, --format, --ignore-vuln, --index-url, --output, --path, --requirement, -e, -f, -i, -o, -r
- Bare invocation allowed

### `poetry`
<p class="cmd-url"><a href="https://python-poetry.org/docs/cli/">https://python-poetry.org/docs/cli/</a></p>

- **check**: Flags: --help, --lock, -h
- **env info**: Flags: --full-path, --help, -h
- **env list**: Flags: --full-path, --help, -h
- **show**: Flags: --all, --help, --latest, --no-dev, --outdated, --top-level, --tree, -T, -h, -l, -o. Valued: --why
- Allowed standalone flags: --help, --version, -V, -h

### `pyenv`
<p class="cmd-url"><a href="https://github.com/pyenv/pyenv#readme">https://github.com/pyenv/pyenv#readme</a></p>

- **help**: Flags: --bare, --help, -h
- **root**: Flags: --bare, --help, -h
- **shims**: Flags: --bare, --help, -h
- **version**: Flags: --bare, --help, -h
- **versions**: Flags: --bare, --help, -h
- **which**: Flags: --bare, --help, -h
- Allowed standalone flags: --help, --version, -V, -h

### `pytest`
<p class="cmd-url"><a href="https://docs.pytest.org/">https://docs.pytest.org/</a></p>

- Allowed standalone flags: --cache-clear, --cache-show, --co, --collect-only, --doctest-modules, --help, --last-failed, --lf, --markers, --new-first, --nf, --no-header, --quiet, --showlocals, --stepwise, --strict-markers, --verbose, --version, -h, -l, -q, -v, -x
- Allowed valued flags: --basetemp, --color, --confcutdir, --deselect, --durations, --ignore, --import-mode, --junitxml, --log-cli-level, --maxfail, --override-ini, --rootdir, --timeout, -c, -k, -m, -o, -p, -r, -W
- Bare invocation allowed

### `tox`
<p class="cmd-url"><a href="https://tox.wiki/">https://tox.wiki/</a></p>

- **config**: Flags: --core, --help, -h
- **list**: Flags: --help, --no-desc, -d, -h
- **run**: Flags: --help, --no-recreate-pkg, --skip-missing-interpreters, -h. Valued: -e, -f, --override, --result-json
- Allowed standalone flags: --help, --version, -h

### `uv`
<p class="cmd-url"><a href="https://docs.astral.sh/uv/reference/cli/">https://docs.astral.sh/uv/reference/cli/</a></p>

- **pip check**: Flags: --help, --verbose, -h, -v. Valued: --python
- **pip freeze**: Flags: --help, --verbose, -h, -v. Valued: --python
- **pip list**: Flags: --editable, --exclude-editable, --help, --outdated, --strict, -h. Valued: --exclude, --format, --python
- **pip show**: Flags: --files, --help, --verbose, -h, -v. Valued: --python
- **python list**: Flags: --help, --verbose, -h, -v. Valued: --python
- **tool list**: Flags: --help, --verbose, -h, -v. Valued: --python
- Allowed standalone flags: --help, --version, -V, -h

