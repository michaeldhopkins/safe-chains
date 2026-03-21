use crate::parse::{Token, WordSet, has_flag};
use crate::verdict::{SafetyLevel, Verdict};
use crate::policy::{self, FlagPolicy, FlagStyle};

static JJPR_AUTH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static JJPR_SUBMIT_DRY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--draft", "--dry-run", "--help", "--no-fetch", "--ready",
        "-h",
    ]),
    valued: WordSet::flags(&[
        "--base", "--remote", "--reviewer",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static JJPR_STATUS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--dry-run", "--help", "--no-fetch",
        "-h",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static JJPR_MERGE_DRY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--dry-run", "--help", "--no-ci-check", "--no-fetch", "--watch",
        "-h",
    ]),
    valued: WordSet::flags(&[
        "--base", "--merge-method", "--reconcile-strategy", "--remote", "--required-approvals",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static AUTH_ACTIONS: WordSet = WordSet::new(&["setup", "test"]);

fn is_safe_jjpr(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    let subcmd = &tokens[1];

    if matches!(subcmd.as_str(), "--help" | "-h" | "--version" | "-V") {
        return if tokens.len() == 2 { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }

    if subcmd == "help" {
        return if tokens.len() <= 3 { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }

    if tokens.len() == 3 && matches!(tokens[2].as_str(), "--help" | "-h" | "help") {
        return Verdict::Allowed(SafetyLevel::Inert);
    }

    if subcmd == "auth" {
        if tokens.len() < 3 || !AUTH_ACTIONS.contains(&tokens[2]) {
            return Verdict::Denied;
        }
        return if policy::check(&tokens[2..], &JJPR_AUTH_POLICY) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }

    if subcmd == "status" {
        return if policy::check(&tokens[1..], &JJPR_STATUS_POLICY) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }

    if subcmd == "submit" {
        return if has_flag(&tokens[1..], None, Some("--dry-run"))
            && policy::check(&tokens[1..], &JJPR_SUBMIT_DRY_POLICY)
        { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }

    if subcmd == "merge" {
        return if has_flag(&tokens[1..], None, Some("--dry-run"))
            && policy::check(&tokens[1..], &JJPR_MERGE_DRY_POLICY)
        { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied };
    }

    if subcmd == "config" {
        return Verdict::Denied;
    }

    Verdict::Denied

}

pub(in crate::handlers::forges) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
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
                .section("status allowed.")
                .section("auth (test, setup).")
                .section("submit (requires --dry-run), merge (requires --dry-run).")
                .section("--help allowed on all subcommands.")
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
        crate::handlers::SubEntry::Policy { name: "config" },
        crate::handlers::SubEntry::Guarded { name: "merge", valid_suffix: "--dry-run" },
        crate::handlers::SubEntry::Policy { name: "status" },
        crate::handlers::SubEntry::Guarded { name: "submit", valid_suffix: "--dry-run" },
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
        auth_help: "jjpr auth --help",
        auth_help_short: "jjpr auth -h",
        auth_help_sub: "jjpr auth help",
        status_bare: "jjpr status",
        status_help: "jjpr status --help",
        status_help_short: "jjpr status -h",
        status_help_sub: "jjpr status help",
        status_no_fetch: "jjpr status --no-fetch",
        status_branch: "jjpr status cycle/renewal-request-sent-layout",
        status_branch_no_fetch: "jjpr status my-stack --no-fetch",
        submit_dry: "jjpr submit --dry-run",
        submit_dry_bookmark: "jjpr submit my-stack --dry-run",
        submit_dry_draft: "jjpr submit --dry-run --draft",
        submit_dry_reviewer: "jjpr submit --dry-run --reviewer user",
        submit_help: "jjpr submit --help",
        submit_help_short: "jjpr submit -h",
        submit_help_sub: "jjpr submit help",
        merge_dry: "jjpr merge --dry-run",
        merge_dry_bookmark: "jjpr merge my-stack --dry-run",
        merge_dry_method: "jjpr merge --dry-run --merge-method squash",
        merge_dry_watch: "jjpr merge --dry-run --watch",
        merge_dry_remote: "jjpr merge --dry-run --remote origin",
        merge_dry_reconcile: "jjpr merge --dry-run --reconcile-strategy rebase",
        merge_help: "jjpr merge --help",
        merge_help_short: "jjpr merge -h",
        merge_help_sub: "jjpr merge help",
        help_sub: "jjpr help",
        help_sub_submit: "jjpr help submit",
        help_sub_merge: "jjpr help merge",
        help_sub_auth: "jjpr help auth",
        help_sub_config: "jjpr help config",
        help_sub_status: "jjpr help status",
        config_help: "jjpr config --help",
        config_help_short: "jjpr config -h",
        config_help_sub: "jjpr config help",
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
        status_unknown_flag_denied: "jjpr status --verbose",
    }
}
