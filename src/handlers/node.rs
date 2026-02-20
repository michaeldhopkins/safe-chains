use std::collections::HashSet;
use std::sync::LazyLock;

static YARN_READ_ONLY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["list", "info", "why", "--version"]));

static NPM_READ_ONLY: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "view", "info", "list", "ls", "test", "audit", "outdated", "explain", "why", "fund",
        "prefix", "root", "doctor",
    ])
});

static NPX_SAFE: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["eslint", "@herb-tools/linter", "karma"]));

static NPX_FLAGS_NO_ARG: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["--yes", "-y", "--no", "--ignore-existing", "-q", "--quiet"]));

static PNPM_READ_ONLY: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["list", "why", "audit", "outdated", "--version"]));

static BUN_SAFE: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["--version", "test"]));

static BUN_MULTI: LazyLock<Vec<(&'static str, HashSet<&'static str>)>> =
    LazyLock::new(|| vec![("pm", HashSet::from(["ls", "hash", "cache"]))]);

static DENO_SAFE: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["--version", "info", "doc", "lint", "check", "test"]));

static NVM_SAFE: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from(["ls", "list", "current", "which", "version", "--version", "ls-remote"])
});

static FNM_SAFE: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["list", "current", "default", "--version", "ls-remote"]));

static VOLTA_SAFE: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| HashSet::from(["list", "which", "--version"]));

pub fn is_safe_yarn(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if YARN_READ_ONLY.contains(tokens[1].as_str()) {
        return true;
    }
    tokens[1] == "test" || tokens[1].starts_with("test:")
}

pub fn is_safe_npm(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if NPM_READ_ONLY.contains(tokens[1].as_str()) {
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

pub fn is_safe_npx(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let mut i = 1;
    while i < tokens.len() {
        if tokens[i] == "--package" || tokens[i] == "-p" {
            i += 2;
            continue;
        }
        if NPX_FLAGS_NO_ARG.contains(tokens[i].as_str()) {
            i += 1;
            continue;
        }
        if tokens[i] == "--" {
            i += 1;
            break;
        }
        if tokens[i].starts_with('-') {
            return false;
        }
        break;
    }
    i < tokens.len() && NPX_SAFE.contains(tokens[i].as_str())
}

pub fn is_safe_pnpm(tokens: &[String]) -> bool {
    tokens.len() >= 2 && PNPM_READ_ONLY.contains(tokens[1].as_str())
}

pub fn is_safe_bun(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if BUN_SAFE.contains(tokens[1].as_str()) {
        return true;
    }
    for (prefix, actions) in BUN_MULTI.iter() {
        if tokens[1] == *prefix {
            return tokens.get(2).is_some_and(|a| actions.contains(a.as_str()));
        }
    }
    false
}

pub fn is_safe_deno(tokens: &[String]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if DENO_SAFE.contains(tokens[1].as_str()) {
        return true;
    }
    if tokens[1] == "fmt" {
        return tokens[2..].iter().any(|t| t == "--check");
    }
    false
}

pub fn is_safe_nvm(tokens: &[String]) -> bool {
    tokens.len() >= 2 && NVM_SAFE.contains(tokens[1].as_str())
}

pub fn is_safe_fnm(tokens: &[String]) -> bool {
    tokens.len() >= 2 && FNM_SAFE.contains(tokens[1].as_str())
}

pub fn is_safe_volta(tokens: &[String]) -> bool {
    tokens.len() >= 2 && VOLTA_SAFE.contains(tokens[1].as_str())
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocKind};
    vec![
        CommandDoc {
            name: "npm",
            kind: DocKind::Handler,
            description: "Read-only: view, info, list, ls, test, audit, outdated, explain, why, fund, prefix, root, doctor. \
                          Guarded: config (list/get only), run/run-script (test/test:* only).",
        },
        CommandDoc {
            name: "yarn",
            kind: DocKind::Handler,
            description: "Read-only: list, info, why, --version. Also allowed: test, test:*.",
        },
        CommandDoc {
            name: "pnpm",
            kind: DocKind::Handler,
            description: "Allowed: list, why, audit, outdated, --version.",
        },
        CommandDoc {
            name: "bun",
            kind: DocKind::Handler,
            description: "Allowed: --version, test. Multi-word: pm ls/hash/cache.",
        },
        CommandDoc {
            name: "deno",
            kind: DocKind::Handler,
            description: "Allowed: --version, info, doc, lint, check, test. Guarded: fmt (requires --check).",
        },
        CommandDoc {
            name: "npx",
            kind: DocKind::Handler,
            description: "Whitelisted packages only: eslint, @herb-tools/linter, karma. Skips flags: --yes/-y/--no/--package/-p.",
        },
        CommandDoc {
            name: "nvm",
            kind: DocKind::Handler,
            description: "Allowed: ls, list, current, which, version, --version, ls-remote.",
        },
        CommandDoc {
            name: "fnm",
            kind: DocKind::Handler,
            description: "Allowed: list, current, default, --version, ls-remote.",
        },
        CommandDoc {
            name: "volta",
            kind: DocKind::Handler,
            description: "Allowed: list, which, --version.",
        },
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe;

    fn check(cmd: &str) -> bool {
        is_safe(cmd)
    }

    #[test]
    fn yarn_list() {
        assert!(check("yarn list --depth=0"));
    }

    #[test]
    fn yarn_info() {
        assert!(check("yarn info react"));
    }

    #[test]
    fn yarn_why() {
        assert!(check("yarn why lodash"));
    }

    #[test]
    fn yarn_version() {
        assert!(check("yarn --version"));
    }

    #[test]
    fn yarn_test() {
        assert!(check("yarn test"));
    }

    #[test]
    fn yarn_test_watch() {
        assert!(check("yarn test:watch"));
    }

    #[test]
    fn yarn_test_with_args() {
        assert!(check("yarn test --testPathPattern=Foo"));
    }

    #[test]
    fn yarn_install_denied() {
        assert!(!check("yarn install"));
    }

    #[test]
    fn yarn_add_denied() {
        assert!(!check("yarn add react"));
    }

    #[test]
    fn yarn_remove_denied() {
        assert!(!check("yarn remove lodash"));
    }

    #[test]
    fn yarn_upgrade_denied() {
        assert!(!check("yarn upgrade"));
    }

    #[test]
    fn npm_view() {
        assert!(check("npm view react version"));
    }

    #[test]
    fn npm_info() {
        assert!(check("npm info lodash"));
    }

    #[test]
    fn npm_list() {
        assert!(check("npm list --depth=0"));
    }

    #[test]
    fn npm_ls() {
        assert!(check("npm ls"));
    }

    #[test]
    fn npm_test() {
        assert!(check("npm test"));
    }

    #[test]
    fn npm_audit() {
        assert!(check("npm audit"));
    }

    #[test]
    fn npm_outdated() {
        assert!(check("npm outdated"));
    }

    #[test]
    fn npm_explain() {
        assert!(check("npm explain lodash"));
    }

    #[test]
    fn npm_why() {
        assert!(check("npm why lodash"));
    }

    #[test]
    fn npm_fund() {
        assert!(check("npm fund"));
    }

    #[test]
    fn npm_prefix() {
        assert!(check("npm prefix"));
    }

    #[test]
    fn npm_root() {
        assert!(check("npm root"));
    }

    #[test]
    fn npm_doctor() {
        assert!(check("npm doctor"));
    }

    #[test]
    fn npm_config_list() {
        assert!(check("npm config list"));
    }

    #[test]
    fn npm_config_get() {
        assert!(check("npm config get registry"));
    }

    #[test]
    fn npm_run_test() {
        assert!(check("npm run test"));
    }

    #[test]
    fn npm_run_test_colon() {
        assert!(check("npm run test:unit"));
    }

    #[test]
    fn npm_install_denied() {
        assert!(!check("npm install react"));
    }

    #[test]
    fn npm_uninstall_denied() {
        assert!(!check("npm uninstall lodash"));
    }

    #[test]
    fn npm_run_build_denied() {
        assert!(!check("npm run build"));
    }

    #[test]
    fn npm_run_start_denied() {
        assert!(!check("npm run start"));
    }

    #[test]
    fn npm_config_set_denied() {
        assert!(!check("npm config set registry https://example.com"));
    }

    #[test]
    fn npx_herb_linter() {
        assert!(check("npx @herb-tools/linter app/views/foo.html.erb"));
    }

    #[test]
    fn npx_eslint() {
        assert!(check("npx eslint src/"));
    }

    #[test]
    fn npx_karma() {
        assert!(check("npx karma start"));
    }


    #[test]
    fn npx_yes_flag() {
        assert!(check("npx --yes eslint src/"));
    }

    #[test]
    fn npx_y_flag() {
        assert!(check("npx -y @herb-tools/linter ."));
    }

    #[test]
    fn npx_package_flag() {
        assert!(check(
            "npx --package @herb-tools/linter @herb-tools/linter ."
        ));
    }

    #[test]
    fn npx_double_dash() {
        assert!(check("npx -- eslint src/"));
    }

    #[test]
    fn npx_react_scripts_denied() {
        assert!(!check("npx react-scripts start"));
    }

    #[test]
    fn npx_cowsay_denied() {
        assert!(!check("npx cowsay hello"));
    }

    #[test]
    fn bare_npx_denied() {
        assert!(!check("npx"));
    }

    #[test]
    fn npx_only_flags_denied() {
        assert!(!check("npx --yes"));
    }

    #[test]
    fn pnpm_list() {
        assert!(check("pnpm list"));
    }

    #[test]
    fn pnpm_why() {
        assert!(check("pnpm why lodash"));
    }

    #[test]
    fn pnpm_audit() {
        assert!(check("pnpm audit"));
    }

    #[test]
    fn pnpm_outdated() {
        assert!(check("pnpm outdated"));
    }

    #[test]
    fn pnpm_version() {
        assert!(check("pnpm --version"));
    }

    #[test]
    fn pnpm_install_denied() {
        assert!(!check("pnpm install"));
    }

    #[test]
    fn pnpm_add_denied() {
        assert!(!check("pnpm add react"));
    }

    #[test]
    fn pnpm_run_denied() {
        assert!(!check("pnpm run build"));
    }

    #[test]
    fn bun_version() {
        assert!(check("bun --version"));
    }

    #[test]
    fn bun_test() {
        assert!(check("bun test"));
    }

    #[test]
    fn bun_pm_ls() {
        assert!(check("bun pm ls"));
    }

    #[test]
    fn bun_pm_hash() {
        assert!(check("bun pm hash"));
    }

    #[test]
    fn bun_pm_cache() {
        assert!(check("bun pm cache"));
    }

    #[test]
    fn bun_install_denied() {
        assert!(!check("bun install"));
    }

    #[test]
    fn bun_run_denied() {
        assert!(!check("bun run build"));
    }

    #[test]
    fn bun_add_denied() {
        assert!(!check("bun add react"));
    }

    #[test]
    fn deno_version() {
        assert!(check("deno --version"));
    }

    #[test]
    fn deno_info() {
        assert!(check("deno info"));
    }

    #[test]
    fn deno_doc() {
        assert!(check("deno doc mod.ts"));
    }

    #[test]
    fn deno_lint() {
        assert!(check("deno lint"));
    }

    #[test]
    fn deno_check() {
        assert!(check("deno check main.ts"));
    }

    #[test]
    fn deno_test() {
        assert!(check("deno test"));
    }

    #[test]
    fn deno_fmt_check() {
        assert!(check("deno fmt --check"));
    }

    #[test]
    fn deno_fmt_denied() {
        assert!(!check("deno fmt"));
    }

    #[test]
    fn deno_run_denied() {
        assert!(!check("deno run main.ts"));
    }

    #[test]
    fn deno_install_denied() {
        assert!(!check("deno install"));
    }

    #[test]
    fn deno_compile_denied() {
        assert!(!check("deno compile main.ts"));
    }

    #[test]
    fn nvm_ls() {
        assert!(check("nvm ls"));
    }

    #[test]
    fn nvm_list() {
        assert!(check("nvm list"));
    }

    #[test]
    fn nvm_current() {
        assert!(check("nvm current"));
    }

    #[test]
    fn nvm_which() {
        assert!(check("nvm which 18"));
    }

    #[test]
    fn nvm_version() {
        assert!(check("nvm version"));
    }

    #[test]
    fn nvm_ls_remote() {
        assert!(check("nvm ls-remote"));
    }

    #[test]
    fn nvm_install_denied() {
        assert!(!check("nvm install 18"));
    }

    #[test]
    fn nvm_use_denied() {
        assert!(!check("nvm use 18"));
    }

    #[test]
    fn fnm_list() {
        assert!(check("fnm list"));
    }

    #[test]
    fn fnm_current() {
        assert!(check("fnm current"));
    }

    #[test]
    fn fnm_default() {
        assert!(check("fnm default"));
    }

    #[test]
    fn fnm_version() {
        assert!(check("fnm --version"));
    }

    #[test]
    fn fnm_install_denied() {
        assert!(!check("fnm install 18"));
    }

    #[test]
    fn fnm_use_denied() {
        assert!(!check("fnm use 18"));
    }

    #[test]
    fn volta_list() {
        assert!(check("volta list"));
    }

    #[test]
    fn volta_which() {
        assert!(check("volta which node"));
    }

    #[test]
    fn volta_version() {
        assert!(check("volta --version"));
    }

    #[test]
    fn volta_install_denied() {
        assert!(!check("volta install node@18"));
    }

    #[test]
    fn volta_pin_denied() {
        assert!(!check("volta pin node@18"));
    }
}
