use crate::verdict::{SafetyLevel, Verdict};
use crate::parse::Token;

pub fn check_bun_x(tokens: &[Token]) -> Verdict {
    if tokens.len() == 2 && (tokens[1] == "--help" || tokens[1] == "-h") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    match super::find_runner_package_index(tokens, 1, &super::BUNX_FLAGS_NO_ARG) {
        Some(idx) => super::runner_verdict(tokens, idx),
        None => Verdict::Denied,
    }
}

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
