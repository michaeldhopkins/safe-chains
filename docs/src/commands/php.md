# PHP

### `composer`
<p class="cmd-url"><a href="https://getcomposer.org/doc/03-cli.md">https://getcomposer.org/doc/03-cli.md</a></p>

- **about**: Flags: --help, -h
- **audit**: Flags: --abandoned, --help, --locked, --no-dev, -h. Valued: --format, -f
- **check-platform-reqs**: Flags: --help, -h
- **config** (requires --list, -l): Flags: --global, --help, --list, --source, -g, -h, -l
- **depends**: Flags: --help, --recursive, --tree, -h, -r, -t
- **diagnose**: Flags: --help, -h
- **dump-autoload**: Flags: --apcu, --classmap-authoritative, --dev, --dry-run, --help, --ignore-platform-reqs, --no-dev, --optimize, --strict-ambiguous, --strict-psr, -a, -h, -o. Valued: --apcu-prefix, --ignore-platform-req
- **fund**: Flags: --help, -h
- **help**: Flags: --help, -h
- **info**: Flags: --all, --available, --direct, --help, --installed, --latest, --locked, --minor-only, --name-only, --no-dev, --outdated, --path, --platform, --self, --strict, --tree, --versions, -D, -H, -N, -P, -a, -h, -i, -l, -o, -s, -t. Valued: --format, --ignore, -f
- **install**: Flags: --ansi, --apcu-autoloader, --audit, --classmap-authoritative, --dev, --download-only, --dry-run, --help, --ignore-platform-reqs, --no-ansi, --no-autoloader, --no-cache, --no-dev, --no-interaction, --no-plugins, --no-progress, --no-scripts, --no-security-blocking, --no-source-fallback, --optimize-autoloader, --quiet, --source-fallback, --strict-psr-autoloader, --verbose, -a, -h, -n, -o, -q, -v, -vv, -vvv. Valued: --apcu-autoloader-prefix, --audit-format, --ignore-platform-req, --prefer-install
- **licenses**: Flags: --help, -h
- **outdated**: Flags: --all, --direct, --help, --locked, --minor-only, --no-dev, --strict, -D, -a, -h, -m. Valued: --format, --ignore, -f
- **prohibits**: Flags: --help, --recursive, --tree, -h, -r, -t
- **search**: Flags: --help, --only-name, --only-vendor, -N, -O, -h. Valued: --format, --type, -f, -t
- **show**: Flags: --all, --available, --direct, --help, --installed, --latest, --locked, --minor-only, --name-only, --no-dev, --outdated, --path, --platform, --self, --strict, --tree, --versions, -D, -H, -N, -P, -a, -h, -i, -l, -o, -s, -t. Valued: --format, --ignore, -f
- **suggests**: Flags: --help, -h
- **validate**: Flags: --check-lock, --help, --no-check-all, --no-check-lock, --no-check-publish, --no-check-version, --strict, --with-dependencies, -h
- **why**: Flags: --help, --recursive, --tree, -h, -r, -t
- **why-not**: Flags: --help, --recursive, --tree, -h, -r, -t
- Allowed standalone flags: --help, --version, -V, -h

### `craft`
<p class="cmd-url"><a href="https://craftcms.com/docs/5.x/reference/cli.html">https://craftcms.com/docs/5.x/reference/cli.html</a></p>

- **env/show**: Flags: --help, -h
- **graphql/list-schemas**: Flags: --help, -h
- **graphql/print-schema**: Flags: --help, -h
- **help**: Flags: --help, -h
- **install/check**: Flags: --help, -h
- **migrate/history**: Flags: --help, -h
- **migrate/new**: Flags: --help, -h
- **pc/diff**: Flags: --help, -h
- **pc/export**: Flags: --help, -h
- **pc/get**: Flags: --help, -h
- **plugin/list**: Flags: --help, -h
- **queue/info**: Flags: --help, -h
- **update/info**: Flags: --help, -h
- **users/list-admins**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

### `pest`
<p class="cmd-url"><a href="https://pestphp.com/docs/cli-api-reference">https://pestphp.com/docs/cli-api-reference</a></p>

- Allowed standalone flags: --bail, --cache-result, --ci, --compact, --debug, --dirty, --display-deprecations, --display-errors, --display-incomplete, --display-notices, --display-phpunit-deprecations, --display-skipped, --display-warnings, --do-not-cache-result, --dont-report-useless-tests, --enforce-time-limit, --fail-on-deprecation, --fail-on-empty-test-suite, --fail-on-incomplete, --fail-on-notice, --fail-on-phpunit-deprecation, --fail-on-risky, --fail-on-skipped, --fail-on-warning, --flaky, --globals-backup, --help, --list-groups, --list-suites, --list-test-files, --list-tests, --mutate, --no-configuration, --no-coverage, --no-extensions, --no-logging, --no-output, --no-progress, --no-results, --notes, --parallel, --path-coverage, --profile, --retry, --reverse-list, --static-backup, --stderr, --stop-on-defect, --stop-on-deprecation, --stop-on-error, --stop-on-failure, --stop-on-incomplete, --stop-on-notice, --stop-on-risky, --stop-on-skipped, --stop-on-warning, --strict-coverage, --strict-global-state, --teamcity, --testdox, --testdox-summary, --todos, --warm-coverage-cache, -h
- Allowed valued flags: --bootstrap, --cache-directory, --colors, --columns, --configuration, --coverage, --coverage-clover, --coverage-cobertura, --coverage-crap4j, --coverage-filter, --coverage-html, --coverage-php, --coverage-text, --coverage-xml, --covers, --default-time-limit, --exclude-filter, --exclude-group, --exclude-testsuite, --extension, --filter, --generate-baseline, --group, --include-path, --log-events-text, --log-events-verbose-text, --log-junit, --log-teamcity, --order-by, --random-order-seed, --requires-php-extension, --shard, --test-directory, --test-suffix, --testdox-html, --testdox-text, --testsuite, --use-baseline, --uses, -c, -d
- Bare invocation allowed

### `phpstan`
<p class="cmd-url"><a href="https://phpstan.org/user-guide/command-line-usage">https://phpstan.org/user-guide/command-line-usage</a></p>

- **analyse**: Flags: --allow-empty-baseline, --ansi, --debug, --help, --no-ansi, --no-progress, --quiet, --verbose, --version, --xdebug, -V, -h, -q, -v, -vv, -vvv. Valued: --autoload-file, --configuration, --error-format, --generate-baseline, --level, --memory-limit, --tmp-file, --instead-of, --use-baseline, -a, -b, -c, -l
- **clear-result-cache**: Flags: --debug, --help, --quiet, --version, -V, -h, -q. Valued: --autoload-file, --configuration, --memory-limit, -a, -c
- **diagnose**: Flags: --debug, --help, -h. Valued: --autoload-file, --configuration, --level, --memory-limit, -a, -c, -l
- Allowed standalone flags: --help, --version, -V, -h

### `phpunit`
<p class="cmd-url"><a href="https://docs.phpunit.de/">https://docs.phpunit.de/</a></p>

- Allowed standalone flags: --cache-result, --check-version, --debug, --disable-coverage-ignore, --disallow-test-output, --display-all-issues, --display-deprecations, --display-errors, --display-incomplete, --display-notices, --display-phpunit-deprecations, --display-phpunit-notices, --display-skipped, --display-warnings, --do-not-cache-result, --do-not-report-useless-tests, --enforce-time-limit, --fail-on-all-issues, --fail-on-deprecation, --fail-on-empty-test-suite, --fail-on-incomplete, --fail-on-notice, --fail-on-phpunit-deprecation, --fail-on-phpunit-notice, --fail-on-phpunit-warning, --fail-on-risky, --fail-on-skipped, --fail-on-warning, --globals-backup, --help, --ignore-baseline, --ignore-dependencies, --list-groups, --list-suites, --list-test-files, --list-test-ids, --list-tests, --no-configuration, --no-coverage, --no-extensions, --no-logging, --no-output, --no-progress, --no-results, --path-coverage, --process-isolation, --random-order, --resolve-dependencies, --reverse-list, --reverse-order, --static-backup, --stderr, --strict-coverage, --strict-global-state, --stop-on-defect, --stop-on-deprecation, --stop-on-error, --stop-on-failure, --stop-on-incomplete, --stop-on-notice, --stop-on-risky, --stop-on-skipped, --stop-on-warning, --teamcity, --testdox, --testdox-summary, --validate-configuration, --version, --warm-coverage-cache, --with-telemetry, -h
- Allowed valued flags: --bootstrap, --cache-directory, --colors, --columns, --configuration, --coverage-clover, --coverage-cobertura, --coverage-crap4j, --coverage-filter, --coverage-html, --coverage-openclover, --coverage-php, --coverage-text, --coverage-xml, --covers, --default-time-limit, --diff-context, --exclude-filter, --exclude-group, --exclude-testsuite, --extension, --filter, --generate-baseline, --group, --include-path, --log-events-text, --log-events-verbose-text, --log-junit, --log-otr, --log-teamcity, --order-by, --random-order-seed, --requires-php-extension, --run-test-id, --test-suffix, --testdox-html, --testdox-text, --testsuite, --use-baseline, --uses, -c, -d
- Bare invocation allowed

