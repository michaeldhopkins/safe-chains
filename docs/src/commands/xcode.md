# Xcode

### `actool`
<p class="cmd-url"><a href="https://keith.github.io/xcode-man-pages/actool.1.html">https://keith.github.io/xcode-man-pages/actool.1.html</a></p>

- Allowed standalone flags: --compress-pngs, --enable-on-demand-resources, --errors, --include-all-app-icons, --include-sticker-content, --notices, --print-contents, --skip-app-store-deployment, --version, --warnings, --help, -h
- Allowed valued flags: --accent-color, --alternate-app-icon, --app-icon, --asset-pack-output-specifications, --compile, --filter-for-device-model, --filter-for-device-os-version, --include-partial-info-plist-localizations, --launch-image, --minimum-deployment-target, --output-format, --output-partial-info-plist, --platform, --product-type, --standalone-icon-behavior, --sticker-pack-identifier-prefix, --sticker-pack-strings-file, --stickers-icon-role, --target-device, --widget-background-color

### `agvtool`
<p class="cmd-url"><a href="https://developer.apple.com/library/archive/qa/qa1827/_index.html">https://developer.apple.com/library/archive/qa/qa1827/_index.html</a></p>

- **bump**: Flags: --help, -all, -h
- **mvers**: Flags: --help, -h
- **new-marketing-version**: Flags: --help, -h
- **new-version**: Flags: --help, -all, -h
- **vers**: Flags: --help, -h
- **what-marketing-version**: Flags: --help, -h
- **what-version**: Flags: --help, -h

**Examples:**

- `agvtool what-version`
- `agvtool what-marketing-version`
- `agvtool vers`
- `agvtool mvers`
- `agvtool new-version 1.2.3`
- `agvtool new-version -all 1.2.3`
- `agvtool new-marketing-version 1.0`
- `agvtool bump`
- `agvtool bump -all`

### `codesign`
<p class="cmd-url"><a href="https://ss64.com/mac/codesign.html">https://ss64.com/mac/codesign.html</a></p>

- Requires --display, --verify, -d, -v. - Allowed standalone flags: --deep, --display, --verify, -R, -d, -v, --help, -h
- Allowed valued flags: --verbose

### `gen_bridge_metadata`
<p class="cmd-url"><a href="https://keith.github.io/xcode-man-pages/gen_bridge_metadata.1.html">https://keith.github.io/xcode-man-pages/gen_bridge_metadata.1.html</a></p>

- Allowed standalone flags: --64-bit, --arm64e, --debug, --no-32-bit, --no-64-bit, --private, --version, --help, -d, -h, -p, -v
- Allowed valued flags: --cflags, --cflags-64, --exception, --format, --framework, --output, -C, -F, -c, -e, -f, -o

### `genstrings`
<p class="cmd-url"><a href="https://keith.github.io/xcode-man-pages/genstrings.1.html">https://keith.github.io/xcode-man-pages/genstrings.1.html</a></p>

- Allowed standalone flags: -SwiftUI, -a, -bigEndian, -d, -littleEndian, -macRoman, -noPositionalParameters, -q, -u, --help, -h
- Allowed valued flags: -encoding, -o, -s, -skipTable

### `ibtool`
<p class="cmd-url"><a href="https://keith.github.io/xcode-man-pages/ibtool.1.html">https://keith.github.io/xcode-man-pages/ibtool.1.html</a></p>

- Allowed standalone flags: --all, --classes, --connections, --enable-auto-layout, --errors, --hierarchy, --localizable-all, --localizable-geometry, --localizable-other, --localizable-stringarrays, --localizable-strings, --localizable-to-many-relationships, --localize-incremental, --notices, --objects, --reference-external-strings-file, --remove-plugin-dependencies, --update-constraints, --update-frames, --upgrade, --version, --version-history, --warnings, --help, -h
- Allowed valued flags: --bundle, --companion-strings-file, --compile, --convert, --export, --export-strings-file, --export-xliff, --flatten, --import, --import-strings-file, --import-xliff, --incremental-file, --module, --output-format, --previous-file, --source-language, --strip, --target-language, --write

### `iconutil`
<p class="cmd-url"><a href="https://keith.github.io/xcode-man-pages/iconutil.1.html">https://keith.github.io/xcode-man-pages/iconutil.1.html</a></p>

- Allowed standalone flags: --help, -h
- Allowed valued flags: --convert, --output, -c, -o

### `layerutil`
<p class="cmd-url"><a href="https://keith.github.io/xcode-man-pages/layerutil.1.html">https://keith.github.io/xcode-man-pages/layerutil.1.html</a></p>

- Allowed standalone flags: --help, --palette-image, --version, -V, -c, -h
- Allowed valued flags: --display-gamut, --flattened-image, --gpu-compression, --lossy-compression, --output, --scale, -f, -g, -l, -o, -p, -s

### `lipo`
<p class="cmd-url"><a href="https://ss64.com/mac/lipo.html">https://ss64.com/mac/lipo.html</a></p>

- Requires -archs, -detailed_info, -info, -verify_arch. - Allowed standalone flags: -archs, -detailed_info, -info, -verify_arch, --help, -h

### `mig`
<p class="cmd-url"><a href="https://keith.github.io/xcode-man-pages/mig.1.html">https://keith.github.io/xcode-man-pages/mig.1.html</a></p>

- Allowed standalone flags: -B, -K, -L, -MD, -Q, -S, -V, -b, -cpp, -k, -l, -q, -s, -split, -v
- Allowed valued flags: -arch, -cc, -dheader, -header, -i, -iheader, -isysroot, -maxonstack, -migcom, -server, -sheader, -user

### `migcom`
<p class="cmd-url"><a href="https://keith.github.io/xcode-man-pages/migcom.1.html">https://keith.github.io/xcode-man-pages/migcom.1.html</a></p>

- Allowed standalone flags: -B, -K, -L, -Q, -S, -V, -b, -k, -l, -q, -s, -split, -v
- Allowed valued flags: -dheader, -header, -i, -iheader, -maxonstack, -server, -sheader, -user

### `periphery`
<p class="cmd-url"><a href="https://github.com/peripheryapp/periphery">https://github.com/peripheryapp/periphery</a></p>

- **check-update**: Flags: --help, -h
- **clear-cache**: Flags: --help, -h
- **scan**: Flags: --bazel, --bazel-check-visibility, --clean-build, --disable-redundant-public-analysis, --disable-unused-import-analysis, --disable-update-check, --exclude-tests, --help, --no-color, --no-superfluous-ignore-comments, --quiet, --relative-results, --retain-assign-only-properties, --retain-codable-properties, --retain-encodable-properties, --retain-objc-accessible, --retain-objc-annotated, --retain-public, --retain-swift-ui-previews, --retain-unused-protocol-func-params, --skip-build, --skip-schemes-validation, --strict, --superfluous-ignore-comments, --verbose, -h. Valued: --baseline, --bazel-filter, --bazel-index-store, --color, --config, --exclude-targets, --external-codable-protocols, --external-encodable-protocols, --external-test-case-classes, --format, --generic-project-config, --index-exclude, --index-store-path, --json-package-manifest-path, --no-retain-spi, --project, --project-root, --report-exclude, --report-include, --retain-assign-only-property-types, --retain-files, --retain-unused-imported-modules, --schemes, --write-baseline, --write-results
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

### `pkgutil`
<p class="cmd-url"><a href="https://ss64.com/mac/pkgutil.html">https://ss64.com/mac/pkgutil.html</a></p>

- Requires --check-signature, --export-plist, --file-info, --file-info-plist, --files, --group-pkgs, --groups, --groups-plist, --packages, --payload-files, --pkg-groups, --pkg-info, --pkg-info-plist, --pkgs, --pkgs-plist. - Allowed standalone flags: --check-signature, --export-plist, --file-info, --file-info-plist, --files, --group-pkgs, --groups, --groups-plist, --packages, --payload-files, --pkg-groups, --pkg-info, --pkg-info-plist, --pkgs, --pkgs-plist, --regexp, --help, -h
- Allowed valued flags: --volume

### `plutil`
<p class="cmd-url"><a href="https://ss64.com/mac/plutil.html">https://ss64.com/mac/plutil.html</a></p>

- **-convert**
- **-lint**: Flags: --help, -h, -s
- **-p**: Flags: --help, -h
- **-type**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h, -help

### `pod`
<p class="cmd-url"><a href="https://guides.cocoapods.org/terminal/commands.html">https://guides.cocoapods.org/terminal/commands.html</a></p>

- **env**: Flags: --help, -h
- **info**: Flags: --help, -h
- **init**: Flags: --help, --no-ansi, --silent, --verbose, -h
- **install**: Flags: --clean-install, --deployment, --help, --no-ansi, --no-clean, --no-integrate, --no-repo-update, --repo-update, --silent, --verbose, -h. Valued: --project-directory
- **list**: Flags: --help, -h
- **outdated**: Flags: --help, --no-ansi, --no-repo-update, --silent, --verbose, -h
- **repo list**: Flags: --count-only, --help, --no-ansi, --silent, --verbose, -h
- **repo update**: Flags: --help, --no-ansi, --silent, --verbose, -h
- **repo**: Flags: --help, -h
- **search**: Flags: --help, --simple, --stats, --web, -h
- **spec cat**: Flags: --help, -h. Valued: --version
- **spec which**: Flags: --help, -h. Valued: --version
- **update**: Flags: --clean-install, --help, --no-ansi, --no-clean, --no-integrate, --no-repo-update, --silent, --sources, --verbose, -h. Valued: --exclude-pods, --project-directory, --sources
- Allowed standalone flags: --help, --version, -V, -h

**Examples:**

- `pod --version`
- `pod env`
- `pod list`
- `pod search Alamofire`
- `pod outdated`
- `pod install`
- `pod install --repo-update --verbose`
- `pod update`
- `pod update Alamofire`
- `pod init`
- `pod repo update`
- `pod spec cat Alamofire`

### `simctl`
<p class="cmd-url"><a href="https://developer.apple.com/documentation/xcode/simctl">https://developer.apple.com/documentation/xcode/simctl</a></p>

- **addmedia**: Flags: --help, -h
- **appinfo**: Flags: --help, -h
- **boot**: Flags: --help, -h. Valued: --arch
- **clone**: Flags: --help, -h
- **create**: Flags: --help, -h
- **delete**: Flags: --help, -h
- **erase**: Flags: --help, -h
- **getenv**: Flags: --help, -h
- **icloud_sync**: Flags: --help, -h
- **install**: Flags: --help, -h
- **keyboard**: Flags: --help, -h
- **keychain**: Flags: --help, -h
- **launch**: Flags: --console, --console-pty, --help, --stdout, --stderr, --terminate-running-process, --wait-for-debugger, -h
- **list**: Flags: --help, --json, --verbose, -h, -j, -v
- **openurl**: Flags: --help, -h
- **pair**: Flags: --help, -h
- **privacy**: Flags: --help, -h
- **push**: Flags: --help, -h
- **rename**: Flags: --help, -h
- **shutdown**: Flags: --help, -h
- **status_bar**: Flags: --help, -h. Valued: --batteryLevel, --batteryState, --cellularBars, --cellularMode, --dataNetwork, --operatorName, --time, --wifiBars, --wifiMode
- **terminate**: Flags: --help, -h
- **ui**: Flags: --help, -h
- **uninstall**: Flags: --help, -h
- **unpair**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

**Examples:**

- `simctl list`
- `simctl list devices`
- `simctl list runtimes --json`
- `simctl getenv booted HOME`
- `simctl boot 'iPhone 15'`
- `simctl shutdown all`
- `simctl shutdown booted`
- `simctl erase all`
- `simctl erase 'iPhone 15'`
- `simctl install booted MyApp.app`
- `simctl uninstall booted com.example.app`
- `simctl launch booted com.example.app`
- `simctl launch --console-pty booted com.example.app`
- `simctl terminate booted com.example.app`
- `simctl openurl booted https://example.com`
- `simctl addmedia booted photo.jpg`
- `simctl push booted com.example.app push.json`
- `simctl status_bar booted clear`
- `simctl appinfo booted com.example.app`
- `simctl create 'iPhone 15 Test' 'iPhone 15'`
- `simctl clone 'iPhone 15' 'iPhone 15 Test'`
- `simctl delete unavailable`

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

- **build**: Flags: --build-output-path, --clean, --generate, --help, --no-binary-cache, --no-clean, --rosetta, --skip-signing, --verbose, -h. Valued: --configuration, --derived-data-path, --destination, --device, --os, --passthrough-xcodebuild-arguments, --path, --platform, --scheme, -c, -p
- **clean**: Flags: --help, --verbose, -h. Valued: --path, -p
- **dump**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **edit**: Flags: --help, --permanent, --verbose, -h, -P. Valued: --path, -p
- **fetch**: Flags: --help, --update, --verbose, -h. Valued: --path, -p
- **generate**: Flags: --binary-cache, --help, --json, --no-binary-cache, --no-open, --open, --verbose, -h. Valued: --configuration, --destination, --device, --os, --path, --platform, --rosetta, -c, -p
- **graph**: Flags: --help, --json, --verbose, -h. Valued: --format, --path, -f, -p
- **hash cache**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **hash selective-testing**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **inspect build**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **inspect bundle**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **inspect dependencies**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **inspect implicit-imports**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **inspect redundant-imports**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **inspect test**: Flags: --help, --json, --verbose, -h. Valued: --path, -p
- **install**: Flags: --help, --update, --verbose, -h. Valued: --path, -p
- **migration check-empty-settings**: Flags: --help, -h. Valued: --path, -p
- **migration list-targets**: Flags: --help, -h. Valued: --path, -p
- **scaffold list**: Flags: --help, --json, -h. Valued: --path, -p
- **test**: Flags: --clean, --generate, --help, --ignore-binary-cache, --no-binary-cache, --no-clean, --no-selective-testing, --no-upload-results, --rosetta, --skip-signing, --skip-ui-tests, --verbose, --without-selective-testing, -h. Valued: --configuration, --derived-data-path, --destination, --device, --os, --passthrough-xcodebuild-arguments, --path, --platform, --result-bundle-path, --retry-count, --scheme, --skip-test-targets, --test-plan, --test-targets, -c, -p
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

**Examples:**

- `tuist --version`
- `tuist version`
- `tuist generate`
- `tuist generate --no-open`
- `tuist generate --no-open -p .`
- `tuist generate App Watch`
- `tuist build`
- `tuist build MyTarget --configuration Release`
- `tuist test`
- `tuist test --skip-ui-tests`
- `tuist clean`
- `tuist clean dependencies manifests`
- `tuist install`
- `tuist install --update`
- `tuist edit`
- `tuist dump`
- `tuist graph --format json`
- `tuist inspect dependencies`

### `xcbeautify`
<p class="cmd-url"><a href="https://github.com/cpisciotta/xcbeautify">https://github.com/cpisciotta/xcbeautify</a></p>

- Allowed standalone flags: --help, --is-ci, --quiet, --quieter, --version, -V, -h, -q
- Allowed valued flags: --renderer
- Bare invocation allowed

### `xccov`
<p class="cmd-url"><a href="https://keith.github.io/xcode-man-pages/xccov.1.html">https://keith.github.io/xcode-man-pages/xccov.1.html</a></p>

- **diff**: Flags: --json, --help, -h. Valued: --path-equivalence
- **view**: Flags: --archive, --file-list, --json, --only-targets, --report, --help, -h. Valued: --file, --files-for-target, --functions-for-file

### `xcode-select`
<p class="cmd-url"><a href="https://ss64.com/mac/xcode-select.html">https://ss64.com/mac/xcode-select.html</a></p>

- Allowed standalone flags: --help, --print-path, --version, -V, -h, -p, -v

### `xcodebuild`
<p class="cmd-url"><a href="https://developer.apple.com/documentation/xcode/xcodebuild">https://developer.apple.com/documentation/xcode/xcodebuild</a></p>

- Allowed standalone flags: -alltargets, -allowProvisioningDeviceRegistration, -allowProvisioningUpdates, -checkFirstLaunchStatus, -disableAutomaticPackageResolution, -disablePackageRepositoryCache, -dry-run, -enableAddressSanitizer, -enableCodeCoverage, -enableThreadSanitizer, -enableUndefinedBehaviorSanitizer, -hideShellScriptEnvironment, -json, -license, -list, -onlyUsePackageVersionsFromResolvedFile, -parallelizeTargets, -quiet, -resolvePackageDependencies, -runFirstLaunch, -showBuildSettings, -showBuildSettingsForIndex, -showComponent, -showFirstLaunchExperience, -showPartialBuildSettings, -showTestPlans, -showdestinations, -showsdks, -skipMacroValidation, -skipPackagePluginValidation, -skipPackageSignatureValidation, -skipPackageUpdates, -skipUnavailableActions, -usePackageSupportBuiltinSCM, -verbose, -version, --help, --version, -V, -h
- Allowed valued flags: -arch, -archivePath, -buildVersion, -configuration, -defaultLanguage, -defaultPackageRegistryURL, -derivedDataPath, -destination, -destination-timeout, -exportArchive, -exportFormat, -exportLanguage, -exportLocalizations, -exportNotarizedApp, -exportOptionsPlist, -exportPath, -find-executable, -find-library, -importComponent, -importLocalizations, -importPath, -jobs, -localizationPath, -maximum-concurrent-test-device-destinations, -maximum-concurrent-test-simulator-destinations, -maximum-parallel-testing-workers, -only-testing, -only-testing:TEST-IDENTIFIER, -onlyTesting, -packageAuthorizationProvider, -packageCachePath, -packageDependencySCMToRegistryTransformation, -packageFingerprintPolicy, -packageSigningEntityPolicy, -parallel-testing-enabled, -parallel-testing-worker-count, -prepareDeviceSupport, -project, -resultBundlePath, -resultBundleVersion, -resultStreamPath, -retry-tests-on-failure, -runDestination, -runDestinationTimeout, -scheme, -scmProvider, -sdk, -skip-testing, -skip-testing:TEST-IDENTIFIER, -skipTesting, -target, -test-enumeration-format, -test-enumeration-output-path, -test-enumeration-style, -test-iterations, -test-repetition-relaunch-enabled, -test-timeouts-enabled, -testLanguage, -testPlan, -testProductsPath, -testRegion, -toolchain, -workspace, -xcconfig, -xctestrun
- Hyphen-prefixed positional arguments accepted

**Examples:**

- `xcodebuild -version`
- `xcodebuild -showsdks`
- `xcodebuild -showdestinations -project Foo.xcodeproj -scheme Foo`
- `xcodebuild -list -project Foo.xcodeproj`
- `xcodebuild -project Foo.xcodeproj -scheme Foo -list`
- `xcodebuild -project Foo.xcodeproj -scheme Foo build`
- `xcodebuild -project Foo.xcodeproj -scheme Foo -configuration Debug build`
- `xcodebuild -project Foo.xcodeproj -scheme Foo -configuration Debug -destination 'platform=macOS' build CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO CODE_SIGN_IDENTITY=""`
- `xcodebuild -workspace Foo.xcworkspace -scheme Foo -configuration Release archive -archivePath build/Foo.xcarchive`
- `xcodebuild -project Foo.xcodeproj -scheme Foo test -destination 'platform=iOS Simulator,name=iPhone 15' -enableCodeCoverage`
- `xcodebuild -project Foo.xcodeproj -scheme Foo clean`
- `xcodebuild -project Foo.xcodeproj -scheme Foo analyze`
- `xcodebuild -project Foo.xcodeproj -scheme Foo docbuild`

### `xcodegen`
<p class="cmd-url"><a href="https://github.com/yonaskolb/XcodeGen">https://github.com/yonaskolb/XcodeGen</a></p>

- **dump**: Flags: --help, --no-env, --quiet, -h, -n, -q. Valued: --project-root, --spec, --type, -r, -s, -t
- **generate**: Flags: --help, --no-env, --only-plists, --quiet, --use-cache, -c, -h, -n, -q. Valued: --cache-path, --project, --project-root, --spec, -p, -r, -s
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

**Examples:**

- `xcodegen --version`
- `xcodegen --help`
- `xcodegen generate`
- `xcodegen generate --spec project.yml`
- `xcodegen generate -s project.yml -p out/`
- `xcodegen generate --quiet --use-cache`
- `xcodegen generate --only-plists`
- `xcodegen generate --no-env`
- `xcodegen dump`
- `xcodegen dump --spec project.yml --type json`
- `xcodegen version`

### `xcresulttool`
<p class="cmd-url"><a href="https://keith.github.io/xcode-man-pages/xcresulttool.1.html">https://keith.github.io/xcode-man-pages/xcresulttool.1.html</a></p>

- **compare**: Flags: --analyzer-issues, --build-warnings, --compact, --schema, --summary, --test-failures, --tests, --help, -h. Valued: --baseline-path, --schema-version
- **formatDescription**: Flags: --hash, --include-event-stream-types, --help, -h. Valued: --format
- **get**: Flags: --compact, --schema, --help, -h. Valued: --format, --id, --path, --schema-version, --test-id, --type
- **graph**: Flags: --help, -h. Valued: --path
- **metadata get**: Flags: --help, -h. Valued: --path
- **version**: Flags: --help, -h

### `xcrun`
<p class="cmd-url"><a href="https://ss64.com/mac/xcrun.html">https://ss64.com/mac/xcrun.html</a></p>

- **--find**: Positional args accepted
- **--show-sdk-build-version**: Positional args accepted
- **--show-sdk-path**: Positional args accepted
- **--show-sdk-platform-path**: Positional args accepted
- **--show-sdk-platform-version**: Positional args accepted
- **--show-sdk-version**: Positional args accepted
- **--show-toolchain-path**: Positional args accepted
- **actool**: delegates to inner command
- **agvtool**: delegates to inner command
- **codesign**: delegates to inner command
- **ibtool**: delegates to inner command
- **lipo**: delegates to inner command
- **pkgutil**: delegates to inner command
- **plutil**: delegates to inner command
- **simctl**: delegates to inner command
- **spctl**: delegates to inner command
- **stapler**: delegates to inner command
- **swift**: delegates to inner command
- **xcodebuild**: delegates to inner command
- **xcresulttool**: delegates to inner command
- **xctest**: delegates to inner command
- **xctrace**: delegates to inner command
- Allowed standalone flags: --help, -h

### `xctest`
<p class="cmd-url"><a href="https://keith.github.io/xcode-man-pages/xctest.1.html">https://keith.github.io/xcode-man-pages/xctest.1.html</a></p>

- Allowed valued flags: -XCTest

### `xctrace`
<p class="cmd-url"><a href="https://keith.github.io/xcode-man-pages/xctrace.1.html">https://keith.github.io/xcode-man-pages/xctrace.1.html</a></p>

- **export**: Flags: --har, --quiet, --toc, --help, -h. Valued: --input, --output, --xpath
- **help**: Positional args accepted
- **import**: Flags: --quiet, --help, -h. Valued: --input, --instrument, --output, --package, --template
- **list**: Allowed arguments: devices, templates, instruments
- **symbolicate**: Flags: --quiet, --help, -h. Valued: --dsym, --input, --output
- **version**: Flags: --help, -h

