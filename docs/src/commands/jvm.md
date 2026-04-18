# JVM

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

### `keytool`
<p class="cmd-url"><a href="https://docs.oracle.com/en/java/javase/21/docs/specs/man/keytool.html">https://docs.oracle.com/en/java/javase/21/docs/specs/man/keytool.html</a></p>

- **-list**: Flags: --help, -h, -rfc, -v. Valued: -alias, -keystore, -storepass, -storetype
- **-printcert**: Flags: --help, -h, -rfc, -v. Valued: -file, -jarfile
- Allowed standalone flags: --help, --version, -V, -h

### `ktlint`
<p class="cmd-url"><a href="https://pinterest.github.io/ktlint/latest/">https://pinterest.github.io/ktlint/latest/</a></p>

- Allowed standalone flags: --color, --color-name, --help, --relative, --verbose, --version, -V, -h
- Allowed valued flags: --editorconfig, --reporter
- Bare invocation allowed

### `mvn / mvnw`
<p class="cmd-url"><a href="https://maven.apache.org/ref/current/maven-embedder/cli.html">https://maven.apache.org/ref/current/maven-embedder/cli.html</a></p>

- Phases: compile, dependency:list, dependency:tree, help:describe, test, test-compile, validate, verify.

