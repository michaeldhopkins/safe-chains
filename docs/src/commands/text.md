# Text Processing

### `awk / gawk / mawk / nawk`
<p class="cmd-url"><a href="https://www.gnu.org/software/gawk/manual/gawk.html">https://www.gnu.org/software/gawk/manual/gawk.html</a></p>

- Program validated: system, getline, |, > constructs checked
- Allowed standalone flags: --characters-as-bytes, --copyright, --gen-pot, --lint, --no-optimize, --optimize, --posix, --re-interval, --sandbox, --traditional, --use-lc-numeric, --version, -C, -N, -O, -P, -S, -V, -b, -c, -g, -r, -s, -t
- Allowed valued flags: --assign, --field-separator, -F, -v

### `cat`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#cat-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#cat-invocation</a></p>

- Allowed standalone flags: -A, -E, -T, -V, -b, -e, -h, -l, -n, -s, -t, -u, -v, --help, --number, --number-nonblank, --show-all, --show-ends, --show-nonprinting, --show-tabs, --squeeze-blank, --version
- Bare invocation allowed

### `col`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/col.1.html">https://man7.org/linux/man-pages/man1/col.1.html</a></p>

- Allowed standalone flags: -V, -b, -f, -h, -p, -x, --help, --version
- Allowed valued flags: -l
- Bare invocation allowed

### `column`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/column.1.html">https://man7.org/linux/man-pages/man1/column.1.html</a></p>

- Allowed standalone flags: -J, -L, -R, -V, -e, -h, -n, -t, -x, --fillrows, --help, --json, --keep-empty-lines, --table, --table-noextreme, --table-noheadings, --table-right-all, --version
- Allowed valued flags: -E, -H, -O, -W, -c, -d, -o, -r, -s, --output-separator, --separator, --table-columns, --table-empty-lines, --table-hide, --table-name, --table-order, --table-right, --table-truncate, --table-wrap
- Bare invocation allowed

### `comm`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#comm-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#comm-invocation</a></p>

- Allowed standalone flags: -1, -2, -3, -V, -h, -i, -z, --check-order, --help, --nocheck-order, --total, --version, --zero-terminated
- Allowed valued flags: --output-delimiter

### `cut`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#cut-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#cut-invocation</a></p>

- Allowed standalone flags: -V, -h, -n, -s, -w, -z, --complement, --help, --only-delimited, --version, --zero-terminated
- Allowed valued flags: -b, -c, -d, -f, --bytes, --characters, --delimiter, --fields, --output-delimiter

### `expand`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#expand-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#expand-invocation</a></p>

- Allowed standalone flags: -V, -h, -i, --help, --initial, --version
- Allowed valued flags: -t, --tabs
- Bare invocation allowed

### `fmt`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#fmt-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#fmt-invocation</a></p>

- Allowed standalone flags: -V, -c, -h, -m, -n, -s, -u, --crown-margin, --help, --split-only, --tagged-paragraph, --uniform-spacing, --version
- Allowed valued flags: -d, -g, -l, -p, -t, -w, --goal, --prefix, --width
- Bare invocation allowed

### `fold`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#fold-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#fold-invocation</a></p>

- Allowed standalone flags: -V, -b, -h, -s, --bytes, --help, --spaces, --version
- Allowed valued flags: -w, --width
- Bare invocation allowed

### `glow`
<p class="cmd-url"><a href="https://github.com/charmbracelet/glow">https://github.com/charmbracelet/glow</a></p>

- Allowed standalone flags: --help, --version, -h, -v, --all, -a, --local, -l, --pager, -p
- Allowed valued flags: --style, -s, --width, -w, --config
- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `head`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#head-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#head-invocation</a></p>

- Allowed standalone flags: -V, -h, -q, -v, -z, --help, --quiet, --silent, --verbose, --version, --zero-terminated
- Allowed valued flags: -c, -n, --bytes, --lines
- Bare invocation allowed
- Numeric shorthand accepted (e.g. -20 for -n 20)

### `iconv`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/iconv.1.html">https://man7.org/linux/man-pages/man1/iconv.1.html</a></p>

- Allowed standalone flags: -V, -c, -h, -l, -s, --help, --list, --silent, --version
- Allowed valued flags: -f, -t, --from-code, --to-code

### `less`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/less.1.html">https://man7.org/linux/man-pages/man1/less.1.html</a></p>

- Allowed standalone flags: -E, -F, -G, -I, -J, -K, -L, -M, -N, -Q, -R, -S, -V, -W, -X, -a, -c, -e, -f, -g, -i, -m, -n, -q, -r, -s, -w, --QUIT-AT-EOF, --RAW-CONTROL-CHARS, --chop-long-lines, --help, --ignore-case, --no-init, --quiet, --quit-at-eof, --quit-if-one-screen, --raw-control-chars, --silent, --squeeze-blank-lines, --version
- Allowed valued flags: -P, -b, -h, -j, -p, -t, -x, -y, -z, --LINE-NUMBERS, --LONG-PROMPT, --pattern, --prompt, --shift, --tabs, --tag, --window
- Bare invocation allowed

### `more`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/more.1.html">https://man7.org/linux/man-pages/man1/more.1.html</a></p>

- Allowed standalone flags: -V, -c, -d, -f, -h, -l, -p, -s, -u, --help, --version
- Allowed valued flags: -n, --lines
- Bare invocation allowed

### `nl`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#nl-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#nl-invocation</a></p>

- Allowed standalone flags: -V, -p, --help, --no-renumber, --version
- Allowed valued flags: -b, -d, -f, -h, -i, -l, -n, -s, -v, -w, --body-numbering, --footer-numbering, --header-numbering, --join-blank-lines, --line-increment, --number-format, --number-separator, --number-width, --section-delimiter, --starting-line-number
- Bare invocation allowed

### `nroff`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/nroff.1.html">https://man7.org/linux/man-pages/man1/nroff.1.html</a></p>

- Allowed standalone flags: -S, -V, -c, -h, -i, -k, -p, -q, -t, --help, --version
- Allowed valued flags: -M, -P, -T, -d, -m, -n, -o, -r, -w

### `paste`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#paste-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#paste-invocation</a></p>

- Allowed standalone flags: -V, -h, -s, -z, --help, --serial, --version, --zero-terminated
- Allowed valued flags: -d, --delimiters
- Bare invocation allowed

### `perl`
<p class="cmd-url"><a href="https://perldoc.perl.org/perl">https://perldoc.perl.org/perl</a></p>

- Allowed: -e/-E inline one-liners with safe built-in functions, --version, --help, -v, -V. Requires -e/-E flag. Code is validated against a safe identifier allowlist.

### `rev`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/rev.1.html">https://man7.org/linux/man-pages/man1/rev.1.html</a></p>

- Allowed standalone flags: -V, -h, --help, --version
- Bare invocation allowed

### `sed`
<p class="cmd-url"><a href="https://www.gnu.org/software/sed/manual/sed.html">https://www.gnu.org/software/sed/manual/sed.html</a></p>

- Allowed standalone flags: --debug, --help, --posix, --quiet, --sandbox, --silent, --unbuffered, --version, -E, -V, -h, -n, -r, -u, -z
- Allowed valued flags: --expression, --file, --line-length, -e, -f, -l
- Inline expressions validated for safety

### `tac`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#tac-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#tac-invocation</a></p>

- Allowed standalone flags: -V, -b, -h, -r, --before, --help, --regex, --version
- Allowed valued flags: -s, --separator
- Bare invocation allowed

### `tail`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#tail-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#tail-invocation</a></p>

- Allowed standalone flags: -F, -V, -f, -h, -q, -r, -v, -z, --follow, --help, --quiet, --retry, --silent, --verbose, --version, --zero-terminated
- Allowed valued flags: -b, -c, -n, --bytes, --lines, --max-unchanged-stats, --pid, --sleep-interval
- Bare invocation allowed
- Numeric shorthand accepted (e.g. -20 for -n 20)

### `tr`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#tr-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#tr-invocation</a></p>

- Allowed standalone flags: -C, -V, -c, -d, -h, -s, --complement, --delete, --help, --squeeze-repeats, --truncate-set1, --version

### `unexpand`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#unexpand-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#unexpand-invocation</a></p>

- Allowed standalone flags: -V, -a, -h, --all, --first-only, --help, --version
- Allowed valued flags: -t, --tabs
- Bare invocation allowed

### `uniq`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#uniq-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#uniq-invocation</a></p>

- Allowed standalone flags: -D, -V, -c, -d, -h, -i, -u, -z, --count, --help, --ignore-case, --repeated, --unique, --version, --zero-terminated
- Allowed valued flags: -f, -s, -w, --all-repeated, --check-chars, --group, --skip-chars, --skip-fields
- Bare invocation allowed

### `wc`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#wc-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#wc-invocation</a></p>

- Allowed standalone flags: -L, -V, -c, -h, -l, -m, -w, --bytes, --chars, --help, --lines, --max-line-length, --version, --words, --zero-terminated
- Allowed valued flags: --files0-from
- Bare invocation allowed

### `zcat`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/zcat.1.html">https://man7.org/linux/man-pages/man1/zcat.1.html</a></p>

Aliases: `gzcat`

- Allowed standalone flags: -V, -f, -h, -q, -v, --force, --help, --quiet, --verbose, --version
- Bare invocation allowed

