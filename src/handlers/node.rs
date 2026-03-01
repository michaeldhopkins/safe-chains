use crate::parse::{FlagCheck, Token, WordSet};

static YARN_READ_ONLY: WordSet =
    WordSet::new(&["--version", "info", "list", "ls", "why"]);

static NPM_READ_ONLY: WordSet = WordSet::new(&[
    "--version", "audit", "doctor", "explain", "fund", "info", "list", "ls",
    "outdated", "prefix", "root", "test", "view", "why",
]);

static NPX_SAFE: WordSet =
    WordSet::new(&["@herb-tools/linter", "eslint", "karma"]);

static NPX_FLAGS_NO_ARG: WordSet =
    WordSet::new(&["--ignore-existing", "--no", "--quiet", "--yes", "-q", "-y"]);

static PNPM_READ_ONLY: WordSet =
    WordSet::new(&["--version", "audit", "list", "ls", "outdated", "why"]);

static BUN_SAFE: WordSet =
    WordSet::new(&["--version", "outdated", "test"]);

static BUN_MULTI: &[(&str, WordSet)] =
    &[("pm", WordSet::new(&["bin", "cache", "hash", "ls"]))];

static BUNX_FLAGS_NO_ARG: WordSet =
    WordSet::new(&["--bun", "--no-install", "--silent", "--verbose"]);

static DENO_SAFE: WordSet =
    WordSet::new(&["--version", "check", "doc", "info", "lint", "test"]);

static DENO_FMT: FlagCheck =
    FlagCheck::new(&["--check"], &[]);

static TSC_CHECK: FlagCheck =
    FlagCheck::new(&["--noEmit"], &[]);

static NVM_SAFE: WordSet =
    WordSet::new(&["--version", "current", "list", "ls", "ls-remote", "version", "which"]);

static FNM_SAFE: WordSet =
    WordSet::new(&["--version", "current", "default", "list", "ls-remote"]);

static VOLTA_SAFE: WordSet =
    WordSet::new(&["--version", "list", "which"]);

pub fn is_safe_yarn(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if YARN_READ_ONLY.contains(&tokens[1]) {
        return true;
    }
    tokens[1] == "test" || tokens[1].starts_with("test:")
}

pub fn is_safe_npm(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if NPM_READ_ONLY.contains(&tokens[1]) {
        return true;
    }
    if tokens[1] == "config" {
        return tokens
            .get(2)
            .is_some_and(|a| a == "list" || a == "get");
    }
    if tokens[1] == "run" || tokens[1] == "run-script" {
        return tokens
            .get(2)
            .is_some_and(|a| a == "test" || a.starts_with("test:"));
    }
    false
}

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

pub fn is_safe_pnpm(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && PNPM_READ_ONLY.contains(&tokens[1])
}

pub fn is_safe_bun(tokens: &[Token]) -> bool {
    if tokens.len() >= 2 && tokens[1] == "x" {
        return find_runner_package_index(tokens, 2, &BUNX_FLAGS_NO_ARG)
            .is_some_and(|idx| is_safe_runner_package(tokens, idx));
    }
    super::is_safe_subcmd(tokens, &BUN_SAFE, BUN_MULTI)
}

pub fn is_safe_deno(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if DENO_SAFE.contains(&tokens[1]) {
        return true;
    }
    if tokens[1] == "fmt" {
        return DENO_FMT.is_safe(&tokens[2..]);
    }
    false
}

pub fn is_safe_nvm(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && NVM_SAFE.contains(&tokens[1])
}

pub fn is_safe_fnm(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && FNM_SAFE.contains(&tokens[1])
}

pub fn is_safe_volta(tokens: &[Token]) -> bool {
    tokens.len() >= 2 && VOLTA_SAFE.contains(&tokens[1])
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocBuilder, doc, doc_multi, wordset_items};
    vec![
        CommandDoc::handler("npm",
            doc(&NPM_READ_ONLY)
                .section("Guarded: config (list/get only), run/run-script (test/test:* only).")
                .build()),
        CommandDoc::handler("yarn",
            doc(&YARN_READ_ONLY)
                .section("Also allowed: test, test:*.")
                .build()),
        CommandDoc::wordset("pnpm", &PNPM_READ_ONLY),
        CommandDoc::handler("bun",
            doc_multi(&BUN_SAFE, BUN_MULTI)
                .section("x delegates to bunx logic.")
                .build()),
        CommandDoc::handler("bunx",
            DocBuilder::new()
                .section(format!("Allowed packages: {}.", wordset_items(&NPX_SAFE)))
                .section("Guarded: tsc (requires --noEmit).")
                .section("Skips flags: --bun/--no-install/--package/-p.")
                .build()),
        CommandDoc::handler("deno",
            doc(&DENO_SAFE)
                .section("Guarded: fmt (requires --check).")
                .build()),
        CommandDoc::handler("npx",
            DocBuilder::new()
                .section(format!("Allowed packages: {}.", wordset_items(&NPX_SAFE)))
                .section("Guarded: tsc (requires --noEmit).")
                .section("Skips flags: --yes/-y/--no/--package/-p.")
                .build()),
        CommandDoc::wordset("nvm", &NVM_SAFE),
        CommandDoc::wordset("fnm", &FNM_SAFE),
        CommandDoc::wordset("volta", &VOLTA_SAFE),
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
        yarn_ls: "yarn ls bootstrap",
        yarn_info: "yarn info react",
        yarn_why: "yarn why lodash",
        yarn_version: "yarn --version",
        yarn_test: "yarn test",
        yarn_test_watch: "yarn test:watch",
        yarn_test_with_args: "yarn test --testPathPattern=Foo",
        npm_view: "npm view react version",
        npm_info: "npm info lodash",
        npm_list: "npm list --depth=0",
        npm_ls: "npm ls",
        npm_test: "npm test",
        npm_audit: "npm audit",
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
        pnpm_why: "pnpm why lodash",
        pnpm_audit: "pnpm audit",
        pnpm_outdated: "pnpm outdated",
        pnpm_version: "pnpm --version",
        bun_version: "bun --version",
        bun_test: "bun test",
        bun_pm_ls: "bun pm ls",
        bun_pm_hash: "bun pm hash",
        bun_pm_cache: "bun pm cache",
        bun_pm_bin: "bun pm bin",
        bun_outdated: "bun outdated",
        bun_x_eslint: "bun x eslint src/",
        bun_x_tsc_noemit: "bun x tsc --noEmit",
        deno_version: "deno --version",
        deno_info: "deno info",
        deno_doc: "deno doc mod.ts",
        deno_lint: "deno lint",
        deno_check: "deno check main.ts",
        deno_test: "deno test",
        deno_fmt_check: "deno fmt --check",
        nvm_ls: "nvm ls",
        nvm_list: "nvm list",
        nvm_current: "nvm current",
        nvm_which: "nvm which 18",
        nvm_version: "nvm version",
        nvm_ls_remote: "nvm ls-remote",
        fnm_list: "fnm list",
        fnm_current: "fnm current",
        fnm_default: "fnm default",
        fnm_version: "fnm --version",
        volta_list: "volta list",
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
        npm_install_denied: "npm install react",
        npm_uninstall_denied: "npm uninstall lodash",
        npm_run_build_denied: "npm run build",
        npm_run_start_denied: "npm run start",
        npm_config_set_denied: "npm config set registry https://example.com",
        npx_react_scripts_denied: "npx react-scripts start",
        npx_cowsay_denied: "npx cowsay hello",
        bare_npx_denied: "npx",
        npx_only_flags_denied: "npx --yes",
        npx_tsc_without_noemit_denied: "npx tsc",
        pnpm_install_denied: "pnpm install",
        pnpm_add_denied: "pnpm add react",
        pnpm_run_denied: "pnpm run build",
        bun_x_tsc_denied: "bun x tsc",
        bun_x_cowsay_denied: "bun x cowsay hello",
        bun_install_denied: "bun install",
        bun_run_denied: "bun run build",
        bun_add_denied: "bun add react",
        deno_fmt_denied: "deno fmt",
        deno_run_denied: "deno run main.ts",
        deno_install_denied: "deno install",
        deno_compile_denied: "deno compile main.ts",
        nvm_install_denied: "nvm install 18",
        nvm_use_denied: "nvm use 18",
        fnm_install_denied: "fnm install 18",
        fnm_use_denied: "fnm use 18",
        volta_install_denied: "volta install node@18",
        volta_pin_denied: "volta pin node@18",
        bunx_tsc_without_noemit_denied: "bunx tsc",
        bunx_tsc_with_other_flags_denied: "bunx tsc --pretty",
        bunx_cowsay_denied: "bunx cowsay hello",
        bare_bunx_denied: "bunx",
    }
}
