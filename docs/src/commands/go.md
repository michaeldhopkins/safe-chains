# Go

### `buf`
<p class="cmd-url"><a href="https://buf.build/docs/">https://buf.build/docs/</a></p>

- **breaking**: Flags: --exclude-imports, --help, --limit-to-input-files, -h. Valued: --against, --against-config, --config, --error-format, --exclude-path, --path. Positional args accepted
- **build**: Flags: --as-file-descriptor-set, --disable-symlinks, --exclude-imports, --exclude-source-info, --help, -h. Valued: --config, --exclude-path, --output, --path, -o. Positional args accepted
- **completion**: Flags: --help, -h. Positional args accepted
- **convert**: Flags: --from-format, --help, --to-format, -h. Valued: --from, --output, --to, --type, -o. Positional args accepted
- **format**: Flags: --diff, --exit-code, --help, --write, -d, -h, -w. Valued: --config, --exclude-path, --output, --path, -o. Positional args accepted
- **generate**: Flags: --clean, --debug, --help, --include-imports, --include-types, --include-wkt, -h. Valued: --config, --exclude-path, --include-types, --output, --path, --template, --type, -o. Positional args accepted
- **help**: Positional args accepted
- **lint**: Flags: --exclude-imports, --help, -h. Valued: --config, --error-format, --exclude-path, --path. Positional args accepted
- **ls-files**: Flags: --help, --include-imports, --include-import-paths, -h. Valued: --config, --format. Positional args accepted
- Allowed standalone flags: --help, --version, -h

### `dlv`
<p class="cmd-url"><a href="https://github.com/go-delve/delve">https://github.com/go-delve/delve</a></p>

- Allowed standalone flags: --help, --version, -h, -v

### `errcheck`
<p class="cmd-url"><a href="https://github.com/kisielk/errcheck">https://github.com/kisielk/errcheck</a></p>

- Allowed standalone flags: --abspath, --asserts, --blank, --exclude, --help, --ignoretests, --ignoregenerated, --mod, --tags, --verbose, -abspath, -asserts, -blank, -exclude, -h, -ignoregenerated, -ignoretests, -mod, -tags, -verbose
- Bare invocation allowed

### `gci`
<p class="cmd-url"><a href="https://github.com/daixiang0/gci">https://github.com/daixiang0/gci</a></p>

- **diff**: Flags: --debug, --help, --no-prefix-comments, --skip-generated, --skip-vendor, -h. Valued: --custom-order, --section, -s. Positional args accepted
- **help**: Positional args accepted
- **list**: Flags: --debug, --help, --skip-generated, --skip-vendor, -h. Valued: --section, -s. Positional args accepted
- **print**: Flags: --debug, --help, --no-prefix-comments, --skip-generated, --skip-vendor, -h. Valued: --custom-order, --section, -s. Positional args accepted
- **write**: Flags: --debug, --help, --no-prefix-comments, --skip-generated, --skip-vendor, -h. Valued: --custom-order, --section, -s. Positional args accepted
- Allowed standalone flags: --help, -h

### `go`
<p class="cmd-url"><a href="https://pkg.go.dev/cmd/go">https://pkg.go.dev/cmd/go</a></p>

- **build**: Flags: --help, -a, -asan, -cover, -h, -linkshared, -modcacherw, -msan, -n, -race, -trimpath, -v, -work, -x. Valued: -asmflags, -buildmode, -buildvcs, -compiler, -covermode, -coverpkg, -gccgoflags, -gcflags, -installsuffix, -ldflags, -mod, -modfile, -o, -overlay, -p, -pgo, -pkgdir, -tags
- **doc**: Flags: --help, -all, -c, -cmd, -h, -short, -src, -u
- **env**: Flags: --help, -h, -json
- **help**: Positional args accepted
- **list**: Flags: --help, -a, -asan, -compiled, -cover, -deps, -e, -export, -find, -h, -linkshared, -m, -modcacherw, -msan, -n, -race, -retract, -test, -trimpath, -u, -v, -versions, -work, -x. Valued: -asmflags, -buildmode, -buildvcs, -compiler, -covermode, -coverpkg, -f, -gccgoflags, -gcflags, -installsuffix, -json, -ldflags, -mod, -modfile, -overlay, -p, -pgo, -pkgdir, -reuse, -tags
- **test**: Flags: --help, -a, -asan, -benchmem, -cover, -failfast, -h, -json, -linkshared, -modcacherw, -msan, -n, -race, -short, -trimpath, -v, -work, -x. Valued: -asmflags, -bench, -benchtime, -blockprofile, -blockprofilerate, -buildmode, -buildvcs, -compiler, -count, -covermode, -coverpkg, -coverprofile, -cpu, -cpuprofile, -fuzz, -fuzzminimizetime, -fuzztime, -gccgoflags, -gcflags, -installsuffix, -ldflags, -list, -memprofile, -memprofilerate, -mod, -modfile, -mutexprofile, -mutexprofilefraction, -o, -outputdir, -overlay, -p, -parallel, -pgo, -pkgdir, -run, -shuffle, -skip, -tags, -timeout, -trace
- **version**: Flags: --help, -h, -m, -v
- **vet**: Flags: --help, -a, -asan, -cover, -h, -json, -linkshared, -modcacherw, -msan, -n, -race, -trimpath, -v, -work, -x. Valued: -asmflags, -buildmode, -buildvcs, -c, -compiler, -covermode, -coverpkg, -gccgoflags, -gcflags, -installsuffix, -ldflags, -mod, -modfile, -overlay, -p, -pgo, -pkgdir, -tags
- Allowed standalone flags: --help, --version, -V, -h

### `goenv`
<p class="cmd-url"><a href="https://github.com/go-nv/goenv">https://github.com/go-nv/goenv</a></p>

- **completions**: Flags: --help, -h
- **help**: Flags: --help, -h
- **hooks**: Flags: --help, -h
- **init** (requires -): Flags: -, --help, --no-rehash, -h
- **prefix**: Flags: --help, -h
- **root**: Flags: --help, -h
- **shims**: Flags: --help, -h
- **version**: Flags: --help, -h
- **version-file**: Flags: --help, -h
- **version-name**: Flags: --help, -h
- **version-origin**: Flags: --help, -h
- **versions**: Flags: --bare, --help, -h
- **whence**: Flags: --help, -h
- **which**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

**Examples:**

- `goenv versions`
- `goenv version`
- `goenv which go`
- `eval "$(goenv init -)"`
- `eval "$(goenv init - bash)"`
- `eval "$(goenv init - zsh --no-rehash)"`

### `gofmt`
<p class="cmd-url"><a href="https://pkg.go.dev/cmd/gofmt">https://pkg.go.dev/cmd/gofmt</a></p>

- Allowed standalone flags: --cpuprofile, --d, --e, --help, --l, --s, --w, -d, -e, -h, -l, -s, -w
- Allowed valued flags: --r, -r
- Bare invocation allowed

### `gofumpt`
<p class="cmd-url"><a href="https://github.com/mvdan/gofumpt">https://github.com/mvdan/gofumpt</a></p>

- Allowed standalone flags: --cpuprofile, --d, --diff, --e, --extra, --help, --l, --list, --write, --w, -d, -e, -h, -l, -w
- Allowed valued flags: --lang, --modpath
- Bare invocation allowed

### `goimports`
<p class="cmd-url"><a href="https://pkg.go.dev/golang.org/x/tools/cmd/goimports">https://pkg.go.dev/golang.org/x/tools/cmd/goimports</a></p>

- Allowed standalone flags: --cpuprofile, --d, --e, --format-only, --help, --l, --srcdir, --v, --w, -d, -e, -h, -l, -v, -w
- Allowed valued flags: --local, -local, --srcdir, -srcdir
- Bare invocation allowed

### `golangci-lint`
<p class="cmd-url"><a href="https://golangci-lint.run/">https://golangci-lint.run/</a></p>

- **help**: Positional args accepted
- **linters**: Flags: --help, -h
- **run**: Flags: --allow-parallel-runners, --help, --json, --new, --no-config, --print-issued-lines, --print-linter-name, --show-stats, --verbose, -h, -v. Valued: --build-tags, --color, --concurrency, --config, --disable, --enable, --exclude, --go, --max-issues-per-linter, --max-same-issues, --out-format, --path-prefix, --skip-dirs, --skip-files, --sort-results, --timeout, -D, -E, -c, -e, -p
- **version**: Flags: --help, --format, -h
- Allowed standalone flags: --help, --version, -h

### `gomodifytags`
<p class="cmd-url"><a href="https://github.com/fatih/gomodifytags">https://github.com/fatih/gomodifytags</a></p>

- Allowed standalone flags: --add-options, --all, --clear-tags, --clear-options, --format, --help, --quiet, --remove-tags, --remove-options, --skip-unexported, --sort, --transform, --w, -h, -w
- Allowed valued flags: --add-tags, --field, --file, --line, --modified, --offset, --override, --remove-options, --remove-tags, --struct, --template, --transform, -add-tags, -clear-tags, -field, -file, -line, -modified, -offset, -override, -remove-options, -remove-tags, -struct, -template, -transform

### `goreleaser`
<p class="cmd-url"><a href="https://goreleaser.com/">https://goreleaser.com/</a></p>

- **check**: Flags: --help, --quiet, -h, -q. Valued: --config, --load-env-files, -f
- **completion**: Flags: --help, -h. Positional args accepted
- **healthcheck**: Flags: --help, --quiet, -h, -q
- **help**: Positional args accepted
- **init**: Flags: --help, -h. Valued: --config, -f
- **jsonschema**: Flags: --help, -h. Valued: --output, -o
- **schema**: Flags: --help, -h. Valued: --output, -o
- **verify**: Flags: --help, -h, --debug. Valued: -f, --config, --checksum-file. Positional args accepted
- Allowed standalone flags: --help, --version, -h, -v

### `gosec`
<p class="cmd-url"><a href="https://github.com/securego/gosec">https://github.com/securego/gosec</a></p>

- Allowed standalone flags: --help, --exclude-generated, --no-fail, --nosec, --quiet, --sort, --stdout, --track-suppressions, --verbose, -h, -q
- Allowed valued flags: --concurrency, --conf, --confidence, --exclude, --exclude-dir, --fmt, --include, --out, --severity, --tags, -conf, -confidence, -exclude, -exclude-dir, -fmt, -include, -out, -severity, -tags

### `gotestsum`
<p class="cmd-url"><a href="https://github.com/gotestyourself/gotestsum">https://github.com/gotestyourself/gotestsum</a></p>

- Allowed standalone flags: --debug, --dry-run, --force-cache, --format-hide-empty-pkg, --help, --ignore-non-json-output-lines, --no-color, --rerun-fails-only-root-cases, --watch, --watch-chdir, -h
- Allowed valued flags: --format, --format-hivis, --format-icons, --hide-summary, --junitfile, --junitfile-hide-empty-pkg, --junitfile-project-name, --junitfile-testcase-classname, --junitfile-testsuite-name, --max-fails, --packages, --post-run-command, --raw-command, --rerun-fails, --rerun-fails-max-failures, --rerun-fails-report, --rerun-fails-run-root-test, --watch-skip-tests
- Bare invocation allowed

### `govulncheck`
<p class="cmd-url"><a href="https://pkg.go.dev/golang.org/x/vuln/cmd/govulncheck">https://pkg.go.dev/golang.org/x/vuln/cmd/govulncheck</a></p>

- Allowed standalone flags: --help, --json, --version, -h
- Allowed valued flags: -C, -db, -mode, -show, -tags, -test

### `mage`
<p class="cmd-url"><a href="https://magefile.org/">https://magefile.org/</a></p>

- Allowed standalone flags: --help, --list, --version, -V, -h, -l

### `revive`
<p class="cmd-url"><a href="https://revive.run/">https://revive.run/</a></p>

- Allowed standalone flags: --help, --version, -h
- Allowed valued flags: --config, --exclude, --formatter, --include, --max_open_files, --set_exit_status, -config, -exclude, -formatter, -include, -max_open_files, -set_exit_status
- Bare invocation allowed

### `staticcheck`
<p class="cmd-url"><a href="https://staticcheck.dev/">https://staticcheck.dev/</a></p>

- Allowed standalone flags: --debug.version, --help, --json, --matrix, --merge, --show-ignored, --tests, -f
- Allowed valued flags: -checks, -explain, -fail, -go, -tags

### `task`
<p class="cmd-url"><a href="https://taskfile.dev/">https://taskfile.dev/</a></p>

- Allowed standalone flags: --color, --concurrency, --dry, --exit-code, --force, --global, --help, --init, --insecure, --interval, --json, --list, --list-all, --no-color, --offline, --parallel, --silent, --status, --summary, --verbose, --version, --watch, -C, -d, -f, -g, -h, -i, -j, -l, -n, -p, -s, -t, -v, -w, -a
- Allowed valued flags: --dir, --output, --taskfile, -d, -o, -t

