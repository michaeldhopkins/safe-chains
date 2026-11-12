//! A test runner or build tool pointed at code elsewhere runs that code: a spec file, a config file
//! that is a script, a `--require`d file, a package or project directory. Each such operand or
//! flag value is judged where the code comes from, so a temporary directory that is not this
//! session's asks, while the same spelling naming a file in the project is approved.

use proptest::prelude::*;

fn approved_in_project(command: &str) -> bool {
    use safe_chains::pathctx::{PathCtx, enter};
    let project = format!("{}/projects/foreigncodeprobe", std::env::var("HOME").expect("HOME"));
    let _ctx = enter(PathCtx { cwd: Some(project.clone()), root: Some(project), session_id: None });
    safe_chains::is_safe_command(command)
}

const CASES: &[&str] = &[
    "prettier --check --config {d}/x.js .",
    "prettier --check --plugin {d}/x.js .",
    "rollup -c {d}/x.js",
    "rollup --config {d}/x.js",
    "tsup --config {d}/x.ts",
    "mocha {d}/t.js",
    "ava {d}/t.js",
    "ava --config {d}/x.js",
    "pylint --rcfile {d}/rc x.py",
    "flake8 --config {d}/x.cfg",
    "phpunit -c {d}/x.xml",
    "phpunit --configuration {d}/x.xml",
    "phpunit --bootstrap {d}/x.php",
    "phpunit {d}/t.php",
    "phpstan analyse -c {d}/x.neon",
    "phpstan analyse --configuration {d}/x.neon",
    "phpstan analyse -a {d}/x.php",
    "phpstan analyse --autoload-file {d}/x.php",
    "rubocop -c {d}/x.yml",
    "rubocop --config {d}/x.yml",
    "rubocop -r {d}/x.rb",
    "rubocop --require {d}/x.rb",
    "mypy --config-file {d}/x.ini .",
    "sphinx-build {d}/docs out",
    "sphinx-build --pdb {d}/docs out",
    "sphinx-build --nitpicky {d}/docs out",
    "sphinx-build --isolated {d}/docs out",
    "sphinx-build -c {d}/conf docs out",
    "rollup -i src/main.js -c {d}/x.js",
    "next build {d}/web",
    "astro build --root {d}/site",
    "cucumber {d}/features",
    "cucumber -r {d}/x.rb",
    "cucumber --require {d}/x.rb",
    "rspec {d}/x_spec.rb",
    "rspec -r {d}/x.rb",
    "rspec --require {d}/x.rb",
    "pytest {d}/tests",
    "pytest --rootdir {d}/w",
    "pytest --rootdir={d}/w",
    "pytest -c {d}/x.ini",
    "swift build --package-path {d}/pkg",
    "swift test --package-path={d}/pkg",
    "tuist build --path {d}/app",
    "coverage run {d}/x.py",
    "coverage run -m pytest {d}/tests",
    "go test {d}/pkg/...",
    "bun test {d}/tests",
    "dotnet test {d}/app",
    "dotnet build {d}/app/x.csproj",
    "vite build {d}/site",
    "mocha --spec {d}/t.js",
    "rspec --default-path {d}/spec",
    "dotnet publish {d}/app",
    "dotnet pack {d}/app",
    "dotnet clean {d}/app",
    "dotnet restore {d}/app",
    "dotnet test --settings {d}/x.runsettings",
    "dotnet test --test-adapter-path {d}/adapters",
    "dotnet build --source {d}/feed",
    "bun test --preload {d}/x.ts",
    "coverage run --rcfile {d}/rc -m pytest",
    "go test -modfile {d}/go.mod ./...",
    "go test -overlay {d}/o.json ./...",
    // A positional test file, project directory, entry, or manifest is the code the tool runs.
    "jest {d}/evil.test.js",
    "jest --projects {d}",
    "jest --projects={d}",
    "jest --roots {d}",
    "jest --roots={d}",
    "vitest {d}/evil.test.ts",
    "vitest --dir {d}",
    "vitest --dir={d}",
    "vitest --project {d}",
    "vitest --project={d}",
    "webpack {d}/entry.js",
    "webpack --entry {d}/entry.js",
    "webpack --entry={d}/entry.js",
    "webpack --entry main={d}/entry.js",
    "webpack --entry=main={d}/entry.js",
    "tsup {d}/index.ts",
    "tsup --entry {d}/index.ts",
    "tsup --entry={d}/index.ts",
    "tsup --entry main={d}/index.ts",
    "tsup --entry=main={d}/index.ts",
    "tsup --inject {d}/shim.ts",
    "eslint --rulesdir {d}/rules src/",
    "eslint --rulesdir={d}/rules src/",
    "eslint --plugin {d}/p.js src/",
    "eslint --plugin={d}/p.js src/",
    "eslint --parser {d}/p.js src/",
    "eslint --parser={d}/p.js src/",
    "eslint --resolve-plugins-relative-to {d} src/",
    "eslint --resolve-plugins-relative-to={d} src/",
    "stylelint --custom-syntax {d}/syn.js a.css",
    "stylelint --custom-syntax={d}/syn.js a.css",
    "stylelint --custom-formatter {d}/fmt.js a.css",
    "stylelint --custom-formatter={d}/fmt.js a.css",
    "stylelint --config-basedir {d} a.css",
    "stylelint --config-basedir={d} a.css",
    "deno test {d}/evil_test.ts",
    "deno test -c {d}/deno.json",
    "deno test -c={d}/deno.json",
    "deno test --config {d}/deno.json",
    "deno test --config={d}/deno.json",
    "deno test --import-map {d}/map.json",
    "deno test --import-map={d}/map.json",
    "cargo msrv find --path {d}",
    "cargo msrv find --path={d}",
    "bundle check --gemfile {d}/Gemfile",
    "bundle check --gemfile={d}/Gemfile",
    "bundle install --gemfile {d}/Gemfile",
    "bundle install --gemfile={d}/Gemfile",
    "xcodebuild -project {d}/Evil.xcodeproj build",
    "xcodebuild -project={d}/Evil.xcodeproj build",
    "xcodebuild -workspace {d}/Evil.xcworkspace build",
    "xcodebuild -workspace={d}/Evil.xcworkspace build",
    "xcodebuild -xctestrun {d}/Evil.xctestrun test-without-building",
    "xcodebuild -xctestrun={d}/Evil.xctestrun test-without-building",
    "xcodebuild -testPlan {d}/Plan.xctestplan test",
    "xcodebuild -testPlan={d}/Plan.xctestplan test",
    "make -n -C {d}",
    "make -n -C={d}",
    "make -n --directory {d}",
    "make -n --directory={d}",
    "make -n -f {d}/Makefile",
    "make -n -f={d}/Makefile",
    "make -n --file {d}/Makefile",
    "make -n --file={d}/Makefile",
    "make -n --makefile {d}/Makefile",
    "make -n --makefile={d}/Makefile",
    "mvn test -f {d}/pom.xml",
    "mvn test -f={d}/pom.xml",
    "mvn test --file {d}/pom.xml",
    "mvn test --file={d}/pom.xml",
    "mvn test -s {d}/settings.xml",
    "mvn test -s={d}/settings.xml",
    "php {d}/artisan test",
    "jasmine {d}/evil.spec.js",
    "jasmine --config {d}/jasmine.json",
    "jasmine --require {d}/helper.js",
    "jasmine --reporter {d}/rep.js",
    "cucumber-js {d}/evil.feature",
    "cucumber-js --config {d}/cucumber.js",
    "cucumber-js --require {d}/steps.js",
    "cucumber-js --require-module {d}/mod.js",
    "cucumber-js --paths {d}",
    "gotestsum --packages {d}",
    "npm ci --ignore-scripts --prefix {d}",
    "npm ci --ignore-scripts --prefix={d}",
    "make -n -j -f {d}/Makefile",
    "make -n -j --directory={d}",
    "make -n --jobs -C{d}",
    "make -nj -f {d}/Makefile",
    "rspec --bisect {d}/x_spec.rb",
    "rspec --fail-fast {d}/x_spec.rb",
    "rspec -p -r{d}/e.rb",
    "rspec --profile --require {d}/e.rb",
    "gotestsum --rerun-fails --packages {d}",
];

#[test]
fn a_flag_without_a_value_does_not_swallow_the_flag_naming_the_code() {
    let home = std::env::var("HOME").expect("HOME");
    for command in [
        format!("rspec --fail-fast {home}/Downloads/evil_spec.rb"),
        format!("rspec --bisect {home}/Downloads/evil_spec.rb"),
        format!("make -n -j -f {home}/Downloads/Makefile"),
    ] {
        assert!(!approved_in_project(&command), "{command}");
    }
    for command in [
        "make -n -j 4",
        "make -n -j4",
        "make -n --jobs=4 -f Makefile",
        "rspec --bisect spec/x_spec.rb",
        "rspec --fail-fast spec/x_spec.rb",
        "rspec -p spec/x_spec.rb",
        "gotestsum --rerun-fails --rerun-fails-report ./rerun.txt",
    ] {
        assert!(approved_in_project(command), "{command}");
    }
}

#[test]
fn a_named_entry_is_the_path_after_the_equals_when_that_comes_first() {
    assert!(!approved_in_project("webpack --entry /tmp/foo=bar/ok.js"), "a slash before = is the path itself");
    assert!(!approved_in_project("tsup --entry /tmp/foo=bar/index.ts"), "a slash before = is the path itself");
    assert!(!approved_in_project("webpack --entry main="));
    assert!(!approved_in_project("tsup --entry name="));
    assert!(!approved_in_project("webpack --entry main=*.js"));
    assert!(approved_in_project("webpack --entry name=src/index.js"));
    assert!(approved_in_project("tsup --entry name=src/index.ts"));
}

#[test]
fn npm_prefix_outside_the_project_asks() {
    assert!(!approved_in_project("npm ci --ignore-scripts --prefix /etc"));
    assert!(!approved_in_project("npm ls --prefix /etc"));
    assert!(approved_in_project("npm ci --ignore-scripts --prefix ./vendor"));
    assert!(approved_in_project("npm ci --ignore-scripts"));
}

#[test]
fn a_tool_pointed_at_code_in_a_foreign_temp_folder_asks() {
    for case in CASES {
        for dir in ["/tmp/y", "/private/tmp/y", "/var/tmp/y", "/TMP/y", "/System/Volumes/Data/tmp/y", "/private/var/folders/xx/yy/T/y"] {
            let command = case.replace("{d}", dir);
            assert!(!approved_in_project(&command), "{command}");
        }
    }
}

#[test]
fn the_same_tool_pointed_at_code_in_the_project_is_approved() {
    for case in CASES {
        let command = case.replace("{d}", "./sub");
        assert!(approved_in_project(&command), "{command}");
    }
}

#[test]
fn pytest_cannot_add_a_foreign_folder_to_its_import_path() {
    for command in [
        "pytest -o pythonpath=/tmp/y -p evil",
        "pytest --override-ini=pythonpath=/tmp/y -p evil",
        "pytest --override-ini pythonpath=/tmp/y",
    ] {
        assert!(!approved_in_project(command), "{command}");
    }
}

#[test]
fn a_test_runner_cannot_pass_its_interpreter_options_that_load_code() {
    for command in [
        "mocha -n require=/tmp/y/x.js",
        "mocha --node-option require=/tmp/y/x.js",
        "mocha --node-option=import=/tmp/y/x.mjs",
        "ava --node-arguments=--require=/tmp/y/x.js",
        "phpunit -d auto_prepend_file=/tmp/y/x.php --process-isolation",
        "dotnet build -p CustomBeforeMicrosoftCommonTargets=/tmp/y/x.targets",
        "dotnet build --property CustomBeforeMicrosoftCommonTargets=/tmp/y/x.targets",
        "dotnet publish -p CustomBeforeMicrosoftCommonTargets=/tmp/y/x.targets",
        "dotnet pack -p CustomBeforeMicrosoftCommonTargets=/tmp/y/x.targets",
        "dotnet test -p CustomBeforeMicrosoftCommonTargets=/tmp/y/x.targets",
        "dotnet build /p:CustomBeforeMicrosoftCommonTargets=/tmp/y/x.targets",
        "dotnet test -e DOTNET_STARTUP_HOOKS=/tmp/y/x.dll",
        "dotnet test --environment DOTNET_STARTUP_HOOKS=/tmp/y/x.dll",
    ] {
        assert!(!approved_in_project(command), "{command}");
    }
}

#[test]
fn a_valued_flag_that_is_not_a_path_still_takes_any_value() {
    for command in [
        "rspec --tag ~slow spec",
        "rspec --pattern /tmp/y/x spec",
        "pytest -k /tmp/y tests",
        "pytest --basetemp ./tmp tests",
        "pytest -m slow tests",
        "phpunit --filter /tmp/y tests",
        "mocha --grep /tmp/y test",
        "cucumber --tags @wip features",
        "go test -run /tmp/y ./...",
        "go test -count 1 ./...",
        "sphinx-build -b html docs out",
        "rollup -c",
        "rollup -c -w",
        "rollup -i src/main.js -o dist/out.js -f es -p node-resolve",
        "coverage run --source=/tmp/y -m pytest",
        "dotnet build -c Release",
        "dotnet test --filter Foo --logger trx",
        "dotnet test --results-directory ./out",
        "sphinx-build -d ./build/doctrees docs out",
        "phpunit --log-junit ./junit.xml tests",
        "pytest --junitxml ./junit.xml tests",
    ] {
        assert!(approved_in_project(command), "{command}");
    }
}

#[test]
fn a_wrapper_script_in_a_foreign_temp_folder_asks() {
    for dir in ["/tmp/y", "/TMP/y", "/System/Volumes/Data/tmp/y", "/private/var/folders/xx/yy/T/y"] {
        for command in [format!("cd {dir} && mvnw test"), format!("cd {dir} && mvn test")] {
            assert!(!approved_in_project(&command), "{command}");
        }
    }
}

#[test]
fn ordinary_in_project_tool_invocations_stay_approved() {
    for command in [
        "cargo build",
        "cargo test",
        "cargo msrv find --path .",
        "npm test",
        "npm run test",
        "npm run test:unit",
        "npm run-script test",
        "npm run test extra",
        "npm run test -- --watch",
        "npm run test --silent",
        "npm run test --if-present",
        "yarn test",
        "yarn test:unit",
        "yarn test extra",
        "yarn test -- --watch",
        "yarn test --silent",
        "yarn jest spec/date_range_filter_controller.test.js",
        "yarn karma start --single-run",
        "yarn vitest run",
        "npm_config_prefix=. npm run test",
        "npm_config_prefix= npm run test",
        "rollup -i src/main.js -o dist/out.js -f es -p node-resolve",
        "rollup -i src/index.js --plugin node-resolve",
        "rollup -i src/index.js --plugin @rollup/plugin-node-resolve",
        "rollup -i src/index.js -p ./plugin.js",
        "rollup -i src/index.js --plugin=./plugin.js",
        "rollup -i src/index.js --plugin ./plugin.js",
        "pytest",
        "rustup show",
        "jest",
        "jest src/foo.test.js",
        "jest --testPathPattern foo",
        "jest --collectCoverageFrom '**/*.{js,ts}'",
        "jest --projects src",
        "vitest",
        "vitest src/foo.test.ts",
        "vitest --dir src",
        "webpack",
        "webpack src/index.js",
        "webpack --entry src/index.js",
        "webpack --entry name=src/index.js",
        "webpack -o dist",
        "tsup src/index.ts",
        "tsup --entry name=src/index.ts",
        "tsup --out-dir dist src/index.ts",
        "eslint src/",
        "eslint --max-warnings 0 src/",
        "stylelint a.css",
        "deno test",
        "deno test src/app_test.ts",
        "deno fmt --check src/main.ts",
        "bundle check",
        "bundle install",
        "bundle install --gemfile Gemfile",
        "xcodebuild -project Foo.xcodeproj -scheme Foo build",
        "make -n",
        "make -n -j 4",
        "make -n -C .",
        "make -n -f Makefile",
        "mvn test",
        "mvnw test",
        "mvn test -f pom.xml",
        "php artisan test",
        "php ./artisan test",
    ] {
        assert!(approved_in_project(command), "{command}");
    }
}

#[test]
fn a_symlinked_temp_directory_is_foreign_code() {
    let proj = std::env::temp_dir().join(format!("sc-fix-02413-proj-{}", std::process::id()));
    std::fs::create_dir_all(&proj).expect("project dir");
    let _proj = RemoveDir(proj.clone());
    let root = proj.canonicalize().expect("project path").to_string_lossy().into_owned();
    let target = format!("/tmp/sc-fix-02413-{}", std::process::id());
    std::fs::create_dir_all(&target).expect("temp target");
    let _target = RemoveDir(std::path::PathBuf::from(&target));
    std::os::unix::fs::symlink(&target, format!("{root}/tmp_link")).expect("tmp_link");
    let linked = format!("XDG_DATA_HOME={root}/tmp_link pandoc README.md");
    let _ctx = safe_chains::pathctx::enter(safe_chains::pathctx::PathCtx { cwd: Some(root.clone()), root: Some(root), session_id: None });
    for command in [
        "jest tmp_link/evil.test.js",
        "vitest tmp_link/evil.test.ts",
        "webpack tmp_link/entry.js",
        "tsup tmp_link/index.ts",
        "eslint --rulesdir tmp_link/rules src/",
        "deno test tmp_link/evil_test.ts",
        "cargo install --path tmp_link",
        "make -n -C tmp_link",
        "php tmp_link/artisan test",
        "cd tmp_link && mvnw test",
        "npm run test --prefix tmp_link",
        "npm run test --prefix=tmp_link",
        "yarn test --cwd tmp_link",
        "yarn test --cwd=tmp_link",
        "npm_config_prefix=tmp_link npm run test",
        "rollup -i src/index.js --plugin tmp_link/plugin.js",
        "rollup -i src/index.js -p tmp_link/plugin.js",
        "rollup -i src/index.js --plugin=tmp_link/plugin.js",
        "CLANG_PATH_x86_64_unknown_linux_gnu=tmp_link/clang cargo build",
        "PROTOC_x86_64_unknown_linux_gnu=tmp_link/protoc cargo build",
        "LD_x86_64_unknown_linux_gnu=tmp_link/ld cargo build",
        "CPPFLAGS_x86_64_unknown_linux_gnu='-I tmp_link/include' cargo build",
        "LDFLAGS_x86_64_unknown_linux_gnu='-L tmp_link/lib' cargo build",
        "LDFLAGS_x86_64_unknown_linux_gnu=-fuse-ld=tmp_link/ld cargo build",
        "XDG_DATA_HOME=tmp_link pandoc README.md",
    ] {
        assert!(!safe_chains::is_safe_command(command), "{command}");
    }
    assert!(!safe_chains::is_safe_command(&linked), "{linked}");
    assert!(safe_chains::is_safe_command("cargo build"));
    assert!(safe_chains::is_safe_command("jest"));
    assert!(safe_chains::is_safe_command("npm run test"));
    assert!(safe_chains::is_safe_command("yarn test"));
}

/// `npm run` and `yarn <script>` admit the script name and used to ignore every flag after it.
/// `--script-shell` chooses the shell, and `--prefix` / `--cwd` choose the package whose scripts run.
const NPM_SCRIPT_FLAGS: &[&str] = &[
    "--foreground-scripts", "--help", "--if-present", "--ignore-scripts", "--include-workspace-root", "--json", "--silent", "--workspaces",
    "-h", "-s",
];
const YARN_SCRIPT_FLAGS: &[&str] = &[
    "--frozen-lockfile", "--help", "--ignore-scripts", "--json", "--no-progress", "--non-interactive", "--offline", "--prefer-offline",
    "--prod", "--production", "--silent", "-h", "-s",
];

#[test]
fn a_script_runner_flag_that_retargets_the_package_asks() {
    for command in [
        "npm run test --script-shell=/tmp/y/evil.sh",
        "npm run test --script-shell /tmp/y/evil.sh",
        "npm run test --script-shell=/TMP/y/evil.sh",
        "npm run test --script-shell=/private/tmp/y/evil.sh",
        "npm run test --script-shell=/System/Volumes/Data/tmp/y/evil.sh",
        "npm run test --script-shell=/private/var/folders/xx/yy/T/y/evil.sh",
        "npm run test --prefix /tmp/y",
        "npm run test --prefix=/tmp/y",
        "npm run test --prefix /TMP/y",
        "npm run test --prefix /private/tmp/y",
        "npm run test --prefix /System/Volumes/Data/tmp/y",
        "npm run test --prefix /private/var/folders/xx/yy/T/y",
        "npm run-script test --script-shell=/tmp/y/evil.sh",
        "npm run-script test --prefix /tmp/y",
        "npm run-script test --prefix=/tmp/y",
        "npm run --prefix /tmp/y test",
        "npm run --prefix=/tmp/y test",
        "npm --prefix /tmp/y run test",
        "npm --prefix=/tmp/y run test",
        "npm test --prefix /tmp/y",
        "npm test --script-shell=/tmp/y/evil.sh",
        "npm run test --watch",
        "yarn test --cwd /tmp/y",
        "yarn test --cwd=/tmp/y",
        "yarn test --cwd /TMP/y",
        "yarn test --cwd /private/tmp/y",
        "yarn test --cwd /System/Volumes/Data/tmp/y",
        "yarn test --cwd /private/var/folders/xx/yy/T/y",
        "yarn test:unit --cwd /tmp/y",
        "yarn test:unit --cwd=/tmp/y",
        "yarn --cwd /tmp/y test",
        "yarn --cwd=/tmp/y test",
        "yarn test --watch",
        "npm_config_script_shell=/tmp/y/evil.sh npm run test",
        "NPM_CONFIG_SCRIPT_SHELL=/tmp/y/evil.sh npm run test",
        "export npm_config_script_shell=/tmp/y/evil.sh && npm run test",
        "env npm_config_script_shell=/tmp/y/evil.sh npm run test",
        "npm_config_prefix=/tmp/y npm run test",
        "NPM_CONFIG_PREFIX=/tmp/y npm run test",
        "npm_config_prefix=/TMP/y npm test",
        "npm_config_prefix=/private/tmp/y npm run test",
        "npm_config_prefix=/System/Volumes/Data/tmp/y npm run test",
        "npm_config_prefix=/private/var/folders/xx/yy/T/y npm run test",
        "export npm_config_prefix=/tmp/y && npm run test",
        "env npm_config_prefix=/tmp/y npm run test",
    ] {
        assert!(!approved_in_project(command), "{command}");
    }
}

proptest! {
    /// A flag the script family did not declare is not ignored, in either spelling.
    #[test]
    fn an_undeclared_script_runner_flag_is_not_ignored(flag in "--[a-z]{3,12}") {
        if !NPM_SCRIPT_FLAGS.contains(&flag.as_str()) {
            let spaced = format!("npm run test {flag} /tmp/y");
            let glued = format!("npm run test {flag}=/tmp/y");
            prop_assert!(!approved_in_project(&spaced), "{}", spaced);
            prop_assert!(!approved_in_project(&glued), "{}", glued);
        }
        if !YARN_SCRIPT_FLAGS.contains(&flag.as_str()) {
            let spaced = format!("yarn test {flag} /tmp/y");
            let glued = format!("yarn test {flag}=/tmp/y");
            prop_assert!(!approved_in_project(&spaced), "{}", spaced);
            prop_assert!(!approved_in_project(&glued), "{}", glued);
        }
    }
}

/// Rollup evaluates a `--plugin` / `-p` value that contains `{`, `=`, or `,` as JavaScript.
/// A package name or a file in the project does not, and stays approved.
#[test]
fn an_inline_rollup_plugin_asks() {
    for command in [
        r#"rollup -i src/index.js --plugin="{buildStart(){require(\"child_process\").execSync(\"id\")}}""#,
        r#"rollup -i src/index.js --plugin "{buildStart(){require(\"child_process\").execSync(\"id\")}}""#,
        r#"rollup -i src/index.js -p "{buildStart(){require(\"child_process\").execSync(\"id\")}}""#,
        r#"rollup -i src/index.js -p="{buildStart(){require(\"child_process\").execSync(\"id\")}}""#,
        r#"rollup -i src/index.js --plugin "alias={buildStart(){require(\"child_process\").execSync(\"id\")}}""#,
        r#"rollup -i src/index.js --plugin="alias={buildStart(){}}""#,
        "rollup -i src/index.js --plugin '{a(){},b(){}}'",
        "rollup -i src/index.js -p '{a(){},b(){}}'",
        "rollup -i src/index.js --plugin /tmp/y/plugin.js",
        "rollup -i src/index.js --plugin=/tmp/y/plugin.js",
        "rollup -i src/index.js -p /tmp/y/plugin.js",
        "rollup -i src/index.js -p=/tmp/y/plugin.js",
        "rollup -i src/index.js --plugin /TMP/y/plugin.js",
        "rollup -i src/index.js --plugin /private/tmp/y/plugin.js",
        "rollup -i src/index.js --plugin /System/Volumes/Data/tmp/y/plugin.js",
        "rollup -i src/index.js --plugin /private/var/folders/xx/yy/T/y/plugin.js",
    ] {
        assert!(!approved_in_project(command), "{command}");
    }
}

proptest! {
    /// An inline plugin expression is refused in either spelling. A package-shaped name is not.
    #[test]
    fn an_inline_rollup_plugin_expression_is_not_a_project_file(name in "[a-z][a-z0-9-]{0,16}", body in "[A-Za-z]{0,8}") {
        let inline = format!("rollup -i src/index.js --plugin '{{build{body}(){{}}}}'");
        let glued = format!("rollup -i src/index.js --plugin={{build{body}(){{}}}}");
        let short = format!("rollup -i src/index.js -p '{{build{body}(){{}}}}'");
        let with_eq = format!("rollup -i src/index.js --plugin {name}={{build(){{}}}}");
        let with_comma = format!("rollup -i src/index.js -p '{name},{name}'");
        prop_assert!(!approved_in_project(&inline), "{}", inline);
        prop_assert!(!approved_in_project(&glued), "{}", glued);
        prop_assert!(!approved_in_project(&short), "{}", short);
        prop_assert!(!approved_in_project(&with_eq), "{}", with_eq);
        prop_assert!(!approved_in_project(&with_comma), "{}", with_comma);
        let package = format!("rollup -i src/index.js -p {name}");
        let scoped = format!("rollup -i src/index.js --plugin @{name}/plugin-{name}");
        prop_assert!(approved_in_project(&package), "{}", package);
        prop_assert!(approved_in_project(&scoped), "{}", scoped);
    }
}

struct RemoveDir(std::path::PathBuf);

impl Drop for RemoveDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn rollup_watch_mode_cannot_name_a_command_to_run() {
    for hook in ["onStart", "onBundleStart", "onBundleEnd", "onEnd", "onError"] {
        for command in [
            format!("rollup -w --watch.{hook} 'touch out.txt' -i src/main.js -o dist/out.js"),
            format!("rollup -w --watch.{hook}='touch out.txt' -i src/main.js -o dist/out.js"),
        ] {
            assert!(!approved_in_project(&command), "{command}");
        }
    }
}

/// A gated flag followed by an option-like word takes no value, as the policy reads it, so the
/// gate does not judge that word as the flag's path. The command as a whole must still ask: the
/// policy refuses a valued flag left without its value. Found by the gate_prefilter fuzz target.
#[test]
fn a_gated_flag_followed_by_an_option_like_word_is_never_approved() {
    for command in [
        "borg --rsh '--output-dir=/**/******./..' list",
        "cargo build --target-dir '--output-dir=/**/******./..'",
        "cargo build --target-dir --x=/etc",
        "asciidoctor -o --x=/etc a.adoc",
        "rsync -e --x=/etc a b",
    ] {
        assert!(!approved_in_project(command), "{command}");
    }
}
