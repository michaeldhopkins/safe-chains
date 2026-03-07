use crate::command::{CheckFn, CommandDef, SubDef};
use crate::parse::{Segment, Token, WordSet};
use crate::policy::{FlagPolicy, FlagStyle};

static NPM_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--json", "--link", "--long", "--omit",
        "--parseable", "--production", "--unicode",
    ]),
    standalone_short: b"al",
    valued: WordSet::flags(&["--depth", "--prefix"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static NPM_VIEW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json"]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static NPM_AUDIT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--json", "--omit", "--production",
    ]),
    standalone_short: b"",
    valued: WordSet::flags(&["--audit-level"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static NPM_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json"]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static NPM_TEST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static NPM_CONFIG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json", "--long"]),
    standalone_short: b"l",
    valued: WordSet::flags(&[]),
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
    url: "https://docs.npmjs.com/cli",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
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
    }

    denied! {
        npm_run_build_denied: "npm run build",
        npm_run_start_denied: "npm run start",
        npm_config_set_denied: "npm config set registry https://example.com",
    }
}
