# Xcode

### `agvtool`
<p class="cmd-url"><a href="https://developer.apple.com/library/archive/qa/qa1827/_index.html">https://developer.apple.com/library/archive/qa/qa1827/_index.html</a></p>

- **mvers**: Flags: --help, -h
- **vers**: Flags: --help, -h
- **what-marketing-version**: Flags: --help, -h
- **what-version**: Flags: --help, -h

### `codesign`
<p class="cmd-url"><a href="https://ss64.com/mac/codesign.html">https://ss64.com/mac/codesign.html</a></p>

- Requires --display, --verify, -d, -v. - Allowed standalone flags: --deep, --display, --verify, -R, -d, -v, --help, -h
- Allowed valued flags: --verbose

### `lipo`
<p class="cmd-url"><a href="https://ss64.com/mac/lipo.html">https://ss64.com/mac/lipo.html</a></p>

- Requires -archs, -detailed_info, -info, -verify_arch. - Allowed standalone flags: -archs, -detailed_info, -info, -verify_arch, --help, -h

### `periphery`
<p class="cmd-url"><a href="https://github.com/peripheryapp/periphery">https://github.com/peripheryapp/periphery</a></p>

- **scan**: Flags: --help, --quiet, --skip-build, --strict, --verbose, -h. Valued: --config, --format, --index-store-path, --project, --schemes, --targets
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

### `pkgutil`
<p class="cmd-url"><a href="https://ss64.com/mac/pkgutil.html">https://ss64.com/mac/pkgutil.html</a></p>

- Requires --check-signature, --export-plist, --file-info, --file-info-plist, --files, --group-pkgs, --groups, --groups-plist, --packages, --payload-files, --pkg-groups, --pkg-info, --pkg-info-plist, --pkgs, --pkgs-plist. - Allowed standalone flags: --check-signature, --export-plist, --file-info, --file-info-plist, --files, --group-pkgs, --groups, --groups-plist, --packages, --payload-files, --pkg-groups, --pkg-info, --pkg-info-plist, --pkgs, --pkgs-plist, --regexp, --help, -h
- Allowed valued flags: --volume

### `plutil`
<p class="cmd-url"><a href="https://ss64.com/mac/plutil.html">https://ss64.com/mac/plutil.html</a></p>

- **-lint**: Flags: --help, -h, -s
- **-p**: Flags: --help, -h
- **-type**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h, -help

### `pod`
<p class="cmd-url"><a href="https://guides.cocoapods.org/terminal/commands.html">https://guides.cocoapods.org/terminal/commands.html</a></p>

- **env**: Flags: --help, -h
- **info**: Flags: --help, -h
- **list**: Flags: --help, -h
- **search**: Flags: --help, --simple, --stats, --web, -h
- **spec cat**: Flags: --help, -h. Valued: --version
- **spec which**: Flags: --help, -h. Valued: --version
- Allowed standalone flags: --help, --version, -V, -h

### `simctl`
<p class="cmd-url"><a href="https://developer.apple.com/documentation/xcode/simctl">https://developer.apple.com/documentation/xcode/simctl</a></p>

- **list**: Flags: --help, --json, --verbose, -h, -j, -v

### `spctl`
<p class="cmd-url"><a href="https://ss64.com/mac/spctl.html">https://ss64.com/mac/spctl.html</a></p>

- Requires --assess, -a. - Allowed standalone flags: --assess, --verbose, -a, -v, --help, -h
- Allowed valued flags: --context, --type, -t

### `swiftformat`
<p class="cmd-url"><a href="https://github.com/nicklockwood/SwiftFormat">https://github.com/nicklockwood/SwiftFormat</a></p>

- Requires --dryrun, --lint. - Allowed standalone flags: --dryrun, --lenient, --lint, --quiet, --strict, --verbose, --help, -h
- Allowed valued flags: --config, --disable, --enable, --rules

### `swiftlint`
<p class="cmd-url"><a href="https://github.com/realm/SwiftLint">https://github.com/realm/SwiftLint</a></p>

- **analyze**: Flags: --help, --quiet, --strict, -h. Valued: --compiler-log-path, --config, --path, --reporter
- **lint**: Flags: --help, --no-cache, --quiet, --strict, -h. Valued: --config, --path, --reporter
- **reporters**: Flags: --help, -h
- **rules**: Flags: --disabled, --enabled, --help, -h. Valued: --config, --reporter
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

### `tuist`
<p class="cmd-url"><a href="https://docs.tuist.dev/en/cli/">https://docs.tuist.dev/en/cli/</a></p>

- **dump**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **graph**: Flags: --help, --json, --verbose, -h. Valued: --format, --path, -f, -p
- **hash cache**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **hash selective-testing**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **inspect build**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **inspect bundle**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **inspect dependencies**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **inspect implicit-imports**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **inspect redundant-imports**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **inspect test**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **migration check-empty-settings**: Flags: --help, -h. Valued: --path, -p
- **migration list-targets**: Flags: --help, -h. Valued: --path, -p
- **scaffold list**: Flags: --help, --json, -h. Valued: --path, -p
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

### `xcbeautify`
<p class="cmd-url"><a href="https://github.com/cpisciotta/xcbeautify">https://github.com/cpisciotta/xcbeautify</a></p>

- Allowed standalone flags: --help, --is-ci, --quiet, --quieter, --version, -V, -h, -q
- Allowed valued flags: --renderer
- Bare invocation allowed

### `xcode-select`
<p class="cmd-url"><a href="https://ss64.com/mac/xcode-select.html">https://ss64.com/mac/xcode-select.html</a></p>

- Allowed standalone flags: --help, --print-path, --version, -V, -h, -p, -v

### `xcodebuild`
<p class="cmd-url"><a href="https://developer.apple.com/documentation/xcode/xcodebuild">https://developer.apple.com/documentation/xcode/xcodebuild</a></p>

- **-list**: Flags: --help, -h, -json. Valued: -project, -workspace
- **-showBuildSettings**: Flags: --help, -h, -json. Valued: -configuration, -destination, -project, -scheme, -sdk, -target, -workspace
- **-showdestinations**: Flags: --help, -h, -json. Valued: -configuration, -destination, -project, -scheme, -sdk, -target, -workspace
- **-showsdks**: Flags: --help, -h, -json. Valued: -configuration, -destination, -project, -scheme, -sdk, -target, -workspace
- **-version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

### `xcodegen`
<p class="cmd-url"><a href="https://github.com/yonaskolb/XcodeGen">https://github.com/yonaskolb/XcodeGen</a></p>

- **dump**: Flags: --help, --no-env, --quiet, -h, -n, -q. Valued: --project-root, --spec, --type, -r, -s, -t
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

### `xcrun`
<p class="cmd-url"><a href="https://ss64.com/mac/xcrun.html">https://ss64.com/mac/xcrun.html</a></p>

- **--find**: Positional args accepted
- **--show-sdk-build-version**: Positional args accepted
- **--show-sdk-path**: Positional args accepted
- **--show-sdk-platform-path**: Positional args accepted
- **--show-sdk-platform-version**: Positional args accepted
- **--show-sdk-version**: Positional args accepted
- **--show-toolchain-path**: Positional args accepted
- **notarytool history**: Positional args accepted
- **notarytool info**: Positional args accepted
- **notarytool log**: Positional args accepted
- **simctl list**: Positional args accepted
- **stapler validate**: Positional args accepted

