# Shell Builtins

### `:`
<p class="cmd-url"><a href="https://www.gnu.org/software/bash/manual/bash.html#Bourne-Shell-Builtins">https://www.gnu.org/software/bash/manual/bash.html#Bourne-Shell-Builtins</a></p>

- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `alias`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/alias.1p.html">https://man7.org/linux/man-pages/man1/alias.1p.html</a></p>

- Allowed standalone flags: -p
- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `bash / sh`
<p class="cmd-url"><a href="https://www.gnu.org/software/bash/manual/bash.html">https://www.gnu.org/software/bash/manual/bash.html</a></p>

- Allowed: --version, --help, `bash -c` / `sh -c` with a safe inner command.

### `break / continue`
<p class="cmd-url"><a href="https://www.gnu.org/software/bash/manual/bash.html#index-break">https://www.gnu.org/software/bash/manual/bash.html#index-break</a></p>

- Bare invocation or a single non-negative integer level (e.g. `break`, `break 2`).

### `command`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/command.1p.html">https://man7.org/linux/man-pages/man1/command.1p.html</a></p>

- Requires -v, -V, --version. - Allowed standalone flags: --help, --version, -h
- Allowed valued flags: -v, -V

### `declare`
<p class="cmd-url"><a href="https://www.gnu.org/software/bash/manual/html_node/Bash-Builtins.html">https://www.gnu.org/software/bash/manual/html_node/Bash-Builtins.html</a></p>

Aliases: `typeset`

- Allowed standalone flags: -A, -F, -a, -f, -g, -i, -l, -n, -p, -r, -t, -u, -x
- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `exit`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/exit.1p.html">https://man7.org/linux/man-pages/man1/exit.1p.html</a></p>

- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `export`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/export.1p.html">https://man7.org/linux/man-pages/man1/export.1p.html</a></p>

- Allowed standalone flags: -f, -n, -p
- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `false`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#false-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#false-invocation</a></p>

- Allowed standalone flags: --help, --version, -V, -h
- Bare invocation allowed

### `hash`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/hash.1p.html">https://man7.org/linux/man-pages/man1/hash.1p.html</a></p>

- Allowed standalone flags: -d, -l, -r, -t
- Allowed valued flags: -p
- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `hostname`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/hostname.1.html">https://man7.org/linux/man-pages/man1/hostname.1.html</a></p>

- Allowed standalone flags: --help, --version, -A, -I, -V, -d, -f, -h, -i, -s
- Bare invocation allowed

### `printenv`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#printenv-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#printenv-invocation</a></p>

- Allowed standalone flags: --help, --null, --version, -0, -V, -h
- Bare invocation allowed

### `read`
<p class="cmd-url"><a href="https://pubs.opengroup.org/onlinepubs/9799919799/utilities/read.html">https://pubs.opengroup.org/onlinepubs/9799919799/utilities/read.html</a></p>

- Allowed standalone flags: -r, -s
- Allowed valued flags: -a, -d, -n, -p, -t, -u
- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `shopt`
<p class="cmd-url"><a href="https://www.gnu.org/software/bash/manual/bash.html#The-Shopt-Builtin">https://www.gnu.org/software/bash/manual/bash.html#The-Shopt-Builtin</a></p>

- Allowed standalone flags: --help, -h, -o, -p, -q, -s, -u
- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `true`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#true-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#true-invocation</a></p>

- Allowed standalone flags: --help, --version, -V, -h
- Bare invocation allowed

### `type`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/type.1p.html">https://man7.org/linux/man-pages/man1/type.1p.html</a></p>

- Allowed standalone flags: --help, --version, -P, -V, -a, -f, -h, -p, -t

### `unset`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/unset.1p.html">https://man7.org/linux/man-pages/man1/unset.1p.html</a></p>

- Allowed standalone flags: --help, --version, -V, -f, -h, -n, -v
- Bare invocation allowed

### `wait`
<p class="cmd-url"><a href="https://pubs.opengroup.org/onlinepubs/9799919799/utilities/wait.html">https://pubs.opengroup.org/onlinepubs/9799919799/utilities/wait.html</a></p>

- Allowed standalone flags: --help, --version, -V, -h
- Bare invocation allowed

### `whereis`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/whereis.1.html">https://man7.org/linux/man-pages/man1/whereis.1.html</a></p>

- Allowed standalone flags: --help, --version, -V, -b, -h, -l, -m, -s, -u
- Allowed valued flags: -B, -M, -S, -f

### `which`
<p class="cmd-url"><a href="https://man7.org/linux/man-pages/man1/which.1.html">https://man7.org/linux/man-pages/man1/which.1.html</a></p>

- Allowed standalone flags: --all, --help, --version, -V, -a, -h, -s

### `whoami`
<p class="cmd-url"><a href="https://www.gnu.org/software/coreutils/manual/coreutils.html#whoami-invocation">https://www.gnu.org/software/coreutils/manual/coreutils.html#whoami-invocation</a></p>

- Allowed standalone flags: --help, --version, -V, -h
- Bare invocation allowed

### `xargs`
<p class="cmd-url"><a href="https://www.gnu.org/software/findutils/manual/html_mono/find.html#Invoking-xargs">https://www.gnu.org/software/findutils/manual/html_mono/find.html#Invoking-xargs</a></p>

- Recursively validates the inner command. Skips xargs-specific flags (-I, -L, -n, -P, -s, -E, -d, -0, -r, -t, -p, -x).

