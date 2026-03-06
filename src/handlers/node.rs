use crate::command::{CheckFn, CommandDef, SubDef};
use crate::parse::{Segment, Token, WordSet, has_flag};
use crate::policy::{self, FlagPolicy, FlagStyle};

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
    flag_style: FlagStyle::Strict,
};

static NPM_VIEW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
};

static NPM_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static NPM_TEST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static NPM_CONFIG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json", "--long"]),
    standalone_short: b"l",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn check_npm_run(tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    tokens.get(1).is_some_and(|a| a == "test" || a.starts_with("test:"))
}

pub(crate) static NPM: CommandDef = CommandDef {
    name: "npm",
    subs: &[
        SubDef::Policy { name: "list", policy: &NPM_LIST_POLICY },
        SubDef::Policy { name: "ls", policy: &NPM_LIST_POLICY },
        SubDef::Policy { name: "view", policy: &NPM_VIEW_POLICY },
        SubDef::Policy { name: "info", policy: &NPM_VIEW_POLICY },
        SubDef::Policy { name: "audit", policy: &NPM_AUDIT_POLICY },
        SubDef::Policy { name: "test", policy: &NPM_TEST_POLICY },
        SubDef::Policy { name: "doctor", policy: &NPM_BARE_POLICY },
        SubDef::Policy { name: "explain", policy: &NPM_BARE_POLICY },
        SubDef::Policy { name: "fund", policy: &NPM_BARE_POLICY },
        SubDef::Policy { name: "outdated", policy: &NPM_BARE_POLICY },
        SubDef::Policy { name: "prefix", policy: &NPM_BARE_POLICY },
        SubDef::Policy { name: "root", policy: &NPM_BARE_POLICY },
        SubDef::Policy { name: "why", policy: &NPM_BARE_POLICY },
        SubDef::Nested { name: "config", subs: &[
            SubDef::Policy { name: "get", policy: &NPM_CONFIG_POLICY },
            SubDef::Policy { name: "list", policy: &NPM_CONFIG_POLICY },
        ]},
        SubDef::Custom { name: "run", check: check_npm_run as CheckFn, doc: "run/run-script (test only).", test_suffix: None },
        SubDef::Custom { name: "run-script", check: check_npm_run as CheckFn, doc: " ", test_suffix: None },
    ],
    bare_flags: &[],
    help_eligible: true,
};

static YARN_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json", "--long", "--production"]),
    standalone_short: b"",
    valued: WordSet::new(&["--depth", "--pattern"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static YARN_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
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
    flag_style: FlagStyle::Strict,
};

static PNPM_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--json", "--recursive"]),
    standalone_short: b"r",
    valued: WordSet::new(&["--filter"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static PNPM: CommandDef = CommandDef {
    name: "pnpm",
    subs: &[
        SubDef::Policy { name: "list", policy: &PNPM_LIST_POLICY },
        SubDef::Policy { name: "ls", policy: &PNPM_LIST_POLICY },
        SubDef::Policy { name: "audit", policy: &PNPM_BARE_POLICY },
        SubDef::Policy { name: "outdated", policy: &PNPM_BARE_POLICY },
        SubDef::Policy { name: "why", policy: &PNPM_BARE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

static BUN_TEST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--bail", "--only", "--rerun-each", "--todo"]),
    standalone_short: b"",
    valued: WordSet::new(&["--preload", "--timeout"]),
    valued_short: b"t",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BUN_OUTDATED_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BUN_PM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static NPX_SAFE: WordSet =
    WordSet::new(&["@herb-tools/linter", "eslint", "karma"]);

static NPX_FLAGS_NO_ARG: WordSet =
    WordSet::new(&["--ignore-existing", "--no", "--quiet", "--yes", "-q", "-y"]);

static BUNX_FLAGS_NO_ARG: WordSet =
    WordSet::new(&["--bun", "--no-install", "--silent", "--verbose"]);

static TSC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--allowJs", "--checkJs", "--esModuleInterop",
        "--forceConsistentCasingInFileNames", "--incremental",
        "--isolatedModules", "--noEmit", "--noFallthroughCasesInSwitch",
        "--noImplicitAny", "--noImplicitReturns", "--noUnusedLocals",
        "--noUnusedParameters", "--pretty", "--resolveJsonModule",
        "--skipLibCheck", "--strict", "--strictNullChecks",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "--baseUrl", "--jsx", "--lib", "--module",
        "--moduleResolution", "--project",
        "--rootDir", "--target",
    ]),
    valued_short: b"p",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

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
        return has_flag(&tokens[pkg_idx..], None, Some("--noEmit"))
            && policy::check(&tokens[pkg_idx..], &TSC_POLICY);
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

fn check_bun_x(tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    find_runner_package_index(tokens, 1, &BUNX_FLAGS_NO_ARG)
        .is_some_and(|idx| is_safe_runner_package(tokens, idx))
}

pub(crate) static BUN: CommandDef = CommandDef {
    name: "bun",
    subs: &[
        SubDef::Policy { name: "test", policy: &BUN_TEST_POLICY },
        SubDef::Policy { name: "outdated", policy: &BUN_OUTDATED_POLICY },
        SubDef::Nested { name: "pm", subs: &[
            SubDef::Policy { name: "bin", policy: &BUN_PM_POLICY },
            SubDef::Policy { name: "cache", policy: &BUN_PM_POLICY },
            SubDef::Policy { name: "hash", policy: &BUN_PM_POLICY },
            SubDef::Policy { name: "ls", policy: &BUN_PM_POLICY },
        ]},
        SubDef::Custom { name: "x", check: check_bun_x as CheckFn, doc: "x delegates to bunx logic.", test_suffix: None },
    ],
    bare_flags: &[],
    help_eligible: true,
};

static DENO_SAFE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--json", "--no-lock", "--quiet", "--unstable",
    ]),
    standalone_short: b"q",
    valued: WordSet::new(&["--config", "--import-map"]),
    valued_short: b"c",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DENO_FMT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--check", "--no-semicolons", "--single-quote",
        "--unstable",
    ]),
    standalone_short: b"q",
    valued: WordSet::new(&[
        "--config", "--ext", "--ignore", "--indent-width",
        "--line-width", "--log-level", "--prose-wrap",
    ]),
    valued_short: b"c",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static DENO: CommandDef = CommandDef {
    name: "deno",
    subs: &[
        SubDef::Policy { name: "check", policy: &DENO_SAFE_POLICY },
        SubDef::Policy { name: "doc", policy: &DENO_SAFE_POLICY },
        SubDef::Policy { name: "info", policy: &DENO_SAFE_POLICY },
        SubDef::Policy { name: "lint", policy: &DENO_SAFE_POLICY },
        SubDef::Policy { name: "test", policy: &DENO_SAFE_POLICY },
        SubDef::Guarded { name: "fmt", guard_short: None, guard_long: "--check", policy: &DENO_FMT_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

static NVM_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--lts", "--no-colors"]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static NVM: CommandDef = CommandDef {
    name: "nvm",
    subs: &[
        SubDef::Policy { name: "current", policy: &NVM_BARE_POLICY },
        SubDef::Policy { name: "list", policy: &NVM_BARE_POLICY },
        SubDef::Policy { name: "ls", policy: &NVM_BARE_POLICY },
        SubDef::Policy { name: "ls-remote", policy: &NVM_BARE_POLICY },
        SubDef::Policy { name: "version", policy: &NVM_BARE_POLICY },
        SubDef::Policy { name: "which", policy: &NVM_BARE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

static FNM_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static FNM: CommandDef = CommandDef {
    name: "fnm",
    subs: &[
        SubDef::Policy { name: "current", policy: &FNM_BARE_POLICY },
        SubDef::Policy { name: "default", policy: &FNM_BARE_POLICY },
        SubDef::Policy { name: "list", policy: &FNM_BARE_POLICY },
        SubDef::Policy { name: "ls-remote", policy: &FNM_BARE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

static VOLTA_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&["--current", "--default"]),
    standalone_short: b"cd",
    valued: WordSet::new(&["--format"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static VOLTA: CommandDef = CommandDef {
    name: "volta",
    subs: &[
        SubDef::Policy { name: "list", policy: &VOLTA_BARE_POLICY },
        SubDef::Policy { name: "which", policy: &VOLTA_BARE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    NPM.dispatch(cmd, tokens, is_safe)
        .or_else(|| match cmd {
            "yarn" => Some(is_safe_yarn(tokens)),
            _ => None,
        })
        .or_else(|| PNPM.dispatch(cmd, tokens, is_safe))
        .or_else(|| BUN.dispatch(cmd, tokens, is_safe))
        .or_else(|| DENO.dispatch(cmd, tokens, is_safe))
        .or_else(|| match cmd {
            "npx" => Some(is_safe_npx(tokens)),
            "bunx" => Some(is_safe_bunx(tokens)),
            _ => None,
        })
        .or_else(|| NVM.dispatch(cmd, tokens, is_safe))
        .or_else(|| FNM.dispatch(cmd, tokens, is_safe))
        .or_else(|| VOLTA.dispatch(cmd, tokens, is_safe))
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocBuilder, wordset_items};
    vec![
        NPM.to_doc(),
        CommandDoc::handler("yarn",
            "Subcommands: info, list, ls, test, test:*, why."),
        PNPM.to_doc(),
        BUN.to_doc(),
        CommandDoc::handler("bunx",
            DocBuilder::new()
                .section(format!("Allowed packages: {}.", wordset_items(&NPX_SAFE)))
                .section("tsc allowed with --noEmit.")
                .section("Skips flags: --bun/--no-install/--package/-p.")
                .build()),
        DENO.to_doc(),
        CommandDoc::handler("npx",
            DocBuilder::new()
                .section(format!("Allowed packages: {}.", wordset_items(&NPX_SAFE)))
                .section("tsc allowed with --noEmit.")
                .section("Skips flags: --yes/-y/--no/--package/-p.")
                .build()),
        NVM.to_doc(),
        FNM.to_doc(),
        VOLTA.to_doc(),
    ]
}

#[cfg(test)]
pub(super) const REGISTRY: &[super::CommandEntry] = &[
    super::CommandEntry::Subcommand { cmd: "yarn", subs: &[
        super::SubEntry::Policy { name: "list" },
        super::SubEntry::Policy { name: "ls" },
        super::SubEntry::Policy { name: "info" },
        super::SubEntry::Policy { name: "why" },
        super::SubEntry::Positional { name: "test" },
    ]},
    super::CommandEntry::Positional { cmd: "npx" },
    super::CommandEntry::Positional { cmd: "bunx" },
];

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
        npm_run_build_denied: "npm run build",
        npm_run_start_denied: "npm run start",
        npm_config_set_denied: "npm config set registry https://example.com",
        npx_react_scripts_denied: "npx react-scripts start",
        npx_cowsay_denied: "npx cowsay hello",
        bare_npx_denied: "npx",
        npx_only_flags_denied: "npx --yes",
        npx_tsc_without_noemit_denied: "npx tsc",
        bun_x_tsc_denied: "bun x tsc",
        bun_x_cowsay_denied: "bun x cowsay hello",
        deno_fmt_denied: "deno fmt",
        bunx_tsc_without_noemit_denied: "bunx tsc",
        bunx_tsc_with_other_flags_denied: "bunx tsc --pretty",
        bunx_cowsay_denied: "bunx cowsay hello",
        bare_bunx_denied: "bunx",
    }
}
