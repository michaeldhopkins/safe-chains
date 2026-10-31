//! A test runner or build tool pointed at code elsewhere runs that code: a spec file, a config file
//! that is a script, a `--require`d file, a package or project directory. Each such operand or
//! flag value is judged where the code comes from, so a temporary directory that is not this
//! session's asks, while the same spelling naming a file in the project is approved.

fn approved_in_project(command: &str) -> bool {
    use safe_chains::pathctx::{PathCtx, enter};
    let project = format!("{}/projects/foreigncodeprobe", std::env::var("HOME").expect("HOME"));
    let _ctx = enter(PathCtx { cwd: Some(project.clone()), root: Some(project), session_id: None });
    safe_chains::is_safe_command(command)
}

const CASES: &[&str] = &[
    "prettier --check --config {d}/x.js .", "prettier --check --plugin {d}/x.js .", "rollup -c {d}/x.js", "rollup --config {d}/x.js",
    "tsup --config {d}/x.ts", "mocha {d}/t.js", "ava {d}/t.js", "ava --config {d}/x.js", "pylint --rcfile {d}/rc x.py",
    "flake8 --config {d}/x.cfg", "phpunit -c {d}/x.xml", "phpunit --configuration {d}/x.xml", "phpunit --bootstrap {d}/x.php",
    "phpunit {d}/t.php", "phpstan analyse -c {d}/x.neon", "phpstan analyse --configuration {d}/x.neon", "phpstan analyse -a {d}/x.php",
    "phpstan analyse --autoload-file {d}/x.php", "rubocop -c {d}/x.yml", "rubocop --config {d}/x.yml", "rubocop -r {d}/x.rb",
    "rubocop --require {d}/x.rb", "mypy --config-file {d}/x.ini .", "sphinx-build {d}/docs out", "sphinx-build --pdb {d}/docs out",
    "sphinx-build --nitpicky {d}/docs out", "sphinx-build --isolated {d}/docs out", "sphinx-build -c {d}/conf docs out",
    "rollup -i src/main.js -c {d}/x.js", "next build {d}/web", "astro build --root {d}/site", "cucumber {d}/features",
    "cucumber -r {d}/x.rb", "cucumber --require {d}/x.rb", "rspec {d}/x_spec.rb", "rspec -r {d}/x.rb", "rspec --require {d}/x.rb",
    "pytest {d}/tests", "pytest --rootdir {d}/w", "pytest --rootdir={d}/w", "pytest -c {d}/x.ini", "swift build --package-path {d}/pkg",
    "swift test --package-path={d}/pkg", "tuist build --path {d}/app", "coverage run {d}/x.py", "coverage run -m pytest {d}/tests",
    "go test {d}/pkg/...", "bun test {d}/tests", "dotnet test {d}/app", "dotnet build {d}/app/x.csproj", "vite build {d}/site",
    "mocha --spec {d}/t.js", "rspec --default-path {d}/spec", "dotnet publish {d}/app", "dotnet pack {d}/app", "dotnet clean {d}/app",
    "dotnet restore {d}/app", "dotnet test --settings {d}/x.runsettings", "dotnet test --test-adapter-path {d}/adapters",
    "dotnet build --source {d}/feed", "bun test --preload {d}/x.ts", "coverage run --rcfile {d}/rc -m pytest",
    "go test -modfile {d}/go.mod ./...", "go test -overlay {d}/o.json ./...",
];

#[test]
fn a_tool_pointed_at_code_in_a_foreign_temp_folder_asks() {
    for case in CASES {
        for dir in ["/tmp/y", "/private/tmp/y", "/var/tmp/y"] {
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
        "coverage run --source /tmp/y -m pytest",
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
