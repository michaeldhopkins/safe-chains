# Python

### `alembic`
<p class="cmd-url"><a href="https://alembic.sqlalchemy.org/">https://alembic.sqlalchemy.org/</a></p>

- **branches**: Flags: --help, --verbose, -h, -v
- **check**: Flags: --help, -h
- **current**: Flags: --help, --verbose, -h, -v
- **heads**: Flags: --help, --resolve-dependencies, --verbose, -h, -v
- **help**: Positional args accepted
- **history**: Flags: --help, --indicate-current, --verbose, -h, -i, -v. Valued: --rev-range, -r
- **init**: Flags: --help, --package, -h. Valued: --template, -t. Positional args accepted
- **list_templates**: Flags: --help, -h
- **revision**: Flags: --autogenerate, --head, --help, --splice, -h. Valued: --branch-label, --depends-on, --message, --rev-id, --sql, --version-path, -m
- **show**: Flags: --help, -h. Positional args accepted
- Allowed standalone flags: --help, --version, -h

### `autoflake`
<p class="cmd-url"><a href="https://github.com/PyCQA/autoflake">https://github.com/PyCQA/autoflake</a></p>

- Allowed standalone flags: --check, --check-diff, --expand-star-imports, --help, --ignore-pass-after-docstring, --ignore-pass-statements, --in-place, --quiet, --recursive, --remove-all-unused-imports, --remove-duplicate-keys, --remove-rhs-for-unused-variables, --remove-unused-variables, --verbose, --version, -c, -h, -i, -q, -r, -v
- Allowed valued flags: --exclude, --ignore-init-module-imports, --imports, --jobs, --stdin-display-name, -j
- Bare invocation allowed

### `autopep8`
<p class="cmd-url"><a href="https://github.com/hhatto/autopep8">https://github.com/hhatto/autopep8</a></p>

- Allowed standalone flags: --aggressive, --diff, --exit-code, --experimental, --global-config, --help, --ignore-local-config, --in-place, --list-fixes, --pep8-passes, --recursive, --verbose, --version, -a, -d, -h, -i, -r, -v
- Allowed valued flags: --exclude, --global-config, --ignore, --indent-size, --jobs, --line-range, --max-line-length, --select, -j, -l
- Bare invocation allowed

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

### `cookiecutter`
<p class="cmd-url"><a href="https://cookiecutter.readthedocs.io/">https://cookiecutter.readthedocs.io/</a></p>

- Allowed standalone flags: --debug-file, --help, --list-installed, --version, -h

### `copier`
<p class="cmd-url"><a href="https://copier.readthedocs.io/">https://copier.readthedocs.io/</a></p>

- **help**: Positional args accepted
- Allowed standalone flags: --help, --version, -h

### `coverage`
<p class="cmd-url"><a href="https://coverage.readthedocs.io/">https://coverage.readthedocs.io/</a></p>

- **combine**: Flags: --append, --help, --keep, -a, -h. Valued: --data-file
- **html**: Flags: --help, --ignore-errors, --include, --omit, --show-contexts, --skip-covered, --skip-empty, -h. Valued: --data-file, --directory, --fail-under, --precision, --title, -d
- **json**: Flags: --help, --ignore-errors, --include, --omit, --pretty-print, -h. Valued: --data-file, --fail-under, -o
- **report**: Flags: --help, --ignore-errors, --include, --no-skip-covered, --omit, --show-missing, --skip-covered, --skip-empty, -h, -m. Valued: --data-file, --fail-under, --precision, --sort
- **run**: Flags: --append, --branch, --concurrency, --help, --parallel, --source, --timid, -a, -h, -p. Valued: --context, --data-file, --include, --omit, --rcfile, --source. Positional args accepted
- Allowed standalone flags: --help, --version, -h

### `dbt`
<p class="cmd-url"><a href="https://docs.getdbt.com/reference/dbt-commands">https://docs.getdbt.com/reference/dbt-commands</a></p>

- **clean**: Flags: --help, --no-clean-project-files-only, --profile, --quiet, -h, -q. Valued: --profile, --profiles-dir, --project-dir, --target
- **compile**: Flags: --cache-selected-only, --debug, --exclude-resource-type, --full-refresh, --help, --inline, --no-cache-selected-only, --no-defer, --no-favor-state, --no-full-refresh, --no-introspect, --no-partial-parse, --no-populate-cache, --no-print, --no-static-parser, --no-version-check, --profile, --profiles-dir, --quiet, --show, --target, --target-path, --threads, --vars, --warn-error, -h, -q. Valued: --exclude, --inline, --profile, --profiles-dir, --project-dir, --resource-type, --select, --state, --target, --target-path, --threads, --vars, -d, -m, -s, -t
- **debug**: Flags: --config-dir, --connection, --help, --no-version-check, --quiet, --version-check, -h, -q. Valued: --profile, --project-dir, --profiles-dir, --target, -t
- **deps**: Flags: --add-package, --dry-run, --help, --lock, --no-lock, --quiet, --upgrade, -h, -q. Valued: --profile, --profiles-dir, --project-dir, --source, --target, --vars, -t
- **help**: Positional args accepted
- **init**: Flags: --help, --profiles-dir, --quiet, --skip-profile-setup, -h, -q, -s
- **list**: Flags: --help, --no-version-check, --quiet, -h, -q. Valued: --exclude, --output, --output-keys, --profile, --profiles-dir, --project-dir, --resource-type, --select, --state, --target, -d, -m, -s, -t
- **ls**: Flags: --help, --no-version-check, --quiet, --resource-type, -h, -q. Valued: --exclude, --exclude-resource-type, --output, --output-keys, --profile, --profiles-dir, --project-dir, --resource-type, --select, --state, --target, -d, -m, -s, -t
- **parse**: Flags: --help, --no-partial-parse, --no-static-parser, --no-version-check, --quiet, --show-hash, --write-json, -h, -q. Valued: --profile, --profiles-dir, --project-dir, --target, --target-path, --threads, --vars, -t
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h, -V

### `deptry`
<p class="cmd-url"><a href="https://deptry.com/">https://deptry.com/</a></p>

- Allowed standalone flags: --help, --ignore-unused, --known-first-party, --no-ansi, --per-rule-ignores, --verbose, --version, -h, -v
- Allowed valued flags: --config, --exclude, --extend-exclude, --ignore, --json-output, --package-module-name-map, --pep621-dev-dependency-groups, --per-rule-ignores, --requirements-files, --requirements-files-dev, -e, -ee, -i, -o, -pdd

### `dvc`
<p class="cmd-url"><a href="https://dvc.org/">https://dvc.org/</a></p>

- **add**: Flags: --external, --file, --force, --glob, --help, --no-commit, --quiet, --recursive, --remote, --verbose, -R, -f, -h, -q, -v. Valued: --desc, --file, --meta, --out, --remote, --type, -o
- **checkout**: Flags: --allow-missing, --force, --help, --quiet, --recursive, --relink, --summary, -R, -f, -h, -q. Positional args accepted
- **commit**: Flags: --data-only, --force, --help, --no-commit, --quiet, --recursive, --relink, --verbose, -R, -d, -f, -h, -q, -v. Positional args accepted
- **dag**: Flags: --dot, --full, --help, --mermaid, --md, --outs, --quiet, --verbose, -h, -q, -v. Positional args accepted
- **diff**: Flags: --help, --hide-missing, --json, --md, --quiet, --show-hash, --targets, --verbose, -h, -q, -v. Positional args accepted
- **doctor**: Flags: --help, -h
- **help**: Positional args accepted
- **list**: Flags: --dvc-only, --help, --json, --quiet, -R, -h, -q. Valued: --rev. Positional args accepted
- **move**: Flags: --help, --quiet, --verbose, -h, -q, -v. Positional args accepted
- **remove**: Flags: --help, --outs, --quiet, --verbose, -h, -q, -v. Positional args accepted
- **status**: Flags: --all-branches, --all-commits, --all-tags, --cloud, --help, --json, --quiet, --remote, --verbose, -A, -T, -a, -c, -h, -q, -v. Positional args accepted
- **unprotect**: Flags: --help, --quiet, -h, -q. Positional args accepted
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h, -V

**Examples:**

- `dvc add data.csv`
- `dvc status`
- `dvc version`

### `hatch`
<p class="cmd-url"><a href="https://hatch.pypa.io/">https://hatch.pypa.io/</a></p>

- **build**: Flags: --clean, --clean-hooks-after, --clean-only, --ext, --help, -c, -h. Valued: --hooks-only, --target, -t
- **clean**: Flags: --ext, --help, --no-hooks, -h. Valued: --target, -t
- **dep hash**: Flags: --help, -h
- **dep show**: Flags: --help, -h
- **env create**: Flags: --help, -h
- **env find**: Flags: --help, -h
- **env prune**: Flags: --help, -h
- **env remove**: Flags: --help, -h
- **env run**: Flags: --help, -h
- **env show**: Flags: --ascii, --force-ascii, --help, --json, -h
- **help**: Positional args accepted
- **new**: Flags: --cli, --help, --init, -h, -i. Positional args accepted
- **project metadata**: Flags: --help, -h
- **status**: Flags: --help, -h
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h, -V

### `http`
<p class="cmd-url"><a href="https://httpie.io/docs/cli">https://httpie.io/docs/cli</a></p>

Aliases: `https`

- Allowed standalone flags: --all, --body, --check-status, --continue, --debug, --default-scheme, --download, --follow, --form, --headers, --help, --ignore-stdin, --ignore-netrc, --json, --meta, --multipart, --no-stream, --offline, --overwrite, --pretty, --print, --quiet, --stream, --style, --traceback, --verbose, --verify, --version, -F, -I, -S, -b, -c, -d, -f, -h, -j, -m, -o, -p, -q, -s, -v
- Allowed valued flags: --auth, --auth-type, --bearer, --cert, --cert-key, --cert-key-pass, --chunked, --compress, --default-scheme, --format-options, --max-headers, --max-redirects, --output, --path-as-is, --proxy, --response-charset, --response-mime, --session, --session-read-only, --ssl, --timeout, -A, -a, -x

### `ipython`
<p class="cmd-url"><a href="https://ipython.org/">https://ipython.org/</a></p>

- Allowed standalone flags: --help, --version, -h

### `jupyter`
<p class="cmd-url"><a href="https://docs.jupyter.org/en/latest/projects/jupyter-command.html">https://docs.jupyter.org/en/latest/projects/jupyter-command.html</a></p>

- **help**: Positional args accepted
- **troubleshoot**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

### `jupyter-nbconvert`
<p class="cmd-url"><a href="https://nbconvert.readthedocs.io/">https://nbconvert.readthedocs.io/</a></p>

Aliases: `nbconvert`

- Allowed standalone flags: --allow-chromium-download, --allow-errors, --clear-output, --debug, --disable-chromium-sandbox, --embed-images, --exec, --execute, --from, --generate-config, --help, --inplace, --log-level, --no-input, --no-prompt, --no-stdin, --show-config, --show-config-json, --stdin, --stdout, --version, --writer, -h, -y
- Allowed valued flags: --config, --ExecutePreprocessor.timeout, --ExecutePreprocessor.kernel-name, --ExecutePreprocessor.kernel_name, --from, --log-level, --nbformat, --output, --output-dir, --post, --reveal-prefix, --template, --template-file, --to, --use-frontmatter, --writer, -c, -y

### `jupytext`
<p class="cmd-url"><a href="https://jupytext.readthedocs.io/">https://jupytext.readthedocs.io/</a></p>

- Allowed standalone flags: --check-paired, --diff, --help, --out, --quiet, --show-changes, --sync, --test, --test-strict, --update-metadata, --use-source-timestamp, --version, --warn-only, -h, -q
- Allowed valued flags: --from, --input-format, --opt, --output, --output-format, --paired-paths, --pipe, --pipe-fmt, --pre-commit-mode, --set-formats, --set-kernel, --to, --warn-only, -K, -i, -k, -o
- Bare invocation allowed

### `kernprof`
<p class="cmd-url"><a href="https://github.com/pyutils/line_profiler">https://github.com/pyutils/line_profiler</a></p>

- Allowed standalone flags: --help, --version, -h, -V

### `mkdocs`
<p class="cmd-url"><a href="https://www.mkdocs.org/">https://www.mkdocs.org/</a></p>

- **build**: Flags: --clean, --dirty, --help, --no-strict, --quiet, --strict, --verbose, -c, -h, -q, -s, -v. Valued: --config-file, --site-dir, --theme, -d, -f, -t
- **help**: Positional args accepted
- **new**: Flags: --help, --quiet, --verbose, -h, -q, -v. Positional args accepted
- Allowed standalone flags: --help, --version, -h, -V

### `mlflow`
<p class="cmd-url"><a href="https://mlflow.org/docs/latest/cli.html">https://mlflow.org/docs/latest/cli.html</a></p>

- **completion**: Flags: --help, -h. Positional args accepted
- **doctor**: Flags: --help, -h. Valued: --mask-envs
- **help**: Positional args accepted
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

### `mprof`
<p class="cmd-url"><a href="https://github.com/pythonprofilers/memory_profiler">https://github.com/pythonprofilers/memory_profiler</a></p>

- **clean**: Flags: --help, -h
- **help**: Positional args accepted
- **list**: Flags: --help, -h
- **rm**: Flags: --help, -h. Positional args accepted
- Allowed standalone flags: --help, --version, -h

### `nbqa`
<p class="cmd-url"><a href="https://nbqa.readthedocs.io/">https://nbqa.readthedocs.io/</a></p>

- Allowed standalone flags: --help, --nbqa-help, --nbqa-version, --version, -h, -V

### `nbstripout`
<p class="cmd-url"><a href="https://github.com/kynan/nbstripout">https://github.com/kynan/nbstripout</a></p>

- Allowed standalone flags: --attributes, --drop-empty-cells, --dry-run, --extra-keys, --force, --global, --help, --include-files-from-stdin, --install, --is-installed, --keep-count, --keep-id, --keep-metadata-keys, --keep-output, --max-size, --max-size-bytes, --no-empty-cells, --no-strip-init-cells, --strip-empty-cells, --strip-init-cells, --status, --system, --textconv, --uninstall, --verify, --version, -f, -h, -t
- Allowed valued flags: --attributes, --extra-keys, --keep-metadata-keys, --max-size
- Bare invocation allowed

### `nox`
<p class="cmd-url"><a href="https://nox.thea.codes/">https://nox.thea.codes/</a></p>

- Allowed standalone flags: --error-on-external-run, --error-on-missing-interpreters, --help, --list, --no-color, --no-error-on-external-run, --no-error-on-missing-interpreters, --no-install, --no-venv, --reuse-existing-virtualenvs, --stop-on-first-error, --version, -R, -h, -l, -r, -x
- Allowed valued flags: --default-venv-backend, --envdir, --extra-pythons, --force-pythons, --noxfile, --pythons, --sessions, --tags, -e, -f, -p, -s, -t
- Bare invocation allowed

### `papermill`
<p class="cmd-url"><a href="https://papermill.readthedocs.io/">https://papermill.readthedocs.io/</a></p>

- Allowed standalone flags: --help, --version, -h

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

### `pip-compile`
<p class="cmd-url"><a href="https://github.com/jazzband/pip-tools">https://github.com/jazzband/pip-tools</a></p>

- Allowed standalone flags: --all-build-deps, --all-extras, --allow-unsafe, --annotate, --annotation-style, --build-isolation, --cache-dir, --config, --constraint, --dry-run, --emit-find-links, --emit-index-url, --emit-options, --emit-trusted-host, --extra, --extra-index-url, --find-links, --generate-hashes, --header, --help, --index-url, --newline, --no-allow-unsafe, --no-annotate, --no-build-isolation, --no-config, --no-emit-find-links, --no-emit-index-url, --no-emit-options, --no-emit-trusted-host, --no-find-links, --no-generate-hashes, --no-header, --no-index, --no-strip-extras, --only-build-deps, --pip-args, --quiet, --rebuild, --strip-extras, --trusted-host, --unsafe-package, --upgrade, --verbose, --version, -U, -h, -i, -n, -o, -q, -r, -v
- Allowed valued flags: --build-deps-for, --no-build-deps-for, --output-file, --resolver, --upgrade-package, -P
- Bare invocation allowed

### `pip-sync`
<p class="cmd-url"><a href="https://github.com/jazzband/pip-tools">https://github.com/jazzband/pip-tools</a></p>

- Allowed standalone flags: --ask, --config, --dry-run, --force, --help, --no-config, --no-pip-args, --quiet, --verbose, --version, -a, -f, -h, -i, -n, -q, -v
- Allowed valued flags: --cache-dir, --extra-index-url, --find-links, --index-url, --pip-args, --python-executable, --trusted-host, --user-config
- Bare invocation allowed

### `pipx`
<p class="cmd-url"><a href="https://pipx.pypa.io/">https://pipx.pypa.io/</a></p>

- **completions**: Flags: --help, -h
- **ensurepath**: Flags: --force, --global, --help, -f, -h
- **environment**: Flags: --help, -h. Valued: --value, -v
- **inject**: Flags: --force, --global, --help, --include-apps, --include-deps, --quiet, --verbose, -f, -h, -q, -v. Valued: --index-url, --pip-args
- **install**: Flags: --editable, --force, --global, --help, --include-deps, --preinstall, --quiet, --system-site-packages, --verbose, -e, -f, -h, -q, -v. Valued: --fetch-missing-python, --index-url, --pip-args, --python, --suffix
- **install-all**: Flags: --force, --global, --help, --quiet, --verbose, -f, -h, -q, -v
- **list**: Flags: --help, --include-injected, --json, --short, --skip-maintenance, -h. Valued: --global
- **reinstall**: Flags: --global, --help, -h. Valued: --python
- **reinstall-all**: Flags: --global, --help, -h. Valued: --python, --skip
- **uninject**: Flags: --global, --help, --leave-deps, -h
- **uninstall**: Flags: --global, --help, -h
- **uninstall-all**: Flags: --global, --help, -h
- **upgrade**: Flags: --force, --global, --help, --include-injected, --install, --quiet, --verbose, -f, -h, -q, -v. Valued: --pip-args, --python
- **upgrade-all**: Flags: --force, --global, --help, --include-injected, --quiet, --skip, --verbose, -f, -h, -q, -v. Valued: --pip-args
- Allowed standalone flags: --help, --version, -h

### `poetry`
<p class="cmd-url"><a href="https://python-poetry.org/docs/cli/">https://python-poetry.org/docs/cli/</a></p>

- **check**: Flags: --help, --lock, -h
- **env info**: Flags: --full-path, --help, -h
- **env list**: Flags: --full-path, --help, -h
- **show**: Flags: --all, --help, --latest, --no-dev, --outdated, --top-level, --tree, -T, -h, -l, -o. Valued: --why
- Allowed standalone flags: --help, --version, -V, -h

### `pre-commit`
<p class="cmd-url"><a href="https://pre-commit.com/">https://pre-commit.com/</a></p>

- **autoupdate**: Flags: --bleeding-edge, --config, --dry-run, --freeze, --help, --jobs, -c, -h, -j. Valued: --repo
- **clean**: Flags: --help, -h
- **gc**: Flags: --help, -h
- **help**: Positional args accepted
- **install**: Flags: --allow-missing-config, --config, --help, --install-hooks, --overwrite, -c, -f, -h. Valued: --hook-type, -t
- **install-hooks**: Flags: --config, --help, -c, -h
- **sample-config**: Flags: --help, -h
- **uninstall**: Flags: --config, --help, -c, -h. Valued: --hook-type, -t
- **validate-config**: Flags: --help, -h. Positional args accepted
- **validate-manifest**: Flags: --help, -h. Positional args accepted
- Allowed standalone flags: --help, --version, -h

### `py-spy`
<p class="cmd-url"><a href="https://github.com/benfred/py-spy">https://github.com/benfred/py-spy</a></p>

- Allowed standalone flags: --help, --version, -h, -V

### `pycodestyle`
<p class="cmd-url"><a href="https://github.com/PyCQA/pycodestyle">https://github.com/PyCQA/pycodestyle</a></p>

- Allowed standalone flags: --benchmark, --count, --diff, --first, --help, --quiet, --show-pep8, --show-source, --statistics, --testsuite, --verbose, --version, -h, -q, -v
- Allowed valued flags: --config, --exclude, --filename, --format, --ignore, --max-doc-length, --max-line-length, --select

### `pydocstyle`
<p class="cmd-url"><a href="https://github.com/PyCQA/pydocstyle">https://github.com/PyCQA/pydocstyle</a></p>

- Allowed standalone flags: --count, --debug, --explain, --help, --source, --verbose, --version, -d, -e, -h, -s, -v
- Allowed valued flags: --add-ignore, --add-select, --config, --convention, --ignore, --ignore-decorators, --ignore-self-only-init, --match, --match-dir, --property-decorators, --select
- Bare invocation allowed

### `pyenv`
<p class="cmd-url"><a href="https://github.com/pyenv/pyenv#readme">https://github.com/pyenv/pyenv#readme</a></p>

- **commands**: Flags: --bare, --help, -h
- **completions**: Flags: --bare, --help, -h
- **global**: Flags: --help, -h
- **help**: Flags: --bare, --help, -h
- **hooks**: Flags: --bare, --help, -h
- **init** (requires -, --path): Flags: -, --help, --no-push-path, --no-rehash, --path, -h
- **install**: Flags: --debug, --force, --help, --keep, --list, --patch, --skip-existing, --verbose, --version, -f, -g, -h, -k, -l, -p, -s, -v
- **local**: Flags: --force, --help, --unset, -f, -h
- **prefix**: Flags: --bare, --help, -h
- **rehash**: Flags: --help, -h
- **root**: Flags: --bare, --help, -h
- **shell**: Flags: --help, --unset, -h
- **shims**: Flags: --bare, --help, -h
- **uninstall**: Flags: --force, --help, -f, -h
- **version**: Flags: --bare, --help, -h
- **version-file**: Flags: --bare, --help, -h
- **version-name**: Flags: --bare, --help, -h
- **version-origin**: Flags: --bare, --help, -h
- **versions**: Flags: --bare, --help, --skip-aliases, -h
- **whence**: Flags: --bare, --help, -h
- **which**: Flags: --bare, --help, -h
- Allowed standalone flags: --help, --version, -V, -h

**Examples:**

- `pyenv versions`
- `pyenv version`
- `pyenv which python`
- `eval "$(pyenv init -)"`
- `eval "$(pyenv init - bash)"`
- `eval "$(pyenv init --path zsh --no-rehash)"`

### `pyflakes`
<p class="cmd-url"><a href="https://github.com/PyCQA/pyflakes">https://github.com/PyCQA/pyflakes</a></p>

- Allowed standalone flags: --help, --version, -h
- Bare invocation allowed

### `pytest`
<p class="cmd-url"><a href="https://docs.pytest.org/">https://docs.pytest.org/</a></p>

- Allowed standalone flags: --cache-clear, --cache-show, --co, --collect-only, --doctest-modules, --help, --last-failed, --lf, --markers, --new-first, --nf, --no-header, --quiet, --showlocals, --stepwise, --strict-markers, --verbose, --version, -h, -l, -q, -v, -x
- Allowed valued flags: --basetemp, --color, --confcutdir, --deselect, --durations, --ignore, --import-mode, --junitxml, --log-cli-level, --maxfail, --override-ini, --rootdir, --timeout, -c, -k, -m, -o, -p, -r, -W
- Bare invocation allowed

### `pyupgrade`
<p class="cmd-url"><a href="https://github.com/asottile/pyupgrade">https://github.com/asottile/pyupgrade</a></p>

- Allowed standalone flags: --exit-zero-even-if-changed, --help, --keep-mock, --keep-percent-format, --keep-runtime-typing, --py3-only, --py3-plus, --py35-plus, --py36-plus, --py37-plus, --py38-plus, --py39-plus, --py310-plus, --py311-plus, --py312-plus, --py313-plus, -h

### `safety`
<p class="cmd-url"><a href="https://docs.safetycli.com/">https://docs.safetycli.com/</a></p>

- **check**: Flags: --bare, --continue-on-error, --exit-code, --full-report, --help, --ignore-unpinned-requirements, --json, --no-cache, --policy-file, --proxy-required, --save-html, --save-json, --short-report, -h. Valued: --api, --cache, --db, --exclude, --file, --ignore, --key, --output, --proxy-host, --proxy-port, --proxy-protocol, -i, -o, -r
- **help**: Positional args accepted
- **scan**: Flags: --apply-remediations, --detailed-output, --disable-optional-telemetry, --help, --no-fix-suggestion, -h. Valued: --auth-type, --key, --output, --policy-file, --save-as, --target
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

### `scalene`
<p class="cmd-url"><a href="https://github.com/plasma-umass/scalene">https://github.com/plasma-umass/scalene</a></p>

- Allowed standalone flags: --help, --version, -h

### `sphinx-apidoc`
<p class="cmd-url"><a href="https://www.sphinx-doc.org/en/master/man/sphinx-apidoc.html">https://www.sphinx-doc.org/en/master/man/sphinx-apidoc.html</a></p>

- Allowed standalone flags: --ext-autodoc, --ext-coverage, --ext-doctest, --ext-githubpages, --ext-ifconfig, --ext-mathjax, --ext-viewcode, --follow-links, --force, --help, --implicit-namespaces, --include-private, --module-first, --no-headings, --no-toc, --quiet, --remove-old, --separate, --tocfile, --version, -E, -F, -M, -T, -e, -f, -h, -l, -n, -o, -q, -s, -t
- Allowed valued flags: --author, --dot, --extension, --maxdepth, --release, --suffix, --tocfile-name, --templatedir, --version, -A, -H, -R, -V, -d

### `sphinx-build`
<p class="cmd-url"><a href="https://www.sphinx-doc.org/">https://www.sphinx-doc.org/</a></p>

- Allowed standalone flags: --builders, --color, --exception-on-warning, --fail-on-warning, --fresh-env, --full-traceback, --help, --keep-going, --no-color, --quiet, --really-quiet, --show-traceback, --silent, --verbose, --version, --write-all, -E, -M, -N, -P, -Q, -T, -W, -a, -b, -h, -n, -q, -v
- Allowed valued flags: --builder, --conf-dir, --define, --doctree-dir, --filenames, --html-define, --include, --isolated, --jobs, --language, --name-suffix, --nitpicky, --pdb, --tag, --warning-file, -A, -D, -c, -d, -j, -t, -w

### `sphinx-quickstart`
<p class="cmd-url"><a href="https://www.sphinx-doc.org/en/master/man/sphinx-quickstart.html">https://www.sphinx-doc.org/en/master/man/sphinx-quickstart.html</a></p>

- Allowed standalone flags: --ext-autodoc, --ext-coverage, --ext-doctest, --ext-githubpages, --ext-ifconfig, --ext-imgmath, --ext-intersphinx, --ext-mathjax, --ext-todo, --ext-viewcode, --help, --makefile, --no-batchfile, --no-makefile, --no-prompt, --no-sep, --quiet, --sep, --use-make-mode, --version, -h, -q
- Allowed valued flags: --author, --dot, --ext, --extensions, --language, --master, --project, --release, --suffix, --templatedir, --version, -a, -d, -l, -p, -r, -t, -v

### `tox`
<p class="cmd-url"><a href="https://tox.wiki/">https://tox.wiki/</a></p>

- **config**: Flags: --core, --help, -h
- **list**: Flags: --help, --no-desc, -d, -h
- **run**: Flags: --help, --no-recreate-pkg, --skip-missing-interpreters, -h. Valued: -e, -f, --override, --result-json
- Allowed standalone flags: --help, --version, -h

### `twine`
<p class="cmd-url"><a href="https://twine.readthedocs.io/">https://twine.readthedocs.io/</a></p>

- **check**: Flags: --help, --strict, -h. Positional args accepted
- **help**: Positional args accepted
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

### `vulture`
<p class="cmd-url"><a href="https://github.com/jendrikseipp/vulture">https://github.com/jendrikseipp/vulture</a></p>

- Allowed standalone flags: --help, --ignore-decorators, --ignore-names, --make-whitelist, --sort-by-size, --verbose, --version, -h, -v
- Allowed valued flags: --config, --exclude, --min-confidence

### `wandb`
<p class="cmd-url"><a href="https://docs.wandb.ai/ref/cli/">https://docs.wandb.ai/ref/cli/</a></p>

- **completion**: Flags: --help, -h. Positional args accepted
- **help**: Positional args accepted
- **status**: Flags: --help, --settings, -h
- **verify**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h, -v

### `yapf`
<p class="cmd-url"><a href="https://github.com/google/yapf">https://github.com/google/yapf</a></p>

- Allowed standalone flags: --diff, --help, --in-place, --no-local-style, --parallel, --print-modified, --quiet, --recursive, --verify, --version, -d, -h, -i, -m, -p, -q, -r, -vv
- Allowed valued flags: --exclude, --lines, --style, -e, -l
- Bare invocation allowed

