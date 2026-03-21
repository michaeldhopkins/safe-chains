use crate::verdict::{SafetyLevel, Verdict};
use crate::command::{CheckFn, CommandDef, SubDef};
use crate::parse::{Token, WordSet};
use crate::policy::{FlagPolicy, FlagStyle};

static BUN_TEST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--bail", "--help", "--only", "--rerun-each", "--todo", "-h"]),
    valued: WordSet::flags(&["--preload", "--timeout", "-t"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BUN_OUTDATED_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BUN_BUILD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--bytecode", "--compile", "--css-chunking",
        "--emit-dce-annotations", "--help", "--minify", "--minify-identifiers",
        "--minify-syntax", "--minify-whitespace", "--no-bundle",
        "--no-clear-screen", "--production", "--react-fast-refresh",
        "--splitting", "--watch",
        "--windows-hide-console",
        "-h",
    ]),
    valued: WordSet::flags(&[
        "--asset-naming", "--banner", "--chunk-naming", "--conditions",
        "--entry-naming", "--env", "--external", "--footer",
        "--format", "--outdir", "--outfile", "--packages",
        "--public-path", "--root", "--sourcemap", "--target",
        "--windows-icon",
        "-e",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static BUN_PM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn check_bun_x(tokens: &[Token]) -> Verdict {
    if super::find_runner_package_index(tokens, 1, &super::BUNX_FLAGS_NO_ARG)
        .is_some_and(|idx| super::is_safe_runner_package(tokens, idx))
    { Verdict::Allowed(SafetyLevel::SafeRead) } else { Verdict::Denied }
}

pub(crate) static BUN: CommandDef = CommandDef {
    name: "bun",
    subs: &[
        SubDef::Policy { name: "build", policy: &BUN_BUILD_POLICY, level: SafetyLevel::SafeWrite },
        SubDef::Policy { name: "test", policy: &BUN_TEST_POLICY, level: SafetyLevel::SafeRead },
        SubDef::Policy { name: "outdated", policy: &BUN_OUTDATED_POLICY, level: SafetyLevel::Inert },
        SubDef::Nested { name: "pm", subs: &[
            SubDef::Policy { name: "bin", policy: &BUN_PM_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "cache", policy: &BUN_PM_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "hash", policy: &BUN_PM_POLICY, level: SafetyLevel::Inert },
            SubDef::Policy { name: "ls", policy: &BUN_PM_POLICY, level: SafetyLevel::Inert },
        ]},
        SubDef::Custom { name: "x", check: check_bun_x as CheckFn, doc: "x delegates to bunx logic.", test_suffix: None },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://bun.sh/docs/cli",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        bun_version: "bun --version",
        bun_help: "bun --help",
        bun_build_entrypoint: "bun build ./src/index.ts",
        bun_build_outfile: "bun build --outfile=bundle.js ./src/index.ts",
        bun_build_outdir: "bun build --outdir=dist ./src/index.ts",
        bun_build_minify: "bun build --minify --splitting --outdir=out ./index.jsx",
        bun_build_production: "bun build --production --outdir=dist ./src/index.ts",
        bun_build_compile: "bun build --compile --outfile=my-app ./cli.ts",
        bun_build_sourcemap: "bun build --sourcemap=linked --outdir=dist ./src/index.ts",
        bun_build_target: "bun build --target=bun --outfile=server.js ./server.ts",
        bun_build_format: "bun build --format=cjs --outdir=dist ./src/index.ts",
        bun_build_external: "bun build --external react --outdir=dist ./src/index.ts",
        bun_build_no_bundle: "bun build --no-bundle ./src/index.ts",
        bun_build_watch: "bun build --watch --outdir=dist ./src/index.ts",
        bun_build_help: "bun build --help",
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
        bun_build_bare: "bun build",
        bun_build_unknown_flag: "bun build --some-unknown ./src/index.ts",
        bun_x_tsc_denied: "bun x tsc",
        bun_x_cowsay_denied: "bun x cowsay hello",
    }
}
