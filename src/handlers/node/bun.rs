use crate::command::{CheckFn, CommandDef, SubDef};
use crate::parse::{Segment, Token, WordSet};
use crate::policy::{FlagPolicy, FlagStyle};

static BUN_TEST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--bail", "--only", "--rerun-each", "--todo"]),
    valued: WordSet::flags(&["--preload", "--timeout", "-t"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BUN_OUTDATED_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BUN_PM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn check_bun_x(tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    super::find_runner_package_index(tokens, 1, &super::BUNX_FLAGS_NO_ARG)
        .is_some_and(|idx| super::is_safe_runner_package(tokens, idx))
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
    url: "https://bun.sh/docs/cli",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
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
    }

    denied! {
        bun_x_tsc_denied: "bun x tsc",
        bun_x_cowsay_denied: "bun x cowsay hello",
    }
}
