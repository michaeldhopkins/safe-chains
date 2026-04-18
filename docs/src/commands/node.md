# Node.js

### `bun`
<p class="cmd-url"><a href="https://bun.sh/docs/cli">https://bun.sh/docs/cli</a></p>

- **build**: Flags: --bytecode, --compile, --css-chunking, --emit-dce-annotations, --help, --minify, --minify-identifiers, --minify-syntax, --minify-whitespace, --no-bundle, --no-clear-screen, --production, --react-fast-refresh, --splitting, --watch, --windows-hide-console, -h. Valued: --asset-naming, --banner, --chunk-naming, --conditions, --entry-naming, --env, --external, --footer, --format, --outdir, --outfile, --packages, --public-path, --root, --sourcemap, --target, --windows-icon, -e
- **outdated**: Flags: --help, -h
- **pm bin**: Flags: --help, -h
- **pm cache**: Flags: --help, -h
- **pm hash**: Flags: --help, -h
- **pm ls**: Flags: --help, -h
- **test**: Flags: --bail, --help, --only, --rerun-each, --todo, -h. Valued: --preload, --timeout, -t
- Allowed standalone flags: --help, --version, -V, -h

### `bunx`
<p class="cmd-url"><a href="https://bun.sh/docs/cli/bunx">https://bun.sh/docs/cli/bunx</a></p>

- Delegates to the inner command's safety rules.
- Skips flags: --bun/--no-install/--package/-p.

### `deno`
<p class="cmd-url"><a href="https://docs.deno.com/runtime/reference/cli/">https://docs.deno.com/runtime/reference/cli/</a></p>

- **check**: Flags: --help, --json, --no-lock, --quiet, --unstable, -h, -q. Valued: --config, --import-map, -c
- **doc**: Flags: --help, --json, --no-lock, --quiet, --unstable, -h, -q. Valued: --config, --import-map, -c
- **fmt** (requires --check): Flags: --check, --help, --no-semicolons, --single-quote, --unstable, -h, -q. Valued: --config, --ext, --ignore, --indent-width, --line-width, --log-level, --prose-wrap, -c
- **info**: Flags: --help, --json, --no-lock, --quiet, --unstable, -h, -q. Valued: --config, --import-map, -c
- **lint**: Flags: --help, --json, --no-lock, --quiet, --unstable, -h, -q. Valued: --config, --import-map, -c
- **test**: Flags: --help, --json, --no-lock, --quiet, --unstable, -h, -q. Valued: --config, --import-map, -c
- Allowed standalone flags: --help, --version, -V, -h

### `fnm`
<p class="cmd-url"><a href="https://github.com/Schniz/fnm#readme">https://github.com/Schniz/fnm#readme</a></p>

- **current**: Flags: --help, -h
- **default**: Flags: --help, -h
- **list**: Flags: --help, -h
- **ls-remote**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

### `jest`
<p class="cmd-url"><a href="https://jestjs.io/docs/cli">https://jestjs.io/docs/cli</a></p>

- Allowed standalone flags: --all, --bail, --changedFilesWithAncestor, --ci, --clearCache, --clearMocks, --collectCoverage, --colors, --coverage, --debug, --detectOpenHandles, --errorOnDeprecated, --expand, --forceExit, --help, --json, --lastCommit, --listTests, --logHeapUsage, --noCache, --noStackTrace, --onlyChanged, --passWithNoTests, --runInBand, --showConfig, --silent, --verbose, --version, -b, -e, -h, -i, -o, -u, -w
- Allowed valued flags: --changedSince, --collectCoverageFrom, --config, --coverageDirectory, --coverageProvider, --filter, --maxConcurrency, --maxWorkers, --outputFile, --projects, --reporters, --roots, --shard, --testMatch, --testNamePattern, --testPathPattern, --testRunner, --testTimeout, -c, -t
- Bare invocation allowed

### `mocha`
<p class="cmd-url"><a href="https://mochajs.org/">https://mochajs.org/</a></p>

- Allowed standalone flags: --bail, --check-leaks, --color, --diff, --dry-run, --exit, --forbid-only, --forbid-pending, --full-trace, --help, --inline-diffs, --invert, --list-files, --list-reporters, --no-color, --no-diff, --no-timeouts, --parallel, --quiet, --recursive, --sort, --version, -A, -R, -V, -b, -c, -h
- Allowed valued flags: --config, --delay, --extension, --fgrep, --file, --grep, --ignore, --jobs, --node-option, --package, --reporter, --reporter-option, --require, --retries, --slow, --spec, --timeout, --ui, -f, -g, -j, -n, -r, -s, -t, -u
- Bare invocation allowed

### `next`
<p class="cmd-url"><a href="https://nextjs.org/docs/api-reference/cli">https://nextjs.org/docs/api-reference/cli</a></p>

- **build**: Flags: --debug, --help, --lint, --no-lint, --profile, -d, -h
- **info**: Flags: --help, -h
- **lint**: Flags: --dir, --help, --quiet, --strict, -d, -h. Valued: --cache-location, --ext, --max-warnings, --output-file, --resolve-plugins-relative-to, -c
- Allowed standalone flags: --help, --version, -h

### `npm`
<p class="cmd-url"><a href="https://docs.npmjs.com/cli">https://docs.npmjs.com/cli</a></p>

- **audit**: Flags: --help, --json, --omit, --production, -h. Valued: --audit-level
- **ci**: Flags: --help, --ignore-scripts, --legacy-bundling, --no-audit, --no-fund, --no-optional, --production, -h
- **config get**: Flags: --help, --json, --long, -h, -l
- **config list**: Flags: --help, --json, --long, -h, -l
- **doctor**: Flags: --help, --json, -h
- **explain**: Flags: --help, --json, -h
- **fund**: Flags: --help, --json, -h
- **info**: Flags: --help, --json, -h
- **list**: Flags: --all, --help, --json, --link, --long, --omit, --parseable, --production, --unicode, -a, -h, -l. Valued: --depth, --prefix
- **ls**: Flags: --all, --help, --json, --link, --long, --omit, --parseable, --production, --unicode, -a, -h, -l. Valued: --depth, --prefix
- **outdated**: Flags: --help, --json, -h
- **prefix**: Flags: --help, --json, -h
- **root**: Flags: --help, --json, -h
- **run**: Allowed arguments: test, test:*
- **run-script**: Allowed arguments: test, test:*
- **test**: Flags: --help, -h
- **view**: Flags: --help, --json, -h
- **why**: Flags: --help, --json, -h
- Allowed standalone flags: --help, --version, -V, -h

### `npx`
<p class="cmd-url"><a href="https://docs.npmjs.com/cli/commands/npx">https://docs.npmjs.com/cli/commands/npx</a></p>

- Delegates to the inner command's safety rules.
- Skips flags: --yes/-y/--no/--package/-p.

### `nvm`
<p class="cmd-url"><a href="https://github.com/nvm-sh/nvm#readme">https://github.com/nvm-sh/nvm#readme</a></p>

- **current**: Flags: --help, --lts, --no-colors, -h
- **list**: Flags: --help, --lts, --no-colors, -h
- **ls**: Flags: --help, --lts, --no-colors, -h
- **ls-remote**: Flags: --help, --lts, --no-colors, -h
- **version**: Flags: --help, --lts, --no-colors, -h
- **which**: Flags: --help, --lts, --no-colors, -h
- Allowed standalone flags: --help, --version, -V, -h

### `nx`
<p class="cmd-url"><a href="https://nx.dev/reference/commands">https://nx.dev/reference/commands</a></p>

- **format** (requires --check): Flags: --all, --base, --check, --head, --help, --uncommitted, --untracked, -h. Valued: --exclude, --files, --projects
- **graph**: Flags: --affected, --help, --open, --watch, -h. Valued: --file, --focus, --groupByFolder, --host, --port, --targets, --view
- **list**: Flags: --help, -h. Valued: --plugin
- **print-affected**: Flags: --all, --base, --head, --help, --select, --uncommitted, --untracked, -h. Valued: --exclude, --files, --type
- **report**: Flags: --help, -h
- **show**: Flags: --help, --json, -h. Valued: --projects, --type, --web
- Allowed standalone flags: --help, --version, -h

### `pnpm`
<p class="cmd-url"><a href="https://pnpm.io/pnpm-cli">https://pnpm.io/pnpm-cli</a></p>

- **audit**: Flags: --help, --json, --recursive, -h, -r. Valued: --filter
- **list**: Flags: --dev, --help, --json, --long, --no-optional, --parseable, --production, --recursive, -P, -h, -r. Valued: --depth, --filter
- **ls**: Flags: --dev, --help, --json, --long, --no-optional, --parseable, --production, --recursive, -P, -h, -r. Valued: --depth, --filter
- **outdated**: Flags: --help, --json, --recursive, -h, -r. Valued: --filter
- **why**: Flags: --help, --json, --recursive, -h, -r. Valued: --filter
- Allowed standalone flags: --help, --version, -V, -h

### `turbo`
<p class="cmd-url"><a href="https://turbo.build/repo/docs/reference/command-line-reference">https://turbo.build/repo/docs/reference/command-line-reference</a></p>

- **daemon status**: Flags: --help, -h
- **ls**: Flags: --affected, --help, -h. Valued: --filter, -F
- **prune**: Flags: --docker, --help, -h. Valued: --out-dir, --scope
- **query**: Flags: --help, -h
- **run**: Flags: --affected, --cache-dir, --continue, --dry-run, --env-mode, --force, --framework-inference, --graph, --help, --no-cache, --no-daemon, --output-logs, --parallel, --summarize, --verbose, -h. Valued: --cache-workers, --color, --concurrency, --env-mode, --filter, --global-deps, --graph, --log-order, --log-prefix, --output-logs, --profile, --remote-only, --scope, --team, --token, -F. Positional args accepted
- Allowed standalone flags: --help, --version, -h

### `vitest`
<p class="cmd-url"><a href="https://vitest.dev/guide/cli.html">https://vitest.dev/guide/cli.html</a></p>

- Allowed standalone flags: --bail, --changed, --coverage, --dom, --globals, --help, --hideSkippedTests, --no-color, --no-isolate, --passWithNoTests, --reporter, --run, --silent, --ui, --update, --version, --watch, -h, -v
- Allowed valued flags: --color, --config, --dir, --environment, --exclude, --include, --maxConcurrency, --mode, --project, --root, --testTimeout, -c, -r, -t
- Bare invocation allowed

### `volta`
<p class="cmd-url"><a href="https://docs.volta.sh/reference">https://docs.volta.sh/reference</a></p>

- **list**: Flags: --current, --default, --help, -c, -d, -h. Valued: --format
- **which**: Flags: --current, --default, --help, -c, -d, -h. Valued: --format
- Allowed standalone flags: --help, --version, -V, -h

### `yarn`
<p class="cmd-url"><a href="https://yarnpkg.com/cli">https://yarnpkg.com/cli</a></p>

- **info**: Flags: --help, --json, -h
- **list**: Flags: --help, --json, --long, --production, -h. Valued: --depth, --pattern
- **ls**: Flags: --help, --json, --long, --production, -h. Valued: --depth, --pattern
- **why**: Flags: --help, --json, -h
- Allowed arguments: test, test:*
- Allowed standalone flags: --help, --version, -V, -h

