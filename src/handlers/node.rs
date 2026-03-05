use crate::parse::{FlagCheck, Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy};

static NPM_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--json", "--link", "--long", "--omit",
        "--parseable", "--production", "--unicode",
    ]),
    standalone_short: b"al",
    valued: WordSet::new(&["--depth", "--prefix"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static NPM_VIEW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static NPM_AUDIT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--json", "--omit", "--production",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&["--audit-level"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static NPM_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static NPM_TEST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_npm(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    match tokens[1].as_str() {
        "list" | "ls" => policy::check(&tokens[1..], &NPM_LIST_POLICY),
        "view" | "info" => policy::check(&tokens[1..], &NPM_VIEW_POLICY),
        "audit" => policy::check(&tokens[1..], &NPM_AUDIT_POLICY),
        "test" => policy::check(&tokens[1..], &NPM_TEST_POLICY),
        "doctor" | "explain" | "fund" | "outdated" | "prefix"
        | "root" | "why" => policy::check(&tokens[1..], &NPM_BARE_POLICY),
        "config" => tokens.get(2).is_some_and(|a| a == "list" || a == "get"),
        "run" | "run-script" => tokens
            .get(2)
            .is_some_and(|a| a == "test" || a.starts_with("test:")),
        _ => false,
    }
}

static YARN_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json", "--long", "--production"]),
    standalone_short: b"",
    valued: WordSet::new(&["--depth", "--pattern"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static YARN_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_yarn(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    match tokens[1].as_str() {
        "list" | "ls" => policy::check(&tokens[1..], &YARN_LIST_POLICY),
        "info" | "why" => policy::check(&tokens[1..], &YARN_BARE_POLICY),
        "test" => true,
        _ => tokens[1].starts_with("test:"),
    }
}

static PNPM_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--dev", "--json", "--long", "--no-optional",
        "--parseable", "--production", "--recursive",
    ]),
    standalone_short: b"Pr",
    valued: WordSet::new(&["--depth", "--filter"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static PNPM_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json", "--recursive"]),
    standalone_short: b"r",
    valued: WordSet::new(&["--filter"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_pnpm(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    match tokens[1].as_str() {
        "list" | "ls" => policy::check(&tokens[1..], &PNPM_LIST_POLICY),
        "audit" | "outdated" | "why" => policy::check(&tokens[1..], &PNPM_BARE_POLICY),
        _ => false,
    }
}

static BUN_TEST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--bail", "--only", "--rerun-each", "--todo"]),
    standalone_short: b"",
    valued: WordSet::new(&["--preload", "--timeout"]),
    valued_short: b"t",
    bare: true,
    max_positional: None,
};

static BUN_OUTDATED_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static BUN_PM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

static NPX_SAFE: WordSet =
    WordSet::new(&["@herb-tools/linter", "eslint", "karma"]);

static NPX_FLAGS_NO_ARG: WordSet =
    WordSet::new(&["--ignore-existing", "--no", "--quiet", "--yes", "-q", "-y"]);

static BUNX_FLAGS_NO_ARG: WordSet =
    WordSet::new(&["--bun", "--no-install", "--silent", "--verbose"]);

static TSC_CHECK: FlagCheck =
    FlagCheck::new(&["--noEmit"], &[]);

fn find_runner_package_index(
    tokens: &[Token],
    start: usize,
    flags: &WordSet,
) -> Option<usize> {
    let mut i = start;
    while i < tokens.len() {
        if tokens[i] == "--package" || tokens[i] == "-p" {
            i += 2;
            continue;
        }
        if flags.contains(&tokens[i]) {
            i += 1;
            continue;
        }
        if tokens[i] == "--" {
            return Some(i + 1);
        }
        if tokens[i].starts_with("-") {
            return None;
        }
        return Some(i);
    }
    None
}

fn is_safe_runner_package(tokens: &[Token], pkg_idx: usize) -> bool {
    if pkg_idx >= tokens.len() {
        return false;
    }
    if NPX_SAFE.contains(&tokens[pkg_idx]) {
        return true;
    }
    if tokens[pkg_idx] == "tsc" {
        return TSC_CHECK.is_safe(&tokens[pkg_idx + 1..]);
    }
    false
}

pub fn is_safe_npx(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if tokens.len() == 2 && tokens[1] == "--version" {
        return true;
    }
    find_runner_package_index(tokens, 1, &NPX_FLAGS_NO_ARG)
        .is_some_and(|idx| is_safe_runner_package(tokens, idx))
}

pub fn is_safe_bunx(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if tokens.len() == 2 && tokens[1] == "--version" {
        return true;
    }
    find_runner_package_index(tokens, 1, &BUNX_FLAGS_NO_ARG)
        .is_some_and(|idx| is_safe_runner_package(tokens, idx))
}

pub fn is_safe_bun(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if tokens[1] == "x" {
        return find_runner_package_index(tokens, 2, &BUNX_FLAGS_NO_ARG)
            .is_some_and(|idx| is_safe_runner_package(tokens, idx));
    }
    match tokens[1].as_str() {
        "test" => policy::check(&tokens[1..], &BUN_TEST_POLICY),
        "outdated" => policy::check(&tokens[1..], &BUN_OUTDATED_POLICY),
        "pm" => {
            if tokens.len() < 3 {
                return false;
            }
            static BUN_PM_SAFE: WordSet = WordSet::new(&["bin", "cache", "hash", "ls"]);
            if !BUN_PM_SAFE.contains(&tokens[2]) {
                return false;
            }
            policy::check(&tokens[2..], &BUN_PM_POLICY)
        }
        _ => false,
    }
}

static DENO_SAFE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--json", "--no-lock", "--quiet", "--unstable",
    ]),
    standalone_short: b"q",
    valued: WordSet::new(&["--config", "--import-map"]),
    valued_short: b"c",
    bare: true,
    max_positional: None,
};

static DENO_FMT: FlagCheck =
    FlagCheck::new(&["--check"], &[]);

pub fn is_safe_deno(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    match tokens[1].as_str() {
        "check" | "doc" | "info" | "lint" | "test" => {
            policy::check(&tokens[1..], &DENO_SAFE_POLICY)
        }
        "fmt" => DENO_FMT.is_safe(&tokens[2..]),
        _ => false,
    }
}

static NVM_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--lts", "--no-colors"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_nvm(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    static NVM_SAFE: WordSet =
        WordSet::new(&["current", "list", "ls", "ls-remote", "version", "which"]);
    if !NVM_SAFE.contains(&tokens[1]) {
        return false;
    }
    policy::check(&tokens[1..], &NVM_BARE_POLICY)
}

static FNM_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_fnm(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    static FNM_SAFE: WordSet =
        WordSet::new(&["current", "default", "list", "ls-remote"]);
    if !FNM_SAFE.contains(&tokens[1]) {
        return false;
    }
    policy::check(&tokens[1..], &FNM_BARE_POLICY)
}

static VOLTA_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--current", "--default"]),
    standalone_short: b"cd",
    valued: WordSet::new(&["--format"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
};

pub fn is_safe_volta(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    static VOLTA_SAFE: WordSet = WordSet::new(&["list", "which"]);
    if !VOLTA_SAFE.contains(&tokens[1]) {
        return false;
    }
    policy::check(&tokens[1..], &VOLTA_BARE_POLICY)
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "npm" => Some(is_safe_npm(tokens)),
        "yarn" => Some(is_safe_yarn(tokens)),
        "pnpm" => Some(is_safe_pnpm(tokens)),
        "bun" => Some(is_safe_bun(tokens)),
        "deno" => Some(is_safe_deno(tokens)),
        "npx" => Some(is_safe_npx(tokens)),
        "bunx" => Some(is_safe_bunx(tokens)),
        "nvm" => Some(is_safe_nvm(tokens)),
        "fnm" => Some(is_safe_fnm(tokens)),
        "volta" => Some(is_safe_volta(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocBuilder, wordset_items};
    vec![
        CommandDoc::handler("npm",
            "Subcommands: audit, config (list/get), doctor, explain, fund, info, list, ls, \
             outdated, prefix, root, run/run-script (test only), test, view, why. \
             Each has an explicit flag allowlist."),
        CommandDoc::handler("yarn",
            "Subcommands: info, list, ls, test, test:*, why. \
             Each has an explicit flag allowlist."),
        CommandDoc::handler("pnpm",
            "Subcommands: audit, list, ls, outdated, why. \
             Each has an explicit flag allowlist."),
        CommandDoc::handler("bun",
            "Subcommands: outdated, pm (bin/cache/hash/ls), test. \
             x delegates to bunx logic. Each has an explicit flag allowlist."),
        CommandDoc::handler("bunx",
            DocBuilder::new()
                .section(format!("Allowed packages: {}.", wordset_items(&NPX_SAFE)))
                .section("Guarded: tsc (requires --noEmit).")
                .section("Skips flags: --bun/--no-install/--package/-p.")
                .build()),
        CommandDoc::handler("deno",
            "Subcommands: check, doc, info, lint, test. \
             Guarded: fmt (requires --check). Each has an explicit flag allowlist."),
        CommandDoc::handler("npx",
            DocBuilder::new()
                .section(format!("Allowed packages: {}.", wordset_items(&NPX_SAFE)))
                .section("Guarded: tsc (requires --noEmit).")
                .section("Skips flags: --yes/-y/--no/--package/-p.")
                .build()),
        CommandDoc::handler("nvm",
            "Subcommands: current, list, ls, ls-remote, version, which. \
             Minimal flags allowed."),
        CommandDoc::handler("fnm",
            "Subcommands: current, default, list, ls-remote. No extra flags allowed."),
        CommandDoc::handler("volta",
            "Subcommands: list, which. Flags: --current, --default, --format."),
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        yarn_list: "yarn list --depth=0",
        yarn_list_json: "yarn list --json",
        yarn_ls: "yarn ls bootstrap",
        yarn_info: "yarn info react",
        yarn_info_json: "yarn info react --json",
        yarn_why: "yarn why lodash",
        yarn_version: "yarn --version",
        yarn_test: "yarn test",
        yarn_test_watch: "yarn test:watch",
        yarn_test_with_args: "yarn test --testPathPattern=Foo",
        npm_view: "npm view react version",
        npm_view_json: "npm view react --json",
        npm_info: "npm info lodash",
        npm_list: "npm list --depth=0",
        npm_list_json: "npm list --json",
        npm_list_all: "npm list --all",
        npm_ls: "npm ls",
        npm_test: "npm test",
        npm_audit: "npm audit",
        npm_audit_json: "npm audit --json",
        npm_outdated: "npm outdated",
        npm_explain: "npm explain lodash",
        npm_why: "npm why lodash",
        npm_fund: "npm fund",
        npm_prefix: "npm prefix",
        npm_root: "npm root",
        npm_doctor: "npm doctor",
        npm_config_list: "npm config list",
        npm_config_get: "npm config get registry",
        npm_run_test: "npm run test",
        npm_run_test_colon: "npm run test:unit",
        npm_version: "npm --version",
        npx_herb_linter: "npx @herb-tools/linter app/views/foo.html.erb",
        npx_eslint: "npx eslint src/",
        npx_karma: "npx karma start",
        npx_yes_flag: "npx --yes eslint src/",
        npx_y_flag: "npx -y @herb-tools/linter .",
        npx_package_flag: "npx --package @herb-tools/linter @herb-tools/linter .",
        npx_double_dash: "npx -- eslint src/",
        npx_version: "npx --version",
        npx_tsc_noemit: "npx tsc --noEmit",
        pnpm_list: "pnpm list",
        pnpm_list_json: "pnpm list --json",
        pnpm_list_depth: "pnpm list --depth 0",
        pnpm_why: "pnpm why lodash",
        pnpm_audit: "pnpm audit",
        pnpm_outdated: "pnpm outdated",
        pnpm_version: "pnpm --version",
        bun_version: "bun --version",
        bun_test: "bun test",
        bun_test_bail: "bun test --bail",
        bun_test_timeout: "bun test --timeout 5000",
        bun_pm_ls: "bun pm ls",
        bun_pm_hash: "bun pm hash",
        bun_pm_cache: "bun pm cache",
        bun_pm_bin: "bun pm bin",
        bun_outdated: "bun outdated",
        bun_x_eslint: "bun x eslint src/",
        bun_x_tsc_noemit: "bun x tsc --noEmit",
        deno_version: "deno --version",
        deno_info: "deno info",
        deno_info_json: "deno info --json",
        deno_doc: "deno doc mod.ts",
        deno_lint: "deno lint",
        deno_check: "deno check main.ts",
        deno_test: "deno test",
        deno_test_quiet: "deno test --quiet",
        deno_fmt_check: "deno fmt --check",
        nvm_ls: "nvm ls",
        nvm_list: "nvm list",
        nvm_current: "nvm current",
        nvm_which: "nvm which 18",
        nvm_version: "nvm version",
        nvm_ls_remote: "nvm ls-remote",
        nvm_ls_remote_lts: "nvm ls-remote --lts",
        fnm_list: "fnm list",
        fnm_current: "fnm current",
        fnm_default: "fnm default",
        fnm_version: "fnm --version",
        volta_list: "volta list",
        volta_list_current: "volta list --current",
        volta_which: "volta which node",
        volta_version: "volta --version",
        bunx_eslint: "bunx eslint src/",
        bunx_tsc_noemit: "bunx tsc --noEmit",
        bunx_tsc_project_noemit: "bunx tsc --project tsconfig.json --noEmit",
        bunx_bun_flag: "bunx --bun eslint src/",
        bunx_no_install_flag: "bunx --no-install eslint .",
        bunx_package_flag: "bunx --package eslint eslint src/",
        bunx_double_dash: "bunx -- eslint src/",
        bunx_version: "bunx --version",
    }

    denied! {
        yarn_install_denied: "yarn install",
        yarn_add_denied: "yarn add react",
        yarn_remove_denied: "yarn remove lodash",
        yarn_upgrade_denied: "yarn upgrade",
        yarn_list_unknown_denied: "yarn list --unknown",
        npm_install_denied: "npm install react",
        npm_uninstall_denied: "npm uninstall lodash",
        npm_run_build_denied: "npm run build",
        npm_run_start_denied: "npm run start",
        npm_config_set_denied: "npm config set registry https://example.com",
        npm_list_unknown_denied: "npm list --unknown",
        npm_audit_unknown_denied: "npm audit --unknown",
        npx_react_scripts_denied: "npx react-scripts start",
        npx_cowsay_denied: "npx cowsay hello",
        bare_npx_denied: "npx",
        npx_only_flags_denied: "npx --yes",
        npx_tsc_without_noemit_denied: "npx tsc",
        pnpm_install_denied: "pnpm install",
        pnpm_add_denied: "pnpm add react",
        pnpm_run_denied: "pnpm run build",
        pnpm_list_unknown_denied: "pnpm list --unknown",
        bun_x_tsc_denied: "bun x tsc",
        bun_x_cowsay_denied: "bun x cowsay hello",
        bun_install_denied: "bun install",
        bun_run_denied: "bun run build",
        bun_add_denied: "bun add react",
        bun_test_unknown_denied: "bun test --unknown",
        deno_fmt_denied: "deno fmt",
        deno_run_denied: "deno run main.ts",
        deno_install_denied: "deno install",
        deno_compile_denied: "deno compile main.ts",
        deno_test_unknown_denied: "deno test --unknown",
        nvm_install_denied: "nvm install 18",
        nvm_use_denied: "nvm use 18",
        nvm_ls_unknown_denied: "nvm ls --unknown",
        fnm_install_denied: "fnm install 18",
        fnm_use_denied: "fnm use 18",
        fnm_list_unknown_denied: "fnm list --unknown",
        volta_install_denied: "volta install node@18",
        volta_pin_denied: "volta pin node@18",
        volta_list_unknown_denied: "volta list --unknown",
        bunx_tsc_without_noemit_denied: "bunx tsc",
        bunx_tsc_with_other_flags_denied: "bunx tsc --pretty",
        bunx_cowsay_denied: "bunx cowsay hello",
        bare_bunx_denied: "bunx",
    }
}
