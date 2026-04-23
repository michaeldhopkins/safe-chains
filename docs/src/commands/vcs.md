# Version Control

### `git`
<p class="cmd-url"><a href="https://git-scm.com/docs">https://git-scm.com/docs</a></p>

- **blame**: Flags: --color-by-age, --color-lines, --help, --incremental, --line-porcelain, --minimal, --porcelain, --progress, --root, --show-email, --show-name, --show-number, --show-stats, -b, -c, -e, -f, -h, -k, -l, -n, -p, -s, -t, -w. Valued: --abbrev, --contents, --ignore-rev, --ignore-revs-file, -C, -L, -M, -S
- **branch**: Flags: --all, --help, --ignore-case, --list, --no-abbrev, --no-color, --no-column, --omit-empty, --remotes, --show-current, --verbose, -a, -h, -i, -l, -r, -v, -vv. Valued: --abbrev, --color, --column, --contains, --format, --merged, --no-contains, --no-merged, --points-at, --sort
- **cat-file**: Flags: --batch-all-objects, --buffer, --filters, --follow-symlinks, --help, --mailmap, --textconv, --unordered, --use-mailmap, -Z, -e, -h, -p, -s, -t. Valued: --batch, --batch-check, --batch-command, --filter, --path
- **check-ignore**: Flags: --help, --no-index, --non-matching, --quiet, --stdin, --verbose, -h, -n, -q, -v, -z
- **cliff** (requires --no-exec): Flags: --bumped-version, --current, --help, --latest, --no-exec, --offline, --topo-order, --unreleased, --use-branch-tags, --verbose, --version, -V, -h, -l, -u, -v. Valued: --body, --bump, --config, --count-tags, --exclude-path, --from-context, --ignore-tags, --include-path, --repository, --skip-commit, --skip-tags, --sort, --strip, --tag, --tag-pattern, --with-commit, --with-tag-message, --workdir, -b, -c, -r, -s, -t, -w
- **config --get**
- **config --get-all**
- **config --get-regexp**
- **config --help**
- **config --list**
- **config -h**
- **config -l**
- **config**: Flags: --global, --local, --name-only, --show-origin, --show-scope, --system, --worktree, -z. Valued: --blob, --file, -f
- **count-objects**: Flags: --help, --human-readable, --verbose, -H, -h, -v
- **describe**: Flags: --all, --always, --contains, --debug, --exact-match, --first-parent, --help, --long, --tags, -h. Valued: --abbrev, --broken, --candidates, --dirty, --exclude, --match
- **diff**: Flags: --cached, --cc, --check, --color-words, --combined-all-paths, --compact-summary, --cumulative, --dirstat-by-file, --exit-code, --find-copies, --find-copies-harder, --find-renames, --first-parent, --follow, --full-index, --help, --histogram, --ignore-all-space, --ignore-blank-lines, --ignore-cr-at-eol, --ignore-space-at-eol, --ignore-space-change, --ignore-submodules, --merge-base, --minimal, --name-only, --name-status, --no-color, --no-ext-diff, --no-index, --no-patch, --no-prefix, --no-renames, --no-textconv, --numstat, --ours, --patch, --patch-with-raw, --patch-with-stat, --patience, --pickaxe-all, --quiet, --raw, --shortstat, --staged, --stat, --summary, --text, --textconv, --theirs, -B, -C, -M, -R, -a, -b, -h, -p, -q, -u, -w, -z. Valued: --abbrev, --color, --color-moved, --diff-algorithm, --diff-filter, --diff-merges, --dirstat, --dst-prefix, --inter-hunk-context, --line-prefix, --output-indicator-new, --output-indicator-old, --relative, --src-prefix, --stat-count, --stat-graph-width, --stat-name-width, --stat-width, --submodule, --unified, --word-diff, --word-diff-regex, -G, -O, -S, -U
- **diff-tree**: Flags: --cc, --combined-all-paths, --find-copies-harder, --full-index, --help, --ignore-all-space, --ignore-space-at-eol, --ignore-space-change, --merge-base, --minimal, --name-only, --name-status, --no-commit-id, --no-ext-diff, --no-patch, --no-renames, --numstat, --patch, --patch-with-raw, --patch-with-stat, --pickaxe-all, --raw, --root, --shortstat, --stat, --stdin, --summary, --text, -B, -C, -M, -R, -a, -c, -h, -m, -p, -r, -s, -t, -u, -v, -z. Valued: --abbrev, --diff-algorithm, --diff-filter, --pretty, -O, -S
- **fetch**: Flags: --all, --append, --atomic, --dry-run, --force, --help, --ipv4, --ipv6, --keep, --multiple, --negotiate-only, --no-auto-gc, --no-auto-maintenance, --no-show-forced-updates, --no-tags, --no-write-fetch-head, --porcelain, --prefetch, --progress, --prune, --prune-tags, --quiet, --refetch, --set-upstream, --show-forced-updates, --stdin, --tags, --unshallow, --update-head-ok, --update-shallow, --verbose, --write-commit-graph, --write-fetch-head, -4, -6, -P, -a, -f, -h, -k, -m, -n, -p, -q, -t, -u, -v. Valued: --deepen, --depth, --filter, --jobs, --negotiation-tip, --recurse-submodules, --refmap, --server-option, --shallow-exclude, --shallow-since, -j, -o
- **for-each-ref**: Flags: --help, --ignore-case, --include-root-refs, --omit-empty, --perl, --python, --shell, --stdin, --tcl, -h, -p, -s. Valued: --color, --contains, --count, --exclude, --format, --merged, --no-contains, --no-merged, --points-at, --sort
- **grep**: Flags: --all-match, --and, --basic-regexp, --break, --cached, --column, --count, --exclude-standard, --extended-regexp, --files-with-matches, --files-without-match, --fixed-strings, --full-name, --function-context, --heading, --help, --ignore-case, --index, --invert-match, --line-number, --name-only, --no-color, --no-index, --null, --only-matching, --perl-regexp, --quiet, --recurse-submodules, --recursive, --show-function, --text, --textconv, --untracked, --word-regexp, -E, -F, -G, -H, -I, -L, -P, -W, -a, -c, -h, -i, -l, -n, -o, -p, -q, -r, -v, -w, -z. Valued: --after-context, --before-context, --color, --context, --max-count, --max-depth, --open-files-in-pager, --threads, -A, -B, -C, -O, -e, -f, -m
- **help**: Positional args accepted
- **log**: Flags: --abbrev-commit, --all, --ancestry-path, --author-date-order, --bisect, --boundary, --branches, --cherry, --cherry-mark, --cherry-pick, --children, --clear-decorations, --compact-summary, --cumulative, --date-order, --dense, --do-walk, --early-output, --first-parent, --follow, --full-diff, --full-history, --graph, --help, --ignore-missing, --invert-grep, --left-only, --left-right, --log-size, --mailmap, --merges, --minimal, --name-only, --name-status, --no-abbrev-commit, --no-color, --no-decorate, --no-expand-tabs, --no-ext-diff, --no-merges, --no-notes, --no-patch, --no-prefix, --no-renames, --no-walk, --numstat, --oneline, --parents, --patch, --patch-with-raw, --patch-with-stat, --patience, --pickaxe-all, --pickaxe-regex, --raw, --reflog, --regexp-ignore-case, --relative-date, --remotes, --reverse, --shortstat, --show-linear-break, --show-notes, --show-pulls, --show-signature, --simplify-by-decoration, --simplify-merges, --source, --sparse, --stat, --stdin, --summary, --tags, --text, --topo-order, --use-mailmap, -0, -1, -2, -3, -4, -5, -6, -7, -8, -9, -h, -i, -p, -q, -s, -u. Valued: --abbrev, --after, --author, --before, --color, --committer, --date, --decorate, --decorate-refs, --decorate-refs-exclude, --diff-algorithm, --diff-filter, --diff-merges, --encoding, --exclude, --format, --glob, --grep, --max-count, --max-parents, --min-parents, --pretty, --since, --skip, --until, -G, -L, -S, -n
- **ls-files**: Flags: --cached, --debug, --deduplicate, --deleted, --directory, --empty-directory, --eol, --error-unmatch, --exclude-standard, --full-name, --help, --ignored, --killed, --modified, --no-empty-directory, --others, --recurse-submodules, --resolve-undo, --sparse, --stage, --unmerged, -c, -d, -f, -h, -i, -k, -m, -o, -r, -s, -t, -u, -v, -z. Valued: --abbrev, --exclude, --exclude-from, --exclude-per-directory, --format, --with-tree, -X, -x
- **ls-remote**: Flags: --branches, --exit-code, --get-url, --help, --quiet, --refs, --symref, --tags, -b, -h, -q, -t. Valued: --server-option, --sort, -o
- **ls-tree**: Flags: --full-name, --full-tree, --help, --long, --name-only, --name-status, --object-only, -d, -h, -l, -r, -t, -z. Valued: --abbrev, --format
- **merge-base**: Flags: --all, --fork-point, --help, --independent, --is-ancestor, --octopus, -a, -h
- **merge-tree**: Flags: --allow-unrelated-histories, --help, --messages, --name-only, --quiet, --stdin, --trivial-merge, --write-tree, -h, -z. Valued: --merge-base, -X
- **name-rev**: Flags: --all, --always, --annotate-stdin, --help, --name-only, --tags, --undefined, -h. Valued: --exclude, --refs
- **notes --help**
- **notes -h**
- **notes list**
- **notes show**
- **reflog**: Flags: --abbrev-commit, --all, --ancestry-path, --author-date-order, --bisect, --boundary, --branches, --cherry, --cherry-mark, --cherry-pick, --children, --clear-decorations, --compact-summary, --cumulative, --date-order, --dense, --do-walk, --early-output, --first-parent, --follow, --full-diff, --full-history, --graph, --help, --ignore-missing, --invert-grep, --left-only, --left-right, --log-size, --mailmap, --merges, --minimal, --name-only, --name-status, --no-abbrev-commit, --no-color, --no-decorate, --no-expand-tabs, --no-ext-diff, --no-merges, --no-notes, --no-patch, --no-prefix, --no-renames, --no-walk, --numstat, --oneline, --parents, --patch, --patch-with-raw, --patch-with-stat, --patience, --pickaxe-all, --pickaxe-regex, --raw, --reflog, --regexp-ignore-case, --relative-date, --remotes, --reverse, --shortstat, --show-linear-break, --show-notes, --show-pulls, --show-signature, --simplify-by-decoration, --simplify-merges, --source, --sparse, --stat, --stdin, --summary, --tags, --text, --topo-order, --use-mailmap, -0, -1, -2, -3, -4, -5, -6, -7, -8, -9, -h, -i, -p, -q, -s, -u. Valued: --abbrev, --after, --author, --before, --color, --committer, --date, --decorate, --decorate-refs, --decorate-refs-exclude, --diff-algorithm, --diff-filter, --diff-merges, --encoding, --exclude, --format, --glob, --grep, --max-count, --max-parents, --min-parents, --pretty, --since, --skip, --until, -G, -L, -S, -n
- **rev-parse**: Flags: --absolute-git-dir, --all, --branches, --git-common-dir, --git-dir, --git-path, --help, --is-bare-repository, --is-inside-git-dir, --is-inside-work-tree, --is-shallow-repository, --local-env-vars, --quiet, --remotes, --shared-index-path, --show-cdup, --show-prefix, --show-superproject-working-tree, --show-toplevel, --symbolic, --symbolic-full-name, --tags, --verify, -h, -q. Valued: --abbrev-ref, --after, --before, --default, --exclude, --glob, --prefix, --resolve-git-dir, --short, --since, --until. Positional args accepted
- **shortlog**: Flags: --committer, --email, --help, --numbered, --summary, -c, -e, -h, -n, -s. Valued: --format, --group
- **show**: Flags: --abbrev-commit, --cc, --color-words, --combined-all-paths, --compact-summary, --cumulative, --expand-tabs, --find-copies, --find-renames, --full-index, --help, --histogram, --ignore-all-space, --ignore-blank-lines, --ignore-space-at-eol, --ignore-space-change, --mailmap, --minimal, --name-only, --name-status, --no-color, --no-ext-diff, --no-notes, --no-patch, --no-prefix, --no-renames, --no-textconv, --numstat, --oneline, --patch, --patch-with-raw, --patch-with-stat, --patience, --pickaxe-all, --pickaxe-regex, --raw, --shortstat, --show-notes, --show-signature, --source, --stat, --summary, --text, --textconv, --use-mailmap, -h, -p, -q, -s, -u, -w. Valued: --abbrev, --color, --color-moved, --decorate, --decorate-refs, --decorate-refs-exclude, --diff-algorithm, --diff-filter, --diff-merges, --encoding, --format, --notes, --pretty, --stat-count, --stat-graph-width, --stat-name-width, --submodule, --word-diff, --word-diff-regex, -G, -O, -S
- **stash --help**: Positional args accepted
- **stash -h**: Positional args accepted
- **stash list**: Positional args accepted
- **stash show**: Flags: --help, --patch, --stat, -h, -p, -u. Positional args accepted
- **status**: Flags: --ahead-behind, --branch, --help, --ignore-submodules, --long, --no-ahead-behind, --no-renames, --null, --renames, --short, --show-stash, --verbose, -b, -h, -s, -v, -z. Valued: --column, --find-renames, --ignored, --porcelain, --untracked-files, -M, -u
- **tag**: Flags: --help, --list, --no-color, --no-column, --verify, -h, -l, -v. Valued: --color, --column, --contains, --format, --merged, --no-contains, --no-merged, --points-at, --sort, -n
- **verify-commit**: Flags: --help, --raw, --verbose, -h, -v
- **verify-tag**: Flags: --help, --raw, --verbose, -h, -v. Valued: --format
- **worktree --help**
- **worktree -h**
- **worktree list**: Flags: --porcelain, --verbose, -v, -z
- Allowed standalone flags: --help, --version, -V, -h

### `git-cliff`
<p class="cmd-url"><a href="https://git-cliff.org/">https://git-cliff.org/</a></p>

- Requires --no-exec. - Allowed standalone flags: --bumped-version, --current, --help, --latest, --no-exec, --offline, --topo-order, --unreleased, --use-branch-tags, --verbose, --version, -V, -h, -l, -u, -v
- Allowed valued flags: --body, --bump, --config, --count-tags, --exclude-path, --from-context, --ignore-tags, --include-path, --repository, --skip-commit, --skip-tags, --sort, --strip, --tag, --tag-pattern, --with-commit, --with-tag-message, --workdir, -b, -c, -r, -s, -t, -w
- Bare invocation allowed

### `git-lfs`
<p class="cmd-url"><a href="https://git-lfs.com/">https://git-lfs.com/</a></p>

- **env**: Flags: --help, -h
- **locks**: Flags: --help, --json, --local, --verify, -h, -l, -v. Valued: --id, --limit, --path
- **ls-files**: Flags: --all, --deleted, --help, --long, --name-only, --size, -a, -d, -h, -l, -n, -s. Valued: --include, --exclude, -I, -X
- **status**: Flags: --help, --json, --porcelain, -h
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

### `jj`
<p class="cmd-url"><a href="https://jj-vcs.github.io/jj/latest/cli-reference/">https://jj-vcs.github.io/jj/latest/cli-reference/</a></p>

- **abandon**: Positional args accepted
- **absorb**: Positional args accepted
- **backout**: Positional args accepted
- **bookmark create**: Positional args accepted
- **bookmark delete**: Positional args accepted
- **bookmark forget**: Positional args accepted
- **bookmark list**: Positional args accepted
- **bookmark move**: Positional args accepted
- **bookmark rename**: Positional args accepted
- **bookmark set**: Positional args accepted
- **bookmark track**: Positional args accepted
- **bookmark untrack**: Positional args accepted
- **cat**: Positional args accepted
- **commit**: Positional args accepted
- **config get**: Positional args accepted
- **config list**: Positional args accepted
- **describe**: Positional args accepted
- **diff**: Positional args accepted
- **duplicate**: Positional args accepted
- **edit**: Positional args accepted
- **file list**: Positional args accepted
- **file show**: Positional args accepted
- **fix**: Positional args accepted
- **git fetch**: Positional args accepted
- **git import**: Positional args accepted
- **git init**: Positional args accepted
- **git remote list**: Positional args accepted
- **help**: Positional args accepted
- **log**: Positional args accepted
- **new**: Positional args accepted
- **op log**: Positional args accepted
- **parallelize**: Positional args accepted
- **rebase**: Positional args accepted
- **resolve**: Positional args accepted
- **restore**: Positional args accepted
- **root**: Positional args accepted
- **show**: Positional args accepted
- **simplify-parents**: Positional args accepted
- **split**: Positional args accepted
- **squash**: Positional args accepted
- **st**: Positional args accepted
- **status**: Positional args accepted
- **tag list**: Positional args accepted
- **undo**: Positional args accepted
- **unsquash**: Positional args accepted
- **version**: Positional args accepted
- **workspace add**: Positional args accepted
- **workspace forget**: Positional args accepted
- **workspace list**: Positional args accepted
- **workspace rename**: Positional args accepted
- **workspace root**: Positional args accepted
- **workspace update-stale**: Positional args accepted
- Allowed standalone flags: --help, --version, -h

### `tig`
<p class="cmd-url"><a href="https://jonas.github.io/tig/">https://jonas.github.io/tig/</a></p>

- Allowed standalone flags: --all, --date-order, --help, --reverse, --version, -C, -h, -v
- Allowed valued flags: -n
- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

