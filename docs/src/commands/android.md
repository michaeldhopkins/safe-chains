# Android

### `aapt2`
<p class="cmd-url"><a href="https://developer.android.com/tools/aapt2">https://developer.android.com/tools/aapt2</a></p>

- **dump badging**: Flags: --help, --no-values, -h, -v. Valued: --config, --file
- **dump configurations**: Flags: --help, --no-values, -h, -v. Valued: --config, --file
- **dump permissions**: Flags: --help, --no-values, -h, -v. Valued: --config, --file
- **dump resources**: Flags: --help, --no-values, -h, -v. Valued: --config, --file
- **dump strings**: Flags: --help, --no-values, -h, -v. Valued: --config, --file
- **dump styleparents**: Flags: --help, --no-values, -h, -v. Valued: --config, --file
- **dump xmlstrings**: Flags: --help, --no-values, -h, -v. Valued: --config, --file
- **dump xmltree**: Flags: --help, --no-values, -h, -v. Valued: --config, --file
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

### `adb`
<p class="cmd-url"><a href="https://developer.android.com/tools/adb">https://developer.android.com/tools/adb</a></p>

- Bare subcommands: devices, get-serialno, get-state, help, start-server, version. forward --list, reverse --list. logcat (requires -d). shell: cat, df, dumpsys, getprop, id, ls, pm list/path, ps, settings get, uname, whoami, wm size/density. Prefix flag -s SERIAL is skipped.

### `apkanalyzer`
<p class="cmd-url"><a href="https://developer.android.com/tools/apkanalyzer">https://developer.android.com/tools/apkanalyzer</a></p>

- **apk compare**: Flags: --help, -h
- **apk download-size**: Flags: --help, -h
- **apk features**: Flags: --help, --not-required, -h
- **apk file-size**: Flags: --help, -h
- **apk summary**: Flags: --help, -h
- **dex code**: Flags: --help, -h
- **dex list**: Flags: --help, -h
- **dex packages**: Flags: --defined-only, --help, -h. Valued: --files, --proguard-folder, --proguard-mappings, --proguard-seeds, --proguard-usages
- **dex references**: Flags: --help, -h
- **files cat**: Flags: --help, -h
- **files list**: Flags: --help, -h
- **manifest application-id**: Flags: --help, -h
- **manifest debuggable**: Flags: --help, -h
- **manifest min-sdk**: Flags: --help, -h
- **manifest permissions**: Flags: --help, -h
- **manifest print**: Flags: --help, -h
- **manifest target-sdk**: Flags: --help, -h
- **manifest version-code**: Flags: --help, -h
- **manifest version-name**: Flags: --help, -h
- **resources configs**: Flags: --help, -h. Valued: --config, --name, --type
- **resources names**: Flags: --help, -h. Valued: --config, --name, --type
- **resources value**: Flags: --help, -h. Valued: --config, --name, --type
- **resources xml**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

### `apksigner`
<p class="cmd-url"><a href="https://developer.android.com/tools/apksigner">https://developer.android.com/tools/apksigner</a></p>

- **help**: Flags: --help, -h
- **verify**: Flags: --help, --print-certs, --verbose, -h, -v. Valued: --in, --max-sdk-version, --min-sdk-version
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

### `avdmanager`
<p class="cmd-url"><a href="https://developer.android.com/tools/avdmanager">https://developer.android.com/tools/avdmanager</a></p>

- **list avd**: Flags: --compact, --help, -c, -h
- **list device**: Flags: --compact, --help, -c, -h
- **list target**: Flags: --compact, --help, -c, -h
- Allowed standalone flags: --help, --version, -V, -h

### `bundletool`
<p class="cmd-url"><a href="https://developer.android.com/tools/bundletool">https://developer.android.com/tools/bundletool</a></p>

- **dump config**: Flags: --help, -h. Valued: --bundle, --module, --xpath
- **dump manifest**: Flags: --help, -h. Valued: --bundle, --module, --xpath
- **dump resources**: Flags: --help, -h. Valued: --bundle, --module, --xpath
- **get-size total**: Flags: --help, -h. Valued: --apks, --device-spec, --dimensions, --modules
- **validate**: Flags: --help, -h. Valued: --bundle
- **version**: Flags: --help, -h
- Allowed standalone flags: --help, --version, -V, -h

### `emulator`
<p class="cmd-url"><a href="https://developer.android.com/studio/run/emulator-commandline">https://developer.android.com/studio/run/emulator-commandline</a></p>

- Allowed standalone flags: --help, --version, -V, -h, -help, -list-avds, -version

### `lint`
<p class="cmd-url"><a href="https://developer.android.com/studio/write/lint">https://developer.android.com/studio/write/lint</a></p>

- Allowed standalone flags: --help, --list, --quiet, --show, --version, -V, -h
- Allowed valued flags: --check, --config, --disable, --enable
- Bare invocation allowed

### `sdkmanager`
<p class="cmd-url"><a href="https://developer.android.com/tools/sdkmanager">https://developer.android.com/tools/sdkmanager</a></p>

- Allowed standalone flags: --help, --list, --version, -V, -h
- Allowed valued flags: --channel, --sdk_root

### `zipalign`
<p class="cmd-url"><a href="https://developer.android.com/tools/zipalign">https://developer.android.com/tools/zipalign</a></p>

- Requires -c. - Allowed standalone flags: -c, -p, -v, --help, -h, --version, -V

