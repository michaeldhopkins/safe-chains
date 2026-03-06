use crate::parse::{Segment, Token, WordSet};

static NPX_FLAGS_NO_ARG: WordSet =
    WordSet::new(&["--ignore-existing", "--no", "--quiet", "--yes", "-q", "-y"]);

pub fn is_safe_npx(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if tokens.len() == 2 && tokens[1] == "--version" {
        return true;
    }
    super::find_runner_package_index(tokens, 1, &NPX_FLAGS_NO_ARG)
        .is_some_and(|idx| super::is_safe_runner_package(tokens, idx))
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "npx" => Some(is_safe_npx(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocBuilder, wordset_items};
    vec![
        CommandDoc::handler("npx",
            DocBuilder::new()
                .section(format!("Allowed packages: {}.", wordset_items(&super::NPX_SAFE)))
                .section("tsc allowed with --noEmit.")
                .section("Skips flags: --yes/-y/--no/--package/-p.")
                .build()),
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
        npx_herb_linter: "npx @herb-tools/linter app/views/foo.html.erb",
        npx_eslint: "npx eslint src/",
        npx_karma: "npx karma start",
        npx_yes_flag: "npx --yes eslint src/",
        npx_y_flag: "npx -y @herb-tools/linter .",
        npx_package_flag: "npx --package @herb-tools/linter @herb-tools/linter .",
        npx_double_dash: "npx -- eslint src/",
        npx_version: "npx --version",
        npx_tsc_noemit: "npx tsc --noEmit",
    }

    denied! {
        npx_react_scripts_denied: "npx react-scripts start",
        npx_cowsay_denied: "npx cowsay hello",
        bare_npx_denied: "npx",
        npx_only_flags_denied: "npx --yes",
        npx_tsc_without_noemit_denied: "npx tsc",
    }
}
