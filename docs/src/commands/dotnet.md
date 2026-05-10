# .NET

### `cake`
<p class="cmd-url"><a href="https://cakebuild.net/docs/running-builds/runners/cake-tool">https://cakebuild.net/docs/running-builds/runners/cake-tool</a></p>

- Allowed standalone flags: --help, --info, --version, -h

### `csi`
<p class="cmd-url"><a href="https://learn.microsoft.com/en-us/visualstudio/scripting/scripting-with-the-csi-utility">https://learn.microsoft.com/en-us/visualstudio/scripting/scripting-with-the-csi-utility</a></p>

- Allowed standalone flags: --help, --version, -?, -help

### `docfx`
<p class="cmd-url"><a href="https://dotnet.github.io/docfx/reference/docfx-cli-reference/docfx.html">https://dotnet.github.io/docfx/reference/docfx-cli-reference/docfx.html</a></p>

- **build**: Flags: --changesFile, --cleanupCacheHistory, --debug, --disableGitFeatures, --dryRun, --exportRawModel, --exportViewModel, --force, --forcePostProcess, --help, --keepFileLink, --lruSize, --maxParallelism, --noLangKeyword, --postProcessors, --rawModelOutputFolder, --serve, --templateFolder, --viewModelOutputFolder, -?, -f, -h. Valued: --changesFile, --exportRawModel, --exportViewModel, --globalMetadata, --globalMetadataFiles, --fileMetadata, --fileMetadataFiles, --logLevel, --logFile, --lruSize, --markdownEngineName, --markdownEngineProperties, --maxParallelism, --output, --postProcessors, --templateFolder, --theme, -l, -o, -t. Positional args accepted
- **help**: Positional args accepted
- **init**: Flags: --help, --quiet, --yes, -h, -q, -y. Valued: --output, -o
- **metadata**: Flags: --disableDefaultFilter, --disableGitFeatures, --force, --help, --namespaceLayout, --shouldSkipMarkup, -?, -h. Valued: --filter, --globalNamespaceId, --logLevel, --logFile, --memberLayout, --namespaceLayout, --outputFormat, --output, --property, --repositoryRoot, -l, -o, -p. Positional args accepted
- **template list**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h, -v

### `dotnet`
<p class="cmd-url"><a href="https://learn.microsoft.com/en-us/dotnet/core/tools/">https://learn.microsoft.com/en-us/dotnet/core/tools/</a></p>

- **build**: Flags: --force, --help, --no-dependencies, --no-incremental, --no-restore, --nologo, --self-contained, --tl, --use-current-runtime, -h. Valued: --arch, --artifacts-path, --configuration, --framework, --os, --output, --property, --runtime, --source, --verbosity, --version-suffix, -a, -c, -f, -o, -p, -r, -s, -v
- **clean**: Flags: --help, --nologo, --tl, -h. Valued: --artifacts-path, --configuration, --framework, --output, --runtime, --verbosity, -c, -f, -o, -r, -v
- **format**: Flags: --exclude-generated, --folder, --help, --include-generated, --no-restore, --verify-no-changes, -h. Valued: --binarylog, --diagnostics, --exclude, --exclude-diagnostics, --include, --report, --severity, --verbosity, -v
- **help**: Positional args accepted
- **list**: Flags: --deprecated, --help, --highest-minor, --highest-patch, --include-prerelease, --include-transitive, --outdated, --vulnerable, -h. Valued: --config, --format, --framework, --source, --verbosity, -v
- **new**: Flags: --diagnostics, --dry-run, --force, --help, --list, --update-apply, --update-check, -d, -h, -l. Valued: --author, --columns, --columns-all, --language, --name, --no-update-check, --output, --project, --type, --verbosity, -lang, -n, -o
- **nuget list**: Flags: --help, -h
- **nuget verify**: Flags: --help, -h, --all, -v. Valued: --certificate-fingerprint, --configfile, --verbosity. Positional args accepted
- **nuget**: Flags: --help, -h
- **pack**: Flags: --force, --help, --include-source, --include-symbols, --no-build, --no-dependencies, --no-restore, --nologo, --serviceable, --tl, -h. Valued: --artifacts-path, --configuration, --output, --property, --runtime, --source, --verbosity, --version-suffix, -c, -o, -p, -r, -s, -v
- **publish**: Flags: --disable-build-servers, --force, --help, --no-build, --no-dependencies, --no-restore, --nologo, --self-contained, --tl, --use-current-runtime, -h. Valued: --arch, --artifacts-path, --configuration, --framework, --manifest, --os, --output, --property, --runtime, --source, --verbosity, --version-suffix, -a, -c, -f, -o, -p, -r, -s, -v
- **restore**: Flags: --disable-build-servers, --disable-parallel, --force, --force-evaluate, --help, --ignore-failed-sources, --interactive, --no-cache, --no-dependencies, --no-http-cache, --use-current-runtime, -h. Valued: --arch, --artifacts-path, --configfile, --lock-file-path, --locked-mode, --os, --packages, --runtime, --source, --verbosity, -a, -r, -s, -v
- **sln add**: Flags: --help, --in-root, -h. Valued: --solution-folder, -s
- **sln list**: Flags: --help, --solution-folders, -h
- **sln remove**: Flags: --help, -h
- **sln**: Flags: --help, -h
- **test**: Flags: --blame, --blame-crash, --blame-hang, --force, --help, --list-tests, --no-build, --no-dependencies, --no-restore, --nologo, -h. Valued: --arch, --artifacts-path, --blame-crash-collect-always, --blame-crash-dump-type, --blame-hang-dump-type, --blame-hang-timeout, --collect, --configuration, --diag, --environment, --filter, --framework, --logger, --os, --output, --property, --results-directory, --runtime, --settings, --test-adapter-path, --verbosity, -a, -c, -d, -e, -f, -l, -o, -r, -s, -v
- **tool list**: Flags: --global, --help, --local, -g, -h. Valued: --tool-path
- **tool**: Flags: --help, -h
- **workload list**: Flags: --help, -h. Valued: --verbosity, -v
- **workload**: Flags: --help, -h
- Allowed standalone flags: --help, --info, --list-runtimes, --list-sdks, --version, -V, -h

### `dotnet-counters`
<p class="cmd-url"><a href="https://learn.microsoft.com/en-us/dotnet/core/diagnostics/dotnet-counters">https://learn.microsoft.com/en-us/dotnet/core/diagnostics/dotnet-counters</a></p>

- **help**: Positional args accepted
- **list**: Flags: --help, -h
- **ps**: Flags: --help, -h
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

### `dotnet-dump`
<p class="cmd-url"><a href="https://learn.microsoft.com/en-us/dotnet/core/diagnostics/dotnet-dump">https://learn.microsoft.com/en-us/dotnet/core/diagnostics/dotnet-dump</a></p>

- **help**: Positional args accepted
- **ps**: Flags: --help, -h
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

### `dotnet-ef`
<p class="cmd-url"><a href="https://learn.microsoft.com/en-us/ef/core/cli/dotnet">https://learn.microsoft.com/en-us/ef/core/cli/dotnet</a></p>

- **database list**: Flags: --help, --no-build, --prefix-output, -h. Valued: --configuration, --framework, --json, --project, --startup-project, -p, -s
- **database**: Flags: --help, -h, --verbose
- **dbcontext info**: Flags: --help, --no-build, --prefix-output, -h. Valued: --configuration, --context, --framework, --json, --project, --runtime, --startup-project, -c, -p, -s
- **dbcontext list**: Flags: --help, --no-build, --prefix-output, -h. Valued: --configuration, --framework, --json, --project, --startup-project, -p, -s
- **dbcontext**: Flags: --help, -h, --verbose
- **help**: Positional args accepted
- **migrations has-pending-model-changes**: Flags: --help, --no-build, -h. Valued: --configuration, --context, --framework, --project, --startup-project, -c, -p, -s
- **migrations list**: Flags: --help, --no-build, --no-color, --no-connect, --prefix-output, -h. Valued: --configuration, --connection, --context, --framework, --json, --msbuildprojectextensionspath, --project, --runtime, --startup-project, -c, -p, -s
- **migrations**: Flags: --help, -h, --verbose
- Allowed standalone flags: --help, --version, -h, -v

### `dotnet-gcdump`
<p class="cmd-url"><a href="https://learn.microsoft.com/en-us/dotnet/core/diagnostics/dotnet-gcdump">https://learn.microsoft.com/en-us/dotnet/core/diagnostics/dotnet-gcdump</a></p>

- **help**: Positional args accepted
- **ps**: Flags: --help, -h
- **report**: Flags: --help, -h. Valued: --type. Positional args accepted
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

### `dotnet-script`
<p class="cmd-url"><a href="https://github.com/dotnet-script/dotnet-script">https://github.com/dotnet-script/dotnet-script</a></p>

- Allowed standalone flags: --help, --info, --version, -h

### `dotnet-sos`
<p class="cmd-url"><a href="https://learn.microsoft.com/en-us/dotnet/core/diagnostics/dotnet-sos">https://learn.microsoft.com/en-us/dotnet/core/diagnostics/dotnet-sos</a></p>

- **help**: Positional args accepted
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

### `dotnet-stack`
<p class="cmd-url"><a href="https://learn.microsoft.com/en-us/dotnet/core/diagnostics/dotnet-stack">https://learn.microsoft.com/en-us/dotnet/core/diagnostics/dotnet-stack</a></p>

- **help**: Positional args accepted
- **ps**: Flags: --help, -h
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

### `dotnet-symbol`
<p class="cmd-url"><a href="https://github.com/dotnet/symstore/blob/main/src/dotnet-symbol/README.md">https://github.com/dotnet/symstore/blob/main/src/dotnet-symbol/README.md</a></p>

- Allowed standalone flags: --debugging, --diagnostics, --help, --host-only, --ignore-errors, --internal-server, --microsoft-symbol-server, --modules, --no-symbols, --quiet, --recurse-subdirectories, --symbols, --version, -d, -h, -r
- Allowed valued flags: --authenticated-server-path, --cache-directory, --output, --server-path, --symstore, --timeout, -o

### `dotnet-trace`
<p class="cmd-url"><a href="https://learn.microsoft.com/en-us/dotnet/core/diagnostics/dotnet-trace">https://learn.microsoft.com/en-us/dotnet/core/diagnostics/dotnet-trace</a></p>

- **convert**: Flags: --help, -h. Valued: --format, --output, -o. Positional args accepted
- **help**: Positional args accepted
- **list-profiles**: Flags: --help, -h
- **ps**: Flags: --help, -h
- **report**: Flags: --help, -h. Valued: --max-depth. Positional args accepted
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -h

### `fantomas`
<p class="cmd-url"><a href="https://github.com/fsprojects/fantomas">https://github.com/fsprojects/fantomas</a></p>

- Allowed standalone flags: --check, --daemon, --force, --help, --out, --profile, --quiet, --recurse, --standalone, --strict, --verbose, --version, -h, -r
- Allowed valued flags: --config, --exclude, --include, --out, --parallel

### `fsi`
<p class="cmd-url"><a href="https://learn.microsoft.com/en-us/dotnet/fsharp/tools/fsharp-interactive/">https://learn.microsoft.com/en-us/dotnet/fsharp/tools/fsharp-interactive/</a></p>

Aliases: `dotnet-fsi`

- Allowed standalone flags: --help, --version, -?, -h

### `mcs`
<p class="cmd-url"><a href="https://www.mono-project.com/docs/about-mono/languages/csharp/">https://www.mono-project.com/docs/about-mono/languages/csharp/</a></p>

- Allowed standalone flags: --help, --version, -help, /checked-, /checked, /codepage:auto, /debug, /debug+, /debug-, /help, /nologo, /noconfig, /nostdlib, /nowarn, /optimize, /optimize+, /optimize-, /recurse, /target:exe, /target:library, /target:winexe, /target:module, /unsafe, /unsafe+, /unsafe-, /v, /warnaserror, /warnaserror+, /warnaserror-, -?, -help, -langversion
- Allowed valued flags: /d, /define, /doc, /keycontainer, /keyfile, /l, /lib, /main, /out, /pkg, /r, /recurse, /reference, /target, /warn, /warnaserror, /win32icon, /win32res

### `mono`
<p class="cmd-url"><a href="https://www.mono-project.com/docs/about-mono/">https://www.mono-project.com/docs/about-mono/</a></p>

- Allowed standalone flags: --help, --version, -V, -h, --info

### `msbuild`
<p class="cmd-url"><a href="https://learn.microsoft.com/en-us/visualstudio/msbuild/msbuild-command-line-reference">https://learn.microsoft.com/en-us/visualstudio/msbuild/msbuild-command-line-reference</a></p>

- Allowed standalone flags: /binaryLogger, /check, /clp, /consoleLoggerParameters, /debug, /detailedSummary, /distributedFileLogger, /distributedlogger, /dl, /ds, /fileLogger, /fl, /getItem, /getProperty, /getTargetResult, /graphBuild, /help, /ignoreProjectExtensions, /inputResultsCaches, /interactive, /isolateProjects, /lowPriority, /m, /maxcpucount, /multiProc, /nodeReuse, /noAutoResponse, /noConsoleLogger, /noLogo, /nologo, /nor, /p:RestoreNoCache=true, /preprocess, /profileEvaluation, /q, /quiet, /r, /restore, /restoreProperty, /t, /target, /terminalLogger, /tl, /toolsVersion, /tv, /v, /validate, /verbosity, /version, /warnAsError, /warnAsMessage, /warnNotAsError, /ver, /?, -binaryLogger, -check, -clp, -consoleLoggerParameters, -debug, -detailedSummary, -distributedFileLogger, -distributedlogger, -dl, -ds, -fileLogger, -fl, -getItem, -getProperty, -getTargetResult, -graphBuild, -help, -ignoreProjectExtensions, -interactive, -isolateProjects, -lowPriority, -m, -maxcpucount, -multiProc, -nodeReuse, -noAutoResponse, -noConsoleLogger, -noLogo, -nologo, -nor, -preprocess, -profileEvaluation, -q, -quiet, -r, -restore, -restoreProperty, -t, -target, -terminalLogger, -tl, -toolsVersion, -tv, -v, -validate, -verbosity, -version, -warnAsError, -warnAsMessage, -warnNotAsError, -ver, --help
- Allowed valued flags: /binaryLogger, /clp, /consoleLoggerParameters, /distributedFileLogger, /distributedlogger, /fileLogger, /fileLoggerParameters, /flp, /getProperty, /getItem, /getTargetResult, /inputResultsCaches, /logger, /maxcpucount, /nodeReuse, /outputResultsCache, /p, /preprocess, /profileEvaluation, /property, /restore, /target, /terminalLogger, /toolsVersion, /tv, /verbosity, /warnAsError, /warnAsMessage, /warnNotAsError, -binaryLogger, -clp, -consoleLoggerParameters, -distributedFileLogger, -distributedlogger, -fileLogger, -fileLoggerParameters, -flp, -getProperty, -getItem, -getTargetResult, -inputResultsCaches, -logger, -maxcpucount, -nodeReuse, -outputResultsCache, -p, -preprocess, -profileEvaluation, -property, -restore, -target, -terminalLogger, -toolsVersion, -tv, -verbosity, -warnAsError, -warnAsMessage, -warnNotAsError
- Bare invocation allowed
- Hyphen-prefixed positional arguments accepted

### `nuget`
<p class="cmd-url"><a href="https://learn.microsoft.com/en-us/nuget/reference/nuget-exe-cli-reference">https://learn.microsoft.com/en-us/nuget/reference/nuget-exe-cli-reference</a></p>

- **config**: Flags: --help, -?, -AsPath, -help. Valued: -ConfigFile. Positional args accepted
- **help**: Positional args accepted
- **list**: Flags: --help, -?, -AllVersions, -IncludeDelisted, -NonInteractive, -PreRelease, -help. Valued: -ConfigFile, -Source, -Verbosity. Positional args accepted
- **locals**: Flags: --help, -?, -clear, -help, -list. Valued: -Verbosity. Positional args accepted
- **search**: Flags: --help, -?, -PreRelease, -help. Valued: -ConfigFile, -Source, -Take, -Verbosity. Positional args accepted
- **sources**: Flags: --help, -?, -Format, -help. Valued: -ConfigFile, -Format, -Name, -Password, -Source, -StorePasswordInClearText, -Username, -ProtocolVersion, -Verbosity. Positional args accepted
- **spec**: Flags: --help, -?, -Force, -help. Valued: -AssemblyPath. Positional args accepted
- **verify**: Flags: --help, -?, -All, -Signatures, -help. Valued: -CertificateFingerprint, -ConfigFile, -Verbosity. Positional args accepted
- Allowed standalone flags: --help, -?, -h, -help

### `nuke`
<p class="cmd-url"><a href="https://nuke.build/docs/getting-started/setup/">https://nuke.build/docs/getting-started/setup/</a></p>

- Allowed standalone flags: --help, --version, --?, -h, -?

### `paket`
<p class="cmd-url"><a href="https://fsprojects.github.io/Paket/paket-commands.html">https://fsprojects.github.io/Paket/paket-commands.html</a></p>

- **find-package-versions**: Flags: --help, -h. Valued: --max, --source. Positional args accepted
- **find-packages**: Flags: --help, -h. Valued: --max, --source. Positional args accepted
- **find-refs**: Flags: --help, -h. Valued: --group. Positional args accepted
- **help**: Positional args accepted
- **info**: Flags: --help, --paket-version, -h
- **outdated**: Flags: --help, --ignore-constraints, --include-prereleases, --strict, -h. Valued: --group
- **show-groups**: Flags: --help, -h
- **show-installed-packages**: Flags: --all, --help, -h. Valued: --group, --package, --project
- **version**: Flags: --help, -h
- **why**: Flags: --details, --help, -h. Valued: --group. Positional args accepted
- Allowed standalone flags: --help, --version, -h, -v

