# JVM

### `clj`
<p class="cmd-url"><a href="https://clojure.org/reference/deps_and_cli">https://clojure.org/reference/deps_and_cli</a></p>

Aliases: `clojure`

- Allowed standalone flags: --help, --version, -Sdescribe, -Spath, -Stree, -Sverbose, -h, -help

### `detekt`
<p class="cmd-url"><a href="https://detekt.dev/docs/gettingstarted/cli/">https://detekt.dev/docs/gettingstarted/cli/</a></p>

- Allowed standalone flags: --build-upon-default-config, --debug, --help, --parallel, --version, -V, -h
- Allowed valued flags: --baseline, --classpath, --config, --config-resource, --excludes, --includes, --input, --jvm-target, --language-version, --plugins, --report
- Bare invocation allowed

### `gradle`
<p class="cmd-url"><a href="https://docs.gradle.org/current/userguide/command_line_interface.html">https://docs.gradle.org/current/userguide/command_line_interface.html</a></p>

Aliases: `gradlew`

- **build**: Flags: --build-cache, --configure-on-demand, --console, --continue, --dry-run, --help, --info, --no-build-cache, --no-daemon, --no-parallel, --no-rebuild, --parallel, --profile, --quiet, --rerun-tasks, --scan, --stacktrace, --warning-mode, -h, -q. Valued: --exclude-task, --max-workers, -x
- **check**: Flags: --build-cache, --configure-on-demand, --console, --continue, --dry-run, --help, --info, --no-build-cache, --no-daemon, --no-parallel, --no-rebuild, --parallel, --profile, --quiet, --rerun-tasks, --scan, --stacktrace, --warning-mode, -h, -q. Valued: --exclude-task, --max-workers, -x
- **dependencies**: Flags: --console, --help, --info, --no-rebuild, --quiet, --stacktrace, --warning-mode, -h, -q. Valued: --configuration
- **properties**: Flags: --console, --help, --info, --no-rebuild, --quiet, --stacktrace, --warning-mode, -h, -q
- **tasks**: Flags: --all, --console, --help, --info, --no-rebuild, --quiet, --stacktrace, --warning-mode, -h, -q. Valued: --group
- **test**: Flags: --build-cache, --configure-on-demand, --console, --continue, --dry-run, --help, --info, --no-build-cache, --no-daemon, --no-parallel, --no-rebuild, --parallel, --profile, --quiet, --rerun-tasks, --scan, --stacktrace, --warning-mode, -h, -q. Valued: --exclude-task, --max-workers, -x
- Allowed standalone flags: --help, --version, -V, -h

### `groovy`
<p class="cmd-url"><a href="https://groovy-lang.org/single-page-documentation.html">https://groovy-lang.org/single-page-documentation.html</a></p>

- Allowed standalone flags: --help, --version, -h, -v

### `jar`
<p class="cmd-url"><a href="https://docs.oracle.com/en/java/javase/21/docs/specs/man/jar.html">https://docs.oracle.com/en/java/javase/21/docs/specs/man/jar.html</a></p>

- List mode only: tf, tvf, --list, -t. Also --version, --help.

### `jarsigner`
<p class="cmd-url"><a href="https://docs.oracle.com/en/java/javase/21/docs/specs/man/jarsigner.html">https://docs.oracle.com/en/java/javase/21/docs/specs/man/jarsigner.html</a></p>

- Requires -verify. - Allowed standalone flags: -certs, -strict, -verbose, -verify, -help, -h

### `javap`
<p class="cmd-url"><a href="https://docs.oracle.com/en/java/javase/21/docs/specs/man/javap.html">https://docs.oracle.com/en/java/javase/21/docs/specs/man/javap.html</a></p>

- Allowed standalone flags: --help, --version, -V, -c, -constants, -h, -l, -p, -private, -protected, -public, -s, -sysinfo, -v, -verbose
- Allowed valued flags: --module, -bootclasspath, -classpath, -cp, -m

### `jcmd`
<p class="cmd-url"><a href="https://docs.oracle.com/en/java/javase/21/docs/specs/man/jcmd.html">https://docs.oracle.com/en/java/javase/21/docs/specs/man/jcmd.html</a></p>

- Allowed standalone flags: --help, -h, -help, -l
- Bare invocation allowed

### `jdeps`
<p class="cmd-url"><a href="https://docs.oracle.com/en/java/javase/21/docs/specs/man/jdeps.html">https://docs.oracle.com/en/java/javase/21/docs/specs/man/jdeps.html</a></p>

- Allowed standalone flags: --api-only, --check, --ignore-missing-deps, --inverse, --jdk-internals, --list-deps, --list-reduced-deps, --missing-deps, --print-module-deps, --regex, -R, -V, -apionly, -c, -cp, -dotoutput, -e, -f, -filter, -h, -help, -include, -jdkinternals, -m, -module, -multi-release, -p, -package, -q, -quiet, -recursive, -regex, -s, -summary, -v, -verbose, -version
- Allowed valued flags: --add-modules, --class-path, --dot-output, --filter, --ignore-missing-deps, --include, --module, --module-path, --multi-release, --print-module-deps, --regex, --require, --upgrade-module-path, -classpath, -cp

### `jenv`
<p class="cmd-url"><a href="https://www.jenv.be/">https://www.jenv.be/</a></p>

- **add**: Flags: --help, --skip-existing, -h
- **commands**: Flags: --help, --sh, --no-sh, -h
- **disable-plugin**: Flags: --help, -h
- **doctor**: Flags: --help, -h
- **enable-plugin**: Flags: --help, -h
- **global**: Flags: --help, --unset, -h
- **help**: Positional args accepted
- **hooks**: Flags: --help, -h
- **init** (requires -): Flags: -, --help, -h
- **javahome**: Flags: --help, -h
- **local**: Flags: --help, --unset, -h
- **plugin list**: Flags: --help, -h
- **plugin ls**: Flags: --help, -h
- **plugins**: Flags: --help, -h
- **prefix**: Flags: --help, -h
- **rehash**: Flags: --help, -h
- **remove**: Flags: --help, -h
- **root**: Flags: --help, -h
- **shell**: Flags: --help, --unset, -h
- **shims**: Flags: --help, --short, -h
- **version**: Flags: --help, -h
- **version-file**: Flags: --help, -h
- **version-file-read**: Flags: --help, -h
- **version-name**: Flags: --help, -h
- **version-origin**: Flags: --help, -h
- **versions**: Flags: --bare, --help, -h
- **whence**: Flags: --help, --path, -h
- **which**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

### `jstack`
<p class="cmd-url"><a href="https://docs.oracle.com/en/java/javase/21/docs/specs/man/jstack.html">https://docs.oracle.com/en/java/javase/21/docs/specs/man/jstack.html</a></p>

- Allowed standalone flags: --help, --version, -h, -help, -version

### `keytool`
<p class="cmd-url"><a href="https://docs.oracle.com/en/java/javase/21/docs/specs/man/keytool.html">https://docs.oracle.com/en/java/javase/21/docs/specs/man/keytool.html</a></p>

- **-list**: Flags: --help, -h, -rfc, -v. Valued: -alias, -keystore, -storepass, -storetype
- **-printcert**: Flags: --help, -h, -rfc, -v. Valued: -file, -jarfile
- Allowed standalone flags: --help, --version, -V, -h

### `kotlinc`
<p class="cmd-url"><a href="https://kotlinlang.org/docs/command-line.html">https://kotlinlang.org/docs/command-line.html</a></p>

- Allowed standalone flags: --help, -Xjava-source-roots, -Xjvm-default, -Xnew-inference, -Xno-call-assertions, -Xno-receiver-assertions, -help, -include-runtime, -java-parameters, -jvm-target, -language-version, -no-jdk, -no-reflect, -no-stdlib, -nowarn, -progressive, -script, -verbose, -version, -Werror, -X, -h
- Allowed valued flags: --release, -Xfriend-paths, -Xplugin, -api-version, -classpath, -cp, -d, -expression, -jdk-home, -kotlin-home, -language-version, -module-name, -opt-in, -script-templates

### `ktlint`
<p class="cmd-url"><a href="https://pinterest.github.io/ktlint/latest/">https://pinterest.github.io/ktlint/latest/</a></p>

- Allowed standalone flags: --color, --color-name, --help, --relative, --verbose, --version, -V, -h
- Allowed valued flags: --editorconfig, --reporter
- Bare invocation allowed

### `lein`
<p class="cmd-url"><a href="https://leiningen.org/">https://leiningen.org/</a></p>

- **help**: Positional args accepted
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h, -v

### `mill`
<p class="cmd-url"><a href="https://mill-build.com/">https://mill-build.com/</a></p>

- Allowed standalone flags: --bell, --debug, --disable-callgraph, --disable-prompt, --enable-prompt, --help, --interactive, --no-server, --silent, --ticker, --version, --watch, -V, -b, -d, -h, -i, -s, -w

### `mvn / mvnw`
<p class="cmd-url"><a href="https://maven.apache.org/ref/current/maven-embedder/cli.html">https://maven.apache.org/ref/current/maven-embedder/cli.html</a></p>

- Phases: compile, dependency:list, dependency:tree, help:describe, test, test-compile, validate, verify.

### `sbt`
<p class="cmd-url"><a href="https://www.scala-sbt.org/">https://www.scala-sbt.org/</a></p>

- Allowed standalone flags: --debug, --error, --help, --info, --jvm-debug, --no-colors, --no-server, --no-share, --numeric-version, --script-version, --supershell, --traces, --verbose, --version, --warn, -V, -d, -h, -v

### `scala`
<p class="cmd-url"><a href="https://docs.scala-lang.org/scala3/reference/cli/cli.html">https://docs.scala-lang.org/scala3/reference/cli/cli.html</a></p>

- Allowed standalone flags: --help, --version, -h, -help, -version, -V

### `scalac`
<p class="cmd-url"><a href="https://docs.scala-lang.org/scala3/reference/cli/scalac.html">https://docs.scala-lang.org/scala3/reference/cli/scalac.html</a></p>

- Allowed standalone flags: --help, --version, -Werror, -Xfatal-warnings, -color, -deprecation, -encoding, -explain, -explain-types, -feature, -help, -language, -new-syntax, -no-indent, -old-syntax, -print, -print-tasty, -rewrite, -shasum, -source, -uniqid, -unchecked, -usejavacp, -verbose, -version, -Wunused, -Xignore-scala2-macros, -explain, -explain-types, -language, -rewrite, -deprecation, -feature, -Yshow-suppressed-errors, -Wnonunit-statement, -h
- Allowed valued flags: --release, -Xss, -bootclasspath, -classpath, -cp, -d, -encoding, -extdirs, -javabootclasspath, -language, -release, -source, -sourcepath, -target

### `scalafix`
<p class="cmd-url"><a href="https://scalacenter.github.io/scalafix/">https://scalacenter.github.io/scalafix/</a></p>

- Allowed standalone flags: --auto-classpath, --check, --debug, --diff, --exclude, --help, --include, --non-interactive, --no-stale-semanticdb, --quiet-parse-errors, --scalac-options, --stdout, --stdin, --syntactic, --test, --triggered, --verbose, --version, -V, -h, -q
- Allowed valued flags: --auto-classpath-roots, --bash, --classpath, --config, --diff-base, --exclude, --files, --format, --include, --rules, --scala-version, --source-root, --sourceroot, --tool-classpath, -c, -r
- Bare invocation allowed

### `scalafmt`
<p class="cmd-url"><a href="https://scalameta.org/scalafmt/">https://scalameta.org/scalafmt/</a></p>

- Allowed standalone flags: --check, --debug, --diff, --diff-branch, --exclude, --git, --help, --include, --list, --non-interactive, --quiet, --respect-project-filters, --stdin, --stdout, --test, --verbose, --version, -V, -h, -q
- Allowed valued flags: --config, --config-str, --dialect, --exclude, --include, -c
- Bare invocation allowed

### `sdk`
<p class="cmd-url"><a href="https://sdkman.io/usage">https://sdkman.io/usage</a></p>

- **current**: Flags: --help, -h
- **default**: Flags: --help, -h
- **env clear**: Flags: --help, -h
- **env init**: Flags: --help, -h
- **env install**: Flags: --help, -h
- **flush**: Flags: --help, -h
- **help**: Positional args accepted
- **home**: Flags: --help, -h
- **install**: Flags: --help, -h
- **list**: Flags: --help, -h
- **offline**: Flags: --help, -h
- **uninstall**: Flags: --help, -h
- **update**: Flags: --help, -h
- **upgrade**: Flags: --help, -h
- **use**: Flags: --help, -h
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

