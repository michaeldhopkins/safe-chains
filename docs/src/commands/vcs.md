# Version Control

### `fossil`
<p class="cmd-url"><a href="https://fossil-scm.org/home/help">https://fossil-scm.org/home/help</a></p>

- **branch current**: Flags: --help, -h
- **branch info**: Flags: --help, --repository, -R, -h. Valued: --repository, -R
- **branch list**: Flags: --all, --closed, --help, --repository, --verbose, -a, -c, -h, -r, -v. Valued: --repository, -R
- **cat**: Flags: --help, --repository, -R, -h, -r. Valued: --repository, -R, -r. Positional args accepted
- **diff**: Flags: --brief, --checkin, --context, --diff-binary, --exclude, --from, --help, --ignore-all-space, --ignore-blank-lines, --ignore-case, --ignore-eol-ws, --ignore-space-at-eol, --ignore-trailing-cr, --include, --internal, --invert, --no-dir-prefix, --numstat, --quiet, --repository, --show-summary, --side-by-side, --strip-trailing-cr, --strip-trailing-lf, --strip-trailing-spaces, --strip-trailing-whitespace, --tk, --to, --unified, --verbose, --versions, --web, -N, -R, -W, -Z, -b, -c, -h, -i, -q, -r, -u, -v, -w, -y, -z. Valued: --against, --diff-binary, --exec-abs-paths, --exec-rel-paths, --from, --internal, --repository, --strip-trailing-spaces, --strip-trailing-whitespace, --to, --unified, -N, -R, -W, -c, -r, -y. Positional args accepted
- **extras**: Flags: --abs-paths, --case-sensitive, --dotfiles, --header, --help, --ignore, --repository, --rel-paths, -R, -h. Valued: --ignore, --repository, -R
- **finfo**: Flags: --help, --limit, --no-graph, --offset, --print, --repository, --show-id, --showid, --status, --type, --width, -R, -h, -l, -p, -w. Valued: --branch, --limit, --repository, --type, --width, -R, -l, -w. Positional args accepted
- **help**: Positional args accepted
- **info**: Flags: --comment-format, --help, --repository, --verbose, -R, -h, -v. Valued: --repository, -R. Positional args accepted
- **ls**: Flags: --age, --help, --repository, --type, --verbose, -R, -h, -l, -r, -v. Valued: --repository, --type, -R, -r. Positional args accepted
- **search**: Flags: --help, -h, --all, --checkin, --documentation, --forum, --ticket, --unindexed, --wiki. Valued: --limit, -n. Positional args accepted
- **status**: Flags: --all, --changed, --differ, --dotfiles, --header, --help, --ifchanged, --limit, --repository, --verbose, -R, -a, -c, -d, -h, -v. Valued: --repository, -R
- **tag find**: Flags: --help, -h. Valued: --repository, --type, -R
- **tag list**: Flags: --all, --inverse, --help, --raw, --repository, -R, -a, -h, -i. Valued: --limit, --prefix, --repository, -R
- **timeline**: Flags: --brief, --full, --help, --limit, --no-graph, --repository, --showfiles, --verbose, -W, -R, -b, -h, -l, -n, -v, -w. Valued: --after, --before, --children, --limit, --repository, --type, --width, -R, -W, -c, -l, -n, -t, -w. Positional args accepted
- **version**: Flags: --help, --verbose, -h, -v
- **whatis**: Flags: --help, --repository, --type, --verbose, -R, -h, -v. Valued: --repository, --type, -R. Positional args accepted
- Allowed standalone flags: --help, --version, -h, -v

### `git`
<p class="cmd-url"><a href="https://git-scm.com/docs">https://git-scm.com/docs</a></p>

- **blame**: Flags: --color-by-age, --color-lines, --help, --incremental, --line-porcelain, --minimal, --porcelain, --progress, --root, --show-email, --show-name, --show-number, --show-stats, -b, -c, -e, -f, -h, -k, -l, -n, -p, -s, -t, -w. Valued: --abbrev, --contents, --date, --ignore-rev, --ignore-revs-file, -C, -L, -M, -S
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
- **pull**: Flags: --all, --allow-unrelated-histories, --append, --autostash, --commit, --compact-summary, --dry-run, --edit, --ff, --ff-only, --force, --help, --ipv4, --ipv6, --keep, --no-autostash, --no-commit, --no-edit, --no-ff, --no-gpg-sign, --no-log, --no-rebase, --no-recurse-submodules, --no-show-forced-updates, --no-signoff, --no-squash, --no-stat, --no-tags, --no-verify, --no-verify-signatures, --progress, --prune, --quiet, --set-upstream, --show-forced-updates, --signoff, --squash, --stat, --tags, --verbose, --verify, --verify-signatures, -4, -6, -a, -e, -f, -h, -k, -n, -p, -q, -t, -v. Valued: --cleanup, --depth, --deepen, --filter, --gpg-sign, --jobs, --log, --rebase, --recurse-submodules, --refmap, --server-option, --shallow-exclude, --shallow-since, --strategy, --strategy-option, --upload-pack, -S, -X, -j, -o, -r, -s
- **push**
- **reflog**: Flags: --abbrev-commit, --all, --ancestry-path, --author-date-order, --bisect, --boundary, --branches, --cherry, --cherry-mark, --cherry-pick, --children, --clear-decorations, --compact-summary, --cumulative, --date-order, --dense, --do-walk, --early-output, --first-parent, --follow, --full-diff, --full-history, --graph, --help, --ignore-missing, --invert-grep, --left-only, --left-right, --log-size, --mailmap, --merges, --minimal, --name-only, --name-status, --no-abbrev-commit, --no-color, --no-decorate, --no-expand-tabs, --no-ext-diff, --no-merges, --no-notes, --no-patch, --no-prefix, --no-renames, --no-walk, --numstat, --oneline, --parents, --patch, --patch-with-raw, --patch-with-stat, --patience, --pickaxe-all, --pickaxe-regex, --raw, --reflog, --regexp-ignore-case, --relative-date, --remotes, --reverse, --shortstat, --show-linear-break, --show-notes, --show-pulls, --show-signature, --simplify-by-decoration, --simplify-merges, --source, --sparse, --stat, --stdin, --summary, --tags, --text, --topo-order, --use-mailmap, -0, -1, -2, -3, -4, -5, -6, -7, -8, -9, -h, -i, -p, -q, -s, -u. Valued: --abbrev, --after, --author, --before, --color, --committer, --date, --decorate, --decorate-refs, --decorate-refs-exclude, --diff-algorithm, --diff-filter, --diff-merges, --encoding, --exclude, --format, --glob, --grep, --max-count, --max-parents, --min-parents, --pretty, --since, --skip, --until, -G, -L, -S, -n
- **remote get-url**: Flags: --push, --all, -h, --help
- **remote show**: Flags: -n, -h, --help
- **remote**: Flags: -v, --verbose, -h, --help
- **rev-parse**: Flags: --absolute-git-dir, --all, --branches, --git-common-dir, --git-dir, --git-path, --help, --is-bare-repository, --is-inside-git-dir, --is-inside-work-tree, --is-shallow-repository, --local-env-vars, --quiet, --remotes, --shared-index-path, --show-cdup, --show-prefix, --show-superproject-working-tree, --show-toplevel, --symbolic, --symbolic-full-name, --tags, --verify, -h, -q. Valued: --abbrev-ref, --after, --before, --default, --exclude, --glob, --prefix, --resolve-git-dir, --short, --since, --until. Positional args accepted
- **shortlog**: Flags: --committer, --email, --help, --numbered, --summary, -c, -e, -h, -n, -s. Valued: --format, --group
- **show**: Flags: --abbrev-commit, --cc, --color-words, --combined-all-paths, --compact-summary, --cumulative, --expand-tabs, --find-copies, --find-renames, --full-index, --help, --histogram, --ignore-all-space, --ignore-blank-lines, --ignore-space-at-eol, --ignore-space-change, --mailmap, --minimal, --name-only, --name-status, --no-color, --no-ext-diff, --no-notes, --no-patch, --no-prefix, --no-renames, --no-textconv, --numstat, --oneline, --patch, --patch-with-raw, --patch-with-stat, --patience, --pickaxe-all, --pickaxe-regex, --raw, --shortstat, --show-notes, --show-signature, --source, --stat, --summary, --text, --textconv, --use-mailmap, -h, -p, -q, -s, -u, -w. Valued: --abbrev, --color, --color-moved, --decorate, --decorate-refs, --decorate-refs-exclude, --diff-algorithm, --diff-filter, --diff-merges, --encoding, --format, --notes, --pretty, --stat-count, --stat-graph-width, --stat-name-width, --submodule, --word-diff, --word-diff-regex, -G, -O, -S
- **stash --help**: Positional args accepted
- **stash -h**: Positional args accepted
- **stash list**: Positional args accepted
- **stash show**: Flags: --help, --patch, --stat, -h, -p, -u. Positional args accepted
- **status**: Flags: --ahead-behind, --branch, --help, --ignore-submodules, --long, --no-ahead-behind, --no-renames, --null, --renames, --short, --show-stash, --verbose, -b, -h, -s, -v, -z. Valued: --column, --find-renames, --ignored, --porcelain, --untracked-files, -M, -u
- **symbolic-ref**: Flags: --help, --no-recurse, --quiet, --recurse, --short, -h, -q
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

### `hg`
<p class="cmd-url"><a href="https://www.mercurial-scm.org/doc/hg.1.html">https://www.mercurial-scm.org/doc/hg.1.html</a></p>

- **annotate**: Flags: --changeset, --date, --file, --follow, --help, --ignore-all-space, --ignore-blank-lines, --ignore-space-change, --ignore-space-at-eol, --line-number, --no-binary, --no-follow, --number, --quiet, --text, --user, --verbose, -B, -Z, -a, -b, -c, -d, -f, -h, -l, -n, -q, -u, -v, -w. Valued: --exclude, --include, --rev, --skip, -I, -X, -r. Positional args accepted
- **blame**: Flags: --changeset, --date, --file, --follow, --help, --ignore-all-space, --ignore-blank-lines, --ignore-space-change, --ignore-space-at-eol, --line-number, --no-binary, --no-follow, --number, --quiet, --text, --user, --verbose, -B, -Z, -a, -b, -c, -d, -f, -h, -l, -n, -q, -u, -v, -w. Valued: --exclude, --include, --rev, --skip, -I, -X, -r. Positional args accepted
- **bm**: Flags: --active, --all, --delete, --force, --help, --inactive, --rename, -B, -d, -f, -h, -i, -l, -m. Valued: --rev, -r. Positional args accepted
- **bookmark**: Flags: --active, --all, --delete, --force, --help, --inactive, --rename, -B, -d, -f, -h, -i, -l, -m. Valued: --rev, -r. Positional args accepted
- **bookmarks**: Flags: --active, --all, --delete, --force, --help, --inactive, --rename, -B, -d, -f, -h, -i, -l, -m. Valued: --rev, -r. Positional args accepted
- **branch**: Flags: --clean, --force, --help, -C, -f, -h. Positional args accepted
- **branches**: Flags: --active, --closed, --help, -a, -c, -h. Valued: --rev, --style, --template, -T, -r
- **cat**: Flags: --decode, --help, -h. Valued: --exclude, --include, --output, --rev, --template, -I, -T, -X, -o, -r. Positional args accepted
- **config**: Flags: --debug, --edit, --global, --help, --local, --non-shared, --quiet, --user, -e, -h, -q. Valued: --source, -T, --template. Positional args accepted
- **diff**: Flags: --change, --git, --ignore-all-space, --ignore-blank-lines, --ignore-space-change, --ignore-space-at-eol, --ignore-changes, --mar, --no-binary, --no-dates, --noprefix, --nodates, --patch, --reverse, --show-function, --stat, --text, --unified, --word-diff, -B, -Z, -a, -b, -c, -g, -h, -p, -r, -w. Valued: --change, --exclude, --include, --rev, --root, --unified, -I, -U, -X, -c, -r. Positional args accepted
- **files**: Flags: --help, --print0, --quiet, --verbose, -0, -h, -q, -v. Valued: --exclude, --include, --rev, -I, -T, -X, -r. Positional args accepted
- **grep**: Flags: --all, --all-files, --diff, --files-with-matches, --ignore-case, --help, --line-number, --print0, --rev, -0, -a, -d, -h, -i, -l, -n, -r. Valued: --exclude, --include, --rev, --user, -I, -X, -r, -u. Positional args accepted
- **heads**: Flags: --active, --closed, --help, -a, -c, -h. Valued: --rev, --style, --template, -T, -r. Positional args accepted
- **help**: Positional args accepted
- **id**: Flags: --branch, --bookmarks, --debug, --help, --id, --num, --tags, -B, -b, -h, -i, -n, -r, -t. Valued: --rev, -r. Positional args accepted
- **identify**: Flags: --branch, --bookmarks, --debug, --help, --id, --num, --tags, -B, -b, -h, -i, -n, -r, -t. Valued: --rev, -r. Positional args accepted
- **incoming**: Flags: --bookmarks, --branch, --bundle, --force, --graph, --help, --insecure, --newest-first, --patch, -B, -G, -S, -b, -f, -h, -l, -n, -p, -r. Valued: --branch, --bundle, --rev, --ssh, --remotecmd, -r. Positional args accepted
- **log**: Flags: --branch, --copies, --debug, --follow, --follow-first, --git, --graph, --hidden, --help, --no-merges, --patch, --quiet, --removed, --reverse, --stat, --style, --template, --user, --verbose, -G, -M, -T, -b, -c, -d, -f, -g, -h, -k, -l, -p, -q, -r, -u, -v, -y. Valued: --branch, --date, --exclude, --include, --keyword, --limit, --prune, --rev, --style, --template, --user, -T, -X, -d, -f, -k, -l, -r, -u. Positional args accepted
- **manifest**: Flags: --all, --debug, --help, -h. Valued: --rev, -r. Positional args accepted
- **outgoing**: Flags: --bookmarks, --branch, --force, --graph, --help, --insecure, --newest-first, --patch, --ssh, --remotecmd, -B, -G, -S, -b, -f, -h, -l, -n, -p, -r. Valued: --branch, --rev, --ssh, --remotecmd, -r. Positional args accepted
- **parents**: Flags: --help, -h. Valued: --rev, --style, --template, -T, -r. Positional args accepted
- **paths**: Flags: --help, --quiet, --verbose, -q, -v. Positional args accepted
- **showconfig**: Flags: --debug, --edit, --global, --help, --local, --non-shared, --quiet, --user, -e, -h, -q. Valued: --source, -T, --template. Positional args accepted
- **st**: Flags: --all, --added, --clean, --copies, --deleted, --ignored, --modified, --no-status, --print0, --quiet, --removed, --rev, --terse, --unknown, -0, -A, -C, -T, -a, -c, -d, -h, -i, -m, -n, -q, -r, -u. Valued: --change, --exclude, --include, --rev, --terse, -I, -X, -r, -t. Positional args accepted
- **status**: Flags: --all, --added, --clean, --copies, --deleted, --ignored, --modified, --no-status, --print0, --quiet, --removed, --rev, --terse, --unknown, -0, -A, -C, -T, -a, -c, -d, -h, -i, -m, -n, -q, -r, -u. Valued: --change, --exclude, --include, --rev, --terse, -I, -X, -r, -t. Positional args accepted
- **sum**: Flags: --help, --remote, -h
- **summary**: Flags: --help, --remote, -h
- **tags**: Flags: --debug, --help, --quiet, --verbose, -h, -q, -v
- **tip**: Flags: --help, --patch, --style, --template, -T, -h, -p
- **verify**: Flags: --help, -h
- **version**: Flags: --help, -h. Valued: -T, --template
- Allowed standalone flags: --help, --version, -h, -v

**Examples:**

- `hg log`
- `hg status`
- `hg st`
- `hg summary`
- `hg sum`
- `hg id`
- `hg identify`
- `hg blame foo.py`
- `hg annotate foo.py`
- `hg bookmarks`
- `hg bm`
- `hg config`
- `hg showconfig`

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
- **config path**: Positional args accepted
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

### `lore`
<p class="cmd-url"><a href="https://epicgames.github.io/lore/">https://epicgames.github.io/lore/</a></p>

- **completions**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

**Examples:**

- `lore --version`
- `lore --help`
- `lore completions bash`
- `lore completions zsh`
- `lore completions fish`

### `pijul`
<p class="cmd-url"><a href="https://pijul.org/manual/">https://pijul.org/manual/</a></p>

- **change**: Flags: --full-hashes, --help, --inverse, --json, -h. Valued: --channel, --repository. Positional args accepted
- **channel list**: Flags: --help, -h. Valued: --repository
- **completion**: Flags: --help, -h. Positional args accepted
- **credit**: Flags: --help, --unrecorded, -h. Valued: --channel, --repository. Positional args accepted
- **diff**: Flags: --full-hashes, --help, --inodes, --json, --no-color, --short, --untracked, -h, -s. Valued: --channel, --repository. Positional args accepted
- **help**: Positional args accepted
- **key list**: Flags: --help, -h
- **log**: Flags: --description, --filter, --full-hashes, --hash-only, --help, --limit, --no-cache, --state, -h. Valued: --channel, --description, --filter, --limit, --repository
- **ls**: Flags: --help, -h. Valued: --repo-path, --repository. Positional args accepted
- **status**: Flags: --full-hashes, --help, --no-mtime, --short, -h. Valued: --channel, --repository. Positional args accepted
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h, -V

### `scalar`
<p class="cmd-url"><a href="https://git-scm.com/docs/scalar">https://git-scm.com/docs/scalar</a></p>

- **diagnose**: Flags: --help, -h
- **list**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

### `svn`
<p class="cmd-url"><a href="https://subversion.apache.org/docs/release-notes/1.14.html">https://subversion.apache.org/docs/release-notes/1.14.html</a></p>

- **ann**: Flags: --force, --help, --use-merge-history, --xml, -g, -h, -r, -v, -x. Valued: --config-dir, --diff-cmd, --extensions, --password, --revision, --username, -r, -x. Positional args accepted
- **annotate**: Flags: --force, --help, --use-merge-history, --xml, -g, -h, -r, -v, -x. Valued: --config-dir, --diff-cmd, --extensions, --password, --revision, --username, -r, -x. Positional args accepted
- **blame**: Flags: --force, --help, --use-merge-history, --xml, -g, -h, -r, -v, -x. Valued: --config-dir, --diff-cmd, --extensions, --password, --revision, --username, -r, -x. Positional args accepted
- **cat**: Flags: --help, --ignore-keywords, -h, -r. Valued: --config-dir, --password, --revision, --username, -r. Positional args accepted
- **diff**: Flags: --diff-cmd, --force, --git, --help, --no-diff-deleted, --no-diff-added, --no-diff-statistics, --notice-ancestry, --patch-compatible, --summarize, --xml, -c, -h, -r, -x. Valued: --changelist, --config-dir, --config-option, --depth, --diff-cmd, --extensions, --internal-diff, --new, --old, --password, --revision, --username, -c, -r, -x. Positional args accepted
- **help**: Positional args accepted
- **info**: Flags: --help, --include-externals, --incremental, --recursive, --targets, --xml, -R, -h, -r. Valued: --changelist, --config-dir, --depth, --password, --revision, --targets, --username, -r. Positional args accepted
- **list**: Flags: --depth, --help, --incremental, --recursive, --verbose, --xml, -R, -h, -r, -v. Valued: --config-dir, --depth, --password, --revision, --username, -r. Positional args accepted
- **log**: Flags: --all-revprops, --diff, --help, --incremental, --quiet, --revprop, --search, --stop-on-copy, --use-merge-history, --verbose, --with-all-revprops, --with-no-revprops, --xml, -c, -g, -h, -l, -q, -r, -v. Valued: --changelist, --config-dir, --config-option, --depth, --diff-cmd, --limit, --password, --revision, --search-and, --targets, --username, --with-revprop, --xml-file, -c, -g, -l, -r. Positional args accepted
- **ls**: Flags: --depth, --help, --incremental, --recursive, --verbose, --xml, -R, -h, -r, -v. Valued: --config-dir, --depth, --password, --revision, --username, -r. Positional args accepted
- **mergeinfo**: Flags: --from-source, --help, --show-revs, -h. Valued: --config-dir, --password, --revision, --username, -R, -r. Positional args accepted
- **pg**: Flags: --depth, --help, --no-newline, --recursive, --revprop, --show-inherited-props, --strict, --verbose, --xml, -R, -h, -r, -v. Valued: --changelist, --config-dir, --depth, --password, --revision, --username, -r. Positional args accepted
- **pget**: Flags: --depth, --help, --no-newline, --recursive, --revprop, --show-inherited-props, --strict, --verbose, --xml, -R, -h, -r, -v. Valued: --changelist, --config-dir, --depth, --password, --revision, --username, -r. Positional args accepted
- **pl**: Flags: --depth, --help, --incremental, --quiet, --recursive, --revprop, --show-inherited-props, --verbose, --xml, -R, -h, -q, -r, -v. Valued: --changelist, --config-dir, --depth, --password, --revision, --username, -r. Positional args accepted
- **plist**: Flags: --depth, --help, --incremental, --quiet, --recursive, --revprop, --show-inherited-props, --verbose, --xml, -R, -h, -q, -r, -v. Valued: --changelist, --config-dir, --depth, --password, --revision, --username, -r. Positional args accepted
- **praise**: Flags: --force, --help, --use-merge-history, --xml, -g, -h, -r, -v, -x. Valued: --config-dir, --diff-cmd, --extensions, --password, --revision, --username, -r, -x. Positional args accepted
- **propget**: Flags: --depth, --help, --no-newline, --recursive, --revprop, --show-inherited-props, --strict, --verbose, --xml, -R, -h, -r, -v. Valued: --changelist, --config-dir, --depth, --password, --revision, --username, -r. Positional args accepted
- **proplist**: Flags: --depth, --help, --incremental, --quiet, --recursive, --revprop, --show-inherited-props, --verbose, --xml, -R, -h, -q, -r, -v. Valued: --changelist, --config-dir, --depth, --password, --revision, --username, -r. Positional args accepted
- **st**: Flags: --changelist, --config-dir, --depth, --help, --ignore-externals, --incremental, --no-ignore, --quiet, --show-updates, --verbose, --xml, -N, -h, -q, -u, -v. Valued: --changelist, --config-dir, --config-option, --depth. Positional args accepted
- **stat**: Flags: --changelist, --config-dir, --depth, --help, --ignore-externals, --incremental, --no-ignore, --quiet, --show-updates, --verbose, --xml, -N, -h, -q, -u, -v. Valued: --changelist, --config-dir, --config-option, --depth. Positional args accepted
- **status**: Flags: --changelist, --config-dir, --depth, --help, --ignore-externals, --incremental, --no-ignore, --quiet, --show-updates, --verbose, --xml, -N, -h, -q, -u, -v. Valued: --changelist, --config-dir, --config-option, --depth. Positional args accepted
- Allowed standalone flags: --help, --version, -h, -v

**Examples:**

- `svn log`
- `svn status`
- `svn stat`
- `svn st`
- `svn list`
- `svn ls`
- `svn info`
- `svn diff`
- `svn blame foo.c`
- `svn praise foo.c`
- `svn annotate foo.c`
- `svn ann foo.c`
- `svn proplist`
- `svn plist`
- `svn pl`
- `svn propget svn:keywords foo.c`
- `svn pget svn:keywords foo.c`

### `tig`
<p class="cmd-url"><a href="https://jonas.github.io/tig/">https://jonas.github.io/tig/</a></p>

- Allowed standalone flags: --all, --date-order, --help, --reverse, --version, -C, -h, -v
- Allowed valued flags: -n
- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

