use crate::verdict::{SafetyLevel, Verdict};
use crate::parse::Token;

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "bunx" => Some(if super::is_safe_runner(tokens, &super::BUNX_FLAGS_NO_ARG) { Verdict::Allowed(SafetyLevel::SafeRead) } else { Verdict::Denied }),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocBuilder, wordset_items};
    vec![
        CommandDoc::handler("bunx",
            "https://bun.sh/docs/cli/bunx",
            DocBuilder::new()
                .section(format!("Allowed packages: {}.", wordset_items(&super::NPX_SAFE)))
                .section("tsc allowed with --noEmit.")
                .section("Skips flags: --bun/--no-install/--package/-p.")
                .build()),
    ]
}

#[cfg(test)]
pub(crate) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Positional { cmd: "bunx" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
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
        bunx_tsc_without_noemit_denied: "bunx tsc",
        bunx_tsc_with_other_flags_denied: "bunx tsc --pretty",
        bunx_cowsay_denied: "bunx cowsay hello",
        bare_bunx_denied: "bunx",
    }
}
