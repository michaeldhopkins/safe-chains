use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

static SWIFTFORMAT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--dryrun", "--lenient", "--lint", "--quiet", "--strict", "--verbose",
    ]),
    valued: WordSet::flags(&["--config", "--disable", "--enable", "--rules"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_swiftformat(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    static REQUIRED: WordSet = WordSet::new(&["--dryrun", "--lint"]);
    if !tokens[1..].iter().any(|t| REQUIRED.contains(t)) {
        return false;
    }
    policy::check(tokens, &SWIFTFORMAT_POLICY)
}

pub(in crate::handlers::xcode) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    if cmd == "swiftformat" {
        Some(is_safe_swiftformat(tokens))
    } else {
        None
    }
}

pub(in crate::handlers::xcode) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("swiftformat",
            "https://github.com/nicklockwood/SwiftFormat",
            "Requires --lint or --dryrun."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::xcode) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "swiftformat", valid_prefix: Some("swiftformat --lint .") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        swiftformat_lint: "swiftformat --lint .",
        swiftformat_lint_config: "swiftformat --lint --config .swiftformat .",
        swiftformat_dryrun: "swiftformat --dryrun .",
        swiftformat_lint_quiet: "swiftformat --lint --quiet .",
        swiftformat_lint_verbose: "swiftformat --lint --verbose .",
        swiftformat_lint_strict: "swiftformat --lint --strict .",
        swiftformat_lint_lenient: "swiftformat --lint --lenient .",
        swiftformat_lint_rules: "swiftformat --lint --rules redundantSelf .",
        swiftformat_lint_disable: "swiftformat --lint --disable trailingCommas .",
        swiftformat_lint_enable: "swiftformat --lint --enable isEmpty .",
        swiftformat_dryrun_config: "swiftformat --dryrun --config .swiftformat Sources/",
    }

    denied! {
        swiftformat_bare_denied: "swiftformat",
        swiftformat_no_lint_denied: "swiftformat .",
        swiftformat_format_denied: "swiftformat --config .swiftformat .",
        swiftformat_infixspacing_denied: "swiftformat --infixspacing .",
    }
}
