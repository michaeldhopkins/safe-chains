use crate::parse::{Segment, Token, WordSet, has_flag};
use crate::policy::{self, FlagPolicy, FlagStyle};

static JJPR_AUTH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static JJPR_SUBMIT_DRY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--draft", "--dry-run", "--no-fetch", "--ready",
    ]),
    valued: WordSet::flags(&[
        "--base", "--remote", "--reviewer",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static JJPR_MERGE_DRY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--dry-run", "--no-ci-check", "--no-fetch",
    ]),
    valued: WordSet::flags(&[
        "--base", "--merge-method", "--required-approvals",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static AUTH_ACTIONS: WordSet = WordSet::new(&["setup", "test"]);

fn is_safe_jjpr(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return true;
    }
    let subcmd = &tokens[1];

    if matches!(subcmd.as_str(), "--help" | "-h" | "--version" | "-V") {
        return tokens.len() == 2;
    }

    if subcmd == "auth" {
        if tokens.len() < 3 || !AUTH_ACTIONS.contains(&tokens[2]) {
            return false;
        }
        return policy::check(&tokens[2..], &JJPR_AUTH_POLICY);
    }

    if subcmd == "submit" {
        return has_flag(&tokens[1..], None, Some("--dry-run"))
            && policy::check(&tokens[1..], &JJPR_SUBMIT_DRY_POLICY);
    }

    if subcmd == "merge" {
        return has_flag(&tokens[1..], None, Some("--dry-run"))
            && policy::check(&tokens[1..], &JJPR_MERGE_DRY_POLICY);
    }

    false
}

pub(in crate::handlers::forges) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "jjpr" => Some(is_safe_jjpr(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::{CommandDoc, DocBuilder};
    vec![
        CommandDoc::handler("jjpr",
            "https://github.com/michaeldhopkins/jjpr",
            DocBuilder::new()
                .section("Bare invocation allowed (displays stack status).")
                .section("auth (test, setup).")
                .section("submit (requires --dry-run), merge (requires --dry-run).")
                .section("")
                .build()),
    ]
}

#[cfg(test)]
pub(super) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Subcommand { cmd: "jjpr", bare_ok: true, subs: &[
        crate::handlers::SubEntry::Nested { name: "auth", subs: &[
            crate::handlers::SubEntry::Policy { name: "test" },
            crate::handlers::SubEntry::Policy { name: "setup" },
        ]},
        crate::handlers::SubEntry::Guarded { name: "submit", valid_suffix: "--dry-run" },
        crate::handlers::SubEntry::Guarded { name: "merge", valid_suffix: "--dry-run" },
    ]},
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        bare: "jjpr",
        help: "jjpr --help",
        help_short: "jjpr -h",
        version: "jjpr --version",
        version_short: "jjpr -V",
        auth_test: "jjpr auth test",
        auth_setup: "jjpr auth setup",
        submit_dry: "jjpr submit --dry-run",
        submit_dry_bookmark: "jjpr submit my-stack --dry-run",
        submit_dry_draft: "jjpr submit --dry-run --draft",
        submit_dry_reviewer: "jjpr submit --dry-run --reviewer user",
        merge_dry: "jjpr merge --dry-run",
        merge_dry_bookmark: "jjpr merge my-stack --dry-run",
        merge_dry_method: "jjpr merge --dry-run --merge-method squash",
    }

    denied! {
        submit_denied: "jjpr submit",
        submit_bookmark_denied: "jjpr submit my-stack",
        merge_denied: "jjpr merge",
        merge_bookmark_denied: "jjpr merge my-stack",
        config_init_denied: "jjpr config init",
        unknown_sub_denied: "jjpr foo",
        unknown_flag_denied: "jjpr --unknown",
        auth_bare_denied: "jjpr auth",
        auth_unknown_denied: "jjpr auth login",
    }
}
