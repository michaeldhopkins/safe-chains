# Text Processing

### `a2p`
<p class="cmd-url"><a href="https://perldoc.perl.org/a2p">https://perldoc.perl.org/a2p</a></p>

- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `aspell`
<p class="cmd-url"><a href="https://aspell.net/">https://aspell.net/</a></p>

- **config**: Positional args accepted
- **dicts**: Flags: --help
- **dump**: Positional args accepted
- **filters**: Flags: --help
- **list**: Flags: --help, --reverse, -r. Valued: --lang, --mode, --encoding, -l, -m
- **modes**: Flags: --help
- **pipe**: Flags: --help. Valued: --lang, --mode, --encoding, -l, -m
- **soundslike**: Flags: --help. Valued: --lang, -l
- Allowed standalone flags: --help, --version, -?

### `awk / gawk / mawk / nawk`
<p class="cmd-url"><a href="https://www.gnu.org/software/gawk/manual/gawk.html">https://www.gnu.org/software/gawk/manual/gawk.html</a></p>

- Program validated: system, getline, |, > constructs checked
- Allowed standalone flags: --characters-as-bytes, --copyright, --gen-pot, --lint, --no-optimize, --optimize, --posix, --re-interval, --sandbox, --traditional, --use-lc-numeric, --version, -C, -N, -O, -P, -S, -V, -b, -c, -g, -r, -s, -t
- Allowed valued flags: --assign, --field-separator, -F, -v

### `base32`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#base32-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#base32-invocation</a></p>

- Allowed standalone flags: --decode, --help, --ignore-garbage, --version, -d, -h, -i
- Allowed valued flags: --wrap, -w
- Bare invocation allowed

### `basenc`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#basenc-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#basenc-invocation</a></p>

- Allowed standalone flags: --base16, --base2lsbf, --base2msbf, --base32, --base32hex, --base64, --base64url, --decode, --help, --ignore-garbage, --version, --z85, -d, -h, -i
- Allowed valued flags: --wrap, -w
- Bare invocation allowed

### `cat`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#cat-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#cat-invocation</a></p>

Aliases: `gcat`

- Reads its file operands to stdout within your workspace.
- Allowed standalone flags: --help, --number, --number-nonblank, --show-all, --show-ends, --show-nonprinting, --show-tabs, --squeeze-blank, --version, -A, -E, -T, -b, -e, -n, -s, -t, -u, -v
- Bare invocation reads stdin

**Examples:**

- `cat README.md`
- `cat src/main.rs`
- `cat ./notes.txt`

### `col`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/col.1.html">https://man7.org/linux/man-pages/man1/col.1.html</a></p>

- Allowed standalone flags: -V, -b, -f, -h, -p, -x, --help, --version
- Allowed valued flags: -l
- Bare invocation allowed

### `colrm`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/colrm.1.html">https://man7.org/linux/man-pages/man1/colrm.1.html</a></p>

- Allowed standalone flags: --help, --version, -V, -h
- Bare invocation allowed

### `column`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/column.1.html">https://man7.org/linux/man-pages/man1/column.1.html</a></p>

- Allowed standalone flags: -J, -L, -R, -V, -e, -h, -n, -t, -x, --fillrows, --help, --json, --keep-empty-lines, --table, --table-noextreme, --table-noheadings, --table-right-all, --version
- Allowed valued flags: -E, -H, -O, -W, -c, -d, -o, -r, -s, --output-separator, --separator, --table-columns, --table-empty-lines, --table-hide, --table-name, --table-order, --table-right, --table-truncate, --table-wrap
- Bare invocation allowed

### `comm`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#comm-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#comm-invocation</a></p>

Aliases: `gcomm`

- Allowed standalone flags: -1, -2, -3, -V, -h, -i, -z, --check-order, --help, --nocheck-order, --total, --version, --zero-terminated
- Allowed valued flags: --output-delimiter

### `cut`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#cut-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#cut-invocation</a></p>

Aliases: `gcut`

- Allowed standalone flags: -V, -h, -n, -s, -w, -z, --complement, --help, --only-delimited, --version, --zero-terminated
- Allowed valued flags: -b, -c, -d, -f, --bytes, --characters, --delimiter, --fields, --output-delimiter

### `dc`
<p class="cmd-url"><a href="https://www.gnu.org/software/bc/manual/dc-1.05/html_mono/dc.html">https://www.gnu.org/software/bc/manual/dc-1.05/html_mono/dc.html</a></p>

- Allowed standalone flags: --help, --version, -V, -h
- Allowed valued flags: --expression, --file, -e, -f
- Bare invocation allowed

### `demandoc`
<p class="cmd-url"><a href="https://mandoc.bsd.lv/man/demandoc.1.html">https://mandoc.bsd.lv/man/demandoc.1.html</a></p>

- Allowed standalone flags: -w
- Bare invocation allowed

### `dict`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/dict.1.html">https://man7.org/linux/man-pages/man1/dict.1.html</a></p>

- Allowed standalone flags: --help, --version, -a, -D, -I, -S, -V, -i, -r, -s
- Allowed valued flags: --host, --port, --database, --strategy, --match, -h, -p, -d, -m, -c, -u, -k, -C, -P
- Hyphen-prefixed positional arguments accepted

### `diff3`
<p class="cmd-url"><a href="https://www.gnu.org/software/diffutils/manual/diffutils.html#diff3-Invocation">https://www.gnu.org/software/diffutils/manual/diffutils.html#diff3-Invocation</a></p>

- Allowed standalone flags: --easy-only, --ed, --help, --initial-tab, --merge, --overlap-only, --show-all, --show-overlap, --strip-trailing-cr, --text, --version, -3, -A, -E, -T, -V, -X, -a, -e, -h, -i, -m, -x
- Allowed valued flags: --diff-program, --label, -L

### `diffstat`
<p class="cmd-url"><a href="https://invisible-island.net/diffstat/">https://invisible-island.net/diffstat/</a></p>

- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `dircolors`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#dircolors-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#dircolors-invocation</a></p>

Aliases: `gdircolors`

- Allowed standalone flags: --bourne-shell, --c-shell, --csh, --help, --print-database, --print-ls-colors, --sh, --version, -V, -b, -c, -h, -p
- Bare invocation allowed

### `expand`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#expand-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#expand-invocation</a></p>

Aliases: `gexpand`

- Allowed standalone flags: -V, -h, -i, --help, --initial, --version
- Allowed valued flags: -t, --tabs
- Bare invocation allowed

### `fmt`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#fmt-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#fmt-invocation</a></p>

Aliases: `gfmt`

- Allowed standalone flags: -V, -c, -h, -m, -n, -s, -u, --crown-margin, --help, --split-only, --tagged-paragraph, --uniform-spacing, --version
- Allowed valued flags: -d, -g, -l, -p, -t, -w, --goal, --prefix, --width
- Bare invocation allowed

### `fold`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#fold-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#fold-invocation</a></p>

Aliases: `gfold`

- Allowed standalone flags: -V, -b, -h, -s, --bytes, --help, --spaces, --version
- Allowed valued flags: -w, --width
- Bare invocation allowed

### `gencat`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/gencat.1.html">https://man7.org/linux/man-pages/man1/gencat.1.html</a></p>

- Allowed standalone flags: --help, --version, --new, -V, -h
- Allowed valued flags: --lang, -l

### `gettext`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/gettext-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/gettext-Invocation.html</a></p>

- Allowed standalone flags: --help, --version, -E, -h, -n, -V, -s
- Allowed valued flags: --domain, --env, -d, -e

### `glow`
<p class="cmd-url"><a href="https://github.com/charmbracelet/glow">https://github.com/charmbracelet/glow</a></p>

- Allowed standalone flags: --help, --version, -h, -v, --all, -a, --local, -l, --pager, -p
- Allowed valued flags: --style, -s, --width, -w, --config
- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `grep`
<p class="cmd-url"><a href="https://www.gnu.org/software/grep/manual/grep.html">https://www.gnu.org/software/grep/manual/grep.html</a></p>

Aliases: `egrep`, `fgrep`, `rgrep`

- Searches its file operands for a pattern within your workspace.
- Flag handling follows the command's own grammar (see examples).

**Examples:**

- `grep foo README.md`
- `grep -n TODO src/main.rs`
- `grep -r pattern src`
- `grep -i Error ./log.txt`

### `head`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#head-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#head-invocation</a></p>

Aliases: `ghead`

- Reads its file operands to stdout within your workspace.
- Allowed standalone flags: --help, --quiet, --silent, --verbose, --version, --zero-terminated, -V, -h, -q, -v, -z
- Allowed valued flags: --bytes, --lines, -c, -n
- Bare invocation reads stdin

**Examples:**

- `head README.md`
- `head -n 20 src/main.rs`
- `head -20 src/main.rs`

### `hunspell`
<p class="cmd-url"><a href="https://hunspell.github.io/">https://hunspell.github.io/</a></p>

- Allowed standalone flags: -1, -D, -G, -H, -L, -O, -P, -X, -a, -h, -l, -m, -n, -r, -s, -t, -u, -v, -w, -x, --help, --version
- Allowed valued flags: -d, -p, -i, -o

### `hyphen`
<p class="cmd-url"><a href="https://github.com/hunspell/hyphen">https://github.com/hunspell/hyphen</a></p>

- Hyphen-prefixed positional arguments accepted

### `iconv`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/iconv.1.html">https://man7.org/linux/man-pages/man1/iconv.1.html</a></p>

- Allowed standalone flags: -V, -c, -h, -l, -s, --help, --list, --silent, --version
- Allowed valued flags: -f, -t, --from-code, --to-code

### `ispell`
<p class="cmd-url"><a href="https://www.cs.hmc.edu/~geoff/ispell.html">https://www.cs.hmc.edu/~geoff/ispell.html</a></p>

- Allowed standalone flags: -a, -A, -l, -v, -V, --help, --version
- Allowed valued flags: -d, -p, -w, -T

### `join`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#join-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#join-invocation</a></p>

Aliases: `gjoin`

- Allowed standalone flags: --check-order, --header, --help, --ignore-case, --nocheck-order, --version, --zero-terminated, -V, -h, -i, -z
- Allowed valued flags: --empty, --output-delimiter, -1, -2, -a, -e, -j, -o, -t, -v

### `jot`
<p class="cmd-url"><a href="https://man.freebsd.org/cgi/man.cgi?jot">https://man.freebsd.org/cgi/man.cgi?jot</a></p>

- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `lam`
<p class="cmd-url"><a href="https://man.freebsd.org/cgi/man.cgi?lam">https://man.freebsd.org/cgi/man.cgi?lam</a></p>

- Hyphen-prefixed positional arguments accepted

### `less`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/less.1.html">https://man7.org/linux/man-pages/man1/less.1.html</a></p>

- Allowed standalone flags: -E, -F, -G, -I, -J, -K, -L, -M, -N, -Q, -R, -S, -V, -W, -X, -a, -c, -e, -f, -g, -i, -m, -n, -q, -r, -s, -w, --QUIT-AT-EOF, --RAW-CONTROL-CHARS, --chop-long-lines, --help, --ignore-case, --no-init, --quiet, --quit-at-eof, --quit-if-one-screen, --raw-control-chars, --silent, --squeeze-blank-lines, --version
- Allowed valued flags: -P, -b, -h, -j, -p, -t, -x, -y, -z, --LINE-NUMBERS, --LONG-PROMPT, --pattern, --prompt, --shift, --tabs, --tag, --window
- Bare invocation allowed

### `lessecho`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/lessecho.1.html">https://man7.org/linux/man-pages/man1/lessecho.1.html</a></p>

- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `lesskey`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/lesskey.1.html">https://man7.org/linux/man-pages/man1/lesskey.1.html</a></p>

- Allowed standalone flags: --help, --version, -V
- Allowed valued flags: --output, -o
- Bare invocation allowed

### `localedef`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/localedef.1.html">https://man7.org/linux/man-pages/man1/localedef.1.html</a></p>

- Allowed standalone flags: --alias-file, --force, --help, --list-archive, --no-archive, --no-hard-links, --no-warnings, --posix, --quiet, --replace, --verbose, --version, --warnings, -A, -c, -f, -i, -u, -v
- Allowed valued flags: --charmap, --inputfile, --prefix, --repertoire-map

### `mandoc`
<p class="cmd-url"><a href="https://mandoc.bsd.lv/man/mandoc.1.html">https://mandoc.bsd.lv/man/mandoc.1.html</a></p>

- Allowed standalone flags: -a, -c, -man, -mdoc
- Allowed valued flags: -I, -K, -O, -T, -W
- Bare invocation allowed

### `more`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/more.1.html">https://man7.org/linux/man-pages/man1/more.1.html</a></p>

- Allowed standalone flags: -V, -c, -d, -f, -h, -l, -p, -s, -u, --help, --version
- Allowed valued flags: -n, --lines
- Bare invocation allowed

### `msgattrib`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/msgattrib-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/msgattrib-Invocation.html</a></p>

- Allowed standalone flags: --help, --version
- Allowed valued flags: --output-file, -o
- Hyphen-prefixed positional arguments accepted

### `msgcat`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/msgcat-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/msgcat-Invocation.html</a></p>

- Allowed standalone flags: --help, --version, -h, -V
- Allowed valued flags: --output-file, --files-from, -o, -f, -D
- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `msgcmp`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/msgcmp-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/msgcmp-Invocation.html</a></p>

- Allowed standalone flags: --help, --version, -h, -V
- Hyphen-prefixed positional arguments accepted

### `msgcomm`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/msgcomm-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/msgcomm-Invocation.html</a></p>

- Allowed standalone flags: --help, --version
- Allowed valued flags: --output-file, -o, -D
- Hyphen-prefixed positional arguments accepted

### `msgconv`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/msgconv-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/msgconv-Invocation.html</a></p>

- Allowed standalone flags: --help, --version
- Allowed valued flags: --output-file, --to-code, -o, -t
- Hyphen-prefixed positional arguments accepted

### `msgen`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/msgen-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/msgen-Invocation.html</a></p>

- Allowed standalone flags: --help, --version
- Allowed valued flags: --output-file, -o
- Hyphen-prefixed positional arguments accepted

### `msgexec`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/msgexec-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/msgexec-Invocation.html</a></p>

- Recursively validates the inner command.

### `msgfilter`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/msgfilter-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/msgfilter-Invocation.html</a></p>

- Recursively validates the inner command.

### `msgfmt`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/msgfmt-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/msgfmt-Invocation.html</a></p>

- Allowed standalone flags: --help, --version, --check, --statistics, --verbose, -c, -v
- Allowed valued flags: --output-file, --directory, -o, -d, -D, -l
- Hyphen-prefixed positional arguments accepted

### `msggrep`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/msggrep-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/msggrep-Invocation.html</a></p>

- Allowed standalone flags: --help, --version, -e, -K, -T, -C, -J, -N
- Allowed valued flags: --output-file, --regexp, -o, -E
- Hyphen-prefixed positional arguments accepted

### `msginit`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/msginit-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/msginit-Invocation.html</a></p>

- Allowed standalone flags: --help, --version, --no-translator
- Allowed valued flags: --input, --output-file, --locale, -i, -o, -l
- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `msgmerge`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/msgmerge-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/msgmerge-Invocation.html</a></p>

- Allowed standalone flags: --help, --version, --update, --backup, -U, -N
- Allowed valued flags: --output-file, -o, -D
- Hyphen-prefixed positional arguments accepted

### `msgunfmt`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/msgunfmt-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/msgunfmt-Invocation.html</a></p>

- Allowed standalone flags: --help, --version
- Allowed valued flags: --output-file, -o
- Hyphen-prefixed positional arguments accepted

### `msguniq`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/msguniq-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/msguniq-Invocation.html</a></p>

- Allowed standalone flags: --help, --version, --repeated, --unique
- Allowed valued flags: --output-file, -o
- Hyphen-prefixed positional arguments accepted

### `ncal`
<p class="cmd-url"><a href="https://man.freebsd.org/cgi/man.cgi?ncal">https://man.freebsd.org/cgi/man.cgi?ncal</a></p>

- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `ngettext`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/ngettext-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/ngettext-Invocation.html</a></p>

- Allowed standalone flags: --help, --version, -E, -h, -V
- Allowed valued flags: --domain, --env, -d, -e

### `nl`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#nl-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#nl-invocation</a></p>

Aliases: `gnl`

- Allowed standalone flags: -V, -p, --help, --no-renumber, --version
- Allowed valued flags: -b, -d, -f, -h, -i, -l, -n, -s, -v, -w, --body-numbering, --footer-numbering, --header-numbering, --join-blank-lines, --line-increment, --number-format, --number-separator, --number-width, --section-delimiter, --starting-line-number
- Bare invocation allowed

### `nohup`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#nohup-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#nohup-invocation</a></p>

Aliases: `gnohup`

- Recursively validates the inner command.

### `nroff`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/nroff.1.html">https://man7.org/linux/man-pages/man1/nroff.1.html</a></p>

- Allowed standalone flags: -S, -V, -c, -h, -i, -k, -p, -q, -t, --help, --version
- Allowed valued flags: -M, -P, -T, -d, -m, -n, -o, -r, -w

### `paste`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#paste-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#paste-invocation</a></p>

Aliases: `gpaste`

- Allowed standalone flags: -V, -h, -s, -z, --help, --serial, --version, --zero-terminated
- Allowed valued flags: -d, --delimiters
- Bare invocation allowed

### `perl`
<p class="cmd-url"><a href="https://perldoc.perl.org/perl">https://perldoc.perl.org/perl</a></p>

- Allowed: -e/-E inline one-liners with safe built-in functions, --version, --help, -v, -V. Requires -e/-E flag. Code is validated against a safe identifier allowlist.

### `pr`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#pr-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#pr-invocation</a></p>

Aliases: `gpr`

- Allowed standalone flags: --double-space, --expand-tabs, --first-line-number, --form-feed, --help, --join-lines, --merge, --no-file-warnings, --number-lines, --omit-header, --omit-pagination, --page-width, --pages, --show-control-chars, --show-nonprinting, --show-tabs, --version, -F, -J, -T, -V, -d, -f, -h, -l, -m, -r, -t, -v
- Allowed valued flags: --columns, --header, --indent, --length, --separator, --sep-string, --width, -N, -S, -W, -e, -i, -n, -o, -s, -w
- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `ptx`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#ptx-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#ptx-invocation</a></p>

Aliases: `gptx`

- Allowed standalone flags: --auto-reference, --flag-truncation, --help, --ignore-case, --references, --version, -A, -G, -O, -T, -f, -r
- Allowed valued flags: --break-file, --gap-size, --ignore-file, --macro-name, --only-file, --sentence-regexp, --width, --word-regexp, --format, -F, -M, -S, -W, -b, -g, -i, -o, -w
- Bare invocation allowed

### `recode`
<p class="cmd-url"><a href="https://github.com/rrthomas/recode">https://github.com/rrthomas/recode</a></p>

- Allowed standalone flags: --copy, --force, --help, --known, --list, --quiet, --silent, --touch, --verbose, --version, -V, -f, -h, -k, -l, -q, -t, -v
- Allowed valued flags: --charsets-source, --graphics, --header, --language, --source, --strict, -C, -S, -d, -g, -i, -s

### `recode-sr-latin`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/recode_002dsr_002dlatin-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/recode_002dsr_002dlatin-Invocation.html</a></p>

- Allowed standalone flags: --help, --version, -h, -V
- Bare invocation allowed

### `rename`
<p class="cmd-url"><a href="https://metacpan.org/dist/File-Rename/view/rename">https://metacpan.org/dist/File-Rename/view/rename</a></p>

- Allowed standalone flags: --force, --help, --man, --no-act, --verbose, --version, -V, -f, -n, -v
- Allowed valued flags: --expr, -e

### `rev`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/rev.1.html">https://man7.org/linux/man-pages/man1/rev.1.html</a></p>

Aliases: `grev`

- Allowed standalone flags: -V, -h, --help, --version
- Bare invocation allowed

### `s2p`
<p class="cmd-url"><a href="https://perldoc.perl.org/s2p">https://perldoc.perl.org/s2p</a></p>

Aliases: `psed`

- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `sdiff`
<p class="cmd-url"><a href="https://www.gnu.org/software/diffutils/manual/diffutils.html#sdiff-Invocation">https://www.gnu.org/software/diffutils/manual/diffutils.html#sdiff-Invocation</a></p>

- Allowed standalone flags: --expand-tabs, --help, --ignore-all-space, --ignore-blank-lines, --ignore-case, --ignore-space-change, --ignore-tab-expansion, --left-column, --minimal, --speed-large-files, --strip-trailing-cr, --suppress-common-lines, --text, --version, -B, -E, -H, -V, -a, -b, -d, -h, -i, -l, -s, -t
- Allowed valued flags: --diff-program, --ignore-matching-lines, --tabsize, --width, -I, -W

### `sed`
<p class="cmd-url"><a href="https://www.gnu.org/software/sed/manual/sed.html">https://www.gnu.org/software/sed/manual/sed.html</a></p>

Aliases: `gsed`

- Edits its file operands in place within your workspace.
- Flag handling follows the command's own grammar (see examples).

**Examples:**

- `sed s/foo/bar/ ./file.txt`
- `sed -n 1,10p ./file.txt`

### `shred`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#shred-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#shred-invocation</a></p>

Aliases: `gshred`

- Allowed standalone flags: --exact, --force, --help, --remove, --verbose, --version, --zero, -f, -u, -v, -x, -z
- Allowed valued flags: --iterations, --random-source, --size, -n, -s

### `soelim`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/soelim.1.html">https://man7.org/linux/man-pages/man1/soelim.1.html</a></p>

- Allowed standalone flags: --compatible, --help, --raw, --tex, --version, -C, -r, -t, -v
- Allowed valued flags: -I
- Bare invocation allowed

### `stdbuf`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#stdbuf-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#stdbuf-invocation</a></p>

- Recursively validates the inner command.

### `tab2space`
<p class="cmd-url"><a href="https://man.freebsd.org/cgi/man.cgi?tab2space">https://man.freebsd.org/cgi/man.cgi?tab2space</a></p>

- Hyphen-prefixed positional arguments accepted

### `tac`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#tac-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#tac-invocation</a></p>

Aliases: `gtac`

- Allowed standalone flags: -V, -b, -h, -r, --before, --help, --regex, --version
- Allowed valued flags: -s, --separator
- Bare invocation allowed

### `tail`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#tail-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#tail-invocation</a></p>

Aliases: `gtail`

- Reads its file operands to stdout within your workspace.
- Allowed standalone flags: --follow, --help, --quiet, --retry, --silent, --verbose, --version, --zero-terminated, -F, -V, -f, -h, -q, -r, -v, -z
- Allowed valued flags: --bytes, --lines, --max-unchanged-stats, --pid, --sleep-interval, -b, -c, -n
- Bare invocation reads stdin

**Examples:**

- `tail -n 50 src/main.rs`
- `tail -f ./app.log`

### `tee`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#tee-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#tee-invocation</a></p>

Aliases: `gtee`

- Allowed standalone flags: --append, --help, --ignore-interrupts, --output-error, --version, -a, -i, -p
- Bare invocation allowed

### `textutil`
<p class="cmd-url"><a href="https://ss64.com/mac/textutil.html">https://ss64.com/mac/textutil.html</a></p>

- Allowed standalone flags: -info, -cat, -convert, -strip, -stdin, -stdout, -help, -noload
- Allowed valued flags: -format, -encoding, -extension, -fontname, -fontsize, -inputencoding, -output, -outputdir
- Hyphen-prefixed positional arguments accepted

### `tidy`
<p class="cmd-url"><a href="https://www.html-tidy.org/">https://www.html-tidy.org/</a></p>

- Allowed standalone flags: -asxhtml, -asxml, -ashtml, -clean, -config, -help, -help-config, -help-option, -i, -indent, -language, -m, -modify, -n, -numeric, -omit, -q, -quiet, -show-config, -show-body-only, -upper, -utf8, -v, -version, -w, -xml, -h, --help, --version
- Allowed valued flags: -access, -encoding, -errors, -file, -output, -show-errors, -wrap, -o
- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `tput`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/tput.1.html">https://man7.org/linux/man-pages/man1/tput.1.html</a></p>

- Allowed standalone flags: -S, -V
- Allowed valued flags: -T
- Hyphen-prefixed positional arguments accepted

### `tr`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#tr-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#tr-invocation</a></p>

Aliases: `gtr`

- Allowed standalone flags: -C, -V, -c, -d, -h, -s, --complement, --delete, --help, --squeeze-repeats, --truncate-set1, --version

### `tsort`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#tsort-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#tsort-invocation</a></p>

Aliases: `gtsort`

- Allowed standalone flags: --help, --version
- Bare invocation allowed

### `uconv`
<p class="cmd-url"><a href="https://unicode-org.github.io/icu/userguide/icu/utilities.html#uconv">https://unicode-org.github.io/icu/userguide/icu/utilities.html#uconv</a></p>

- Allowed standalone flags: --help, --version, -?, -h, -V, -i, -l, -L, -s, -v, --invalid-skip, --list-code, --show-cps, --silent, --verbose
- Allowed valued flags: --add-signature, --callback, --copyright, --encoding, --from-callback, --from-code, --match, --name, --output, --to-callback, --to-code, --transliterate, -c, -f, -o, -t, -x

### `ul`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/ul.1.html">https://man7.org/linux/man-pages/man1/ul.1.html</a></p>

- Allowed standalone flags: --help, --version, -V, -h, -i
- Allowed valued flags: --terminal, -t
- Bare invocation allowed

### `unexpand`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#unexpand-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#unexpand-invocation</a></p>

Aliases: `gunexpand`

- Allowed standalone flags: -V, -a, -h, --all, --first-only, --help, --version
- Allowed valued flags: -t, --tabs
- Bare invocation allowed

### `unifdef`
<p class="cmd-url"><a href="https://dotat.at/prog/unifdef/">https://dotat.at/prog/unifdef/</a></p>

Aliases: `unifdefall`

- Allowed standalone flags: -h, -V
- Allowed valued flags: -o
- Hyphen-prefixed positional arguments accepted

### `uniq`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#uniq-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#uniq-invocation</a></p>

Aliases: `guniq`

- Allowed standalone flags: -D, -V, -c, -d, -h, -i, -u, -z, --count, --help, --ignore-case, --repeated, --unique, --version, --zero-terminated
- Allowed valued flags: -f, -s, -w, --all-repeated, --check-chars, --group, --skip-chars, --skip-fields
- Bare invocation allowed

### `units`
<p class="cmd-url"><a href="https://www.gnu.org/software/units/">https://www.gnu.org/software/units/</a></p>

- Allowed standalone flags: --check, --compact, --exponential, --help, --one-line, --quiet, --strict, --terse, --units, --verbose, --version, -1, -V, -c, -e, -h, -q, -s, -t, -v
- Allowed valued flags: --digits, --file, --history, --log, --output-format, -d, -f, -H, -L, -o
- Bare invocation allowed

### `wc`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#wc-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#wc-invocation</a></p>

Aliases: `gwc`

- Reads its file operands to stdout within your workspace.
- Allowed standalone flags: --bytes, --chars, --help, --lines, --max-line-length, --version, --words, --zero-terminated, -L, -V, -c, -l, -m, -w
- Bare invocation reads stdin

**Examples:**

- `wc -l src/main.rs`
- `wc -w README.md`

### `wdiff`
<p class="cmd-url"><a href="https://www.gnu.org/software/wdiff/">https://www.gnu.org/software/wdiff/</a></p>

- Allowed standalone flags: --auto-pager, --avoid-wraps, --copyright, --end-delete, --end-insert, --help, --ignore-case, --less-mode, --line-numbers, --no-common, --no-deleted, --no-init-term, --no-inserted, --printer, --start-delete, --start-insert, --statistics, --terminal, --version, -1, -2, -3, -C, -V, -a, -c, -d, -h, -i, -l, -n, -p, -s, -t, -w, -y, -z
- Allowed valued flags: --diff-input, -W

### `what`
<p class="cmd-url"><a href="https://man.freebsd.org/cgi/man.cgi?what">https://man.freebsd.org/cgi/man.cgi?what</a></p>

- Allowed standalone flags: -V, -h, -q, -s

### `word-list-compress`
<p class="cmd-url"><a href="https://aspell.net/">https://aspell.net/</a></p>

- Allowed standalone flags: c, d

### `xgettext`
<p class="cmd-url"><a href="https://www.gnu.org/software/gettext/manual/html_node/xgettext-Invocation.html">https://www.gnu.org/software/gettext/manual/html_node/xgettext-Invocation.html</a></p>

- Allowed standalone flags: --help, --version, --join-existing, -j
- Allowed valued flags: --default-domain, --directory, --files-from, --output, --output-dir, --language, --from-code, --keyword, -d, -D, -f, -o, -p, -L, -k
- Hyphen-prefixed positional arguments accepted

### `xmlcatalog`
<p class="cmd-url"><a href="https://gitlab.gnome.org/GNOME/libxml2">https://gitlab.gnome.org/GNOME/libxml2</a></p>

- Allowed standalone flags: --shell, --sgml, --verbose, -v, --noout, --create
- Allowed valued flags: --add, --del
- Hyphen-prefixed positional arguments accepted

### `xsltproc`
<p class="cmd-url"><a href="https://gitlab.gnome.org/GNOME/libxslt">https://gitlab.gnome.org/GNOME/libxslt</a></p>

- Allowed standalone flags: --catalogs, --debug, --debugger, --dumpextensions, --encoding, --help, --html, --load-trace, --maxdepth, --noblanks, --nodict, --nomkdir, --nonet, --novalid, --nowrite, --pedantic, --profile, --repeat, --timing, --verbose, --version, --writesubtree, --xinclude, -v, -V
- Allowed valued flags: --maxparserdepth, --maxvars, --output, --param, --stringparam, -o, -p
- Hyphen-prefixed positional arguments accepted

### `yes`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#yes-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#yes-invocation</a></p>

Aliases: `gyes`

- Allowed standalone flags: --help, --version
- Bare invocation allowed

### `zcat`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/zcat.1.html">https://man7.org/linux/man-pages/man1/zcat.1.html</a></p>

Aliases: `gzcat`

- Allowed standalone flags: -V, -f, -h, -q, -v, --force, --help, --quiet, --verbose, --version
- Bare invocation allowed

### `zlib-flate`
<p class="cmd-url"><a href="https://qpdf.readthedocs.io/">https://qpdf.readthedocs.io/</a></p>

- Allowed standalone flags: -compress, -uncompress
- Allowed valued flags: -level
- Bare invocation allowed

