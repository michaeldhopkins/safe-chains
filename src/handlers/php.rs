use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

static COMPOSER_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--available", "--direct", "--installed", "--latest",
        "--locked", "--minor-only", "--name-only", "--no-dev", "--outdated",
        "--path", "--platform", "--self", "--strict", "--tree", "--versions",
    ]),
    standalone_short: b"aDHilNosPt",
    valued: WordSet::new(&["--format", "--ignore"]),
    valued_short: b"f",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static COMPOSER_OUTDATED_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--all", "--direct", "--locked", "--minor-only",
        "--no-dev", "--strict",
    ]),
    standalone_short: b"aDm",
    valued: WordSet::new(&["--format", "--ignore"]),
    valued_short: b"f",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static COMPOSER_AUDIT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--abandoned", "--locked", "--no-dev",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&["--format"]),
    valued_short: b"f",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static COMPOSER_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_composer(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    let policy = match tokens[1].as_str() {
        "show" | "info" => &COMPOSER_SHOW_POLICY,
        "outdated" => &COMPOSER_OUTDATED_POLICY,
        "audit" => &COMPOSER_AUDIT_POLICY,
        "about" | "check-platform-reqs" | "diagnose" | "fund"
        | "help" | "licenses" | "suggests" => &COMPOSER_BARE_POLICY,
        _ => return false,
    };
    policy::check(&tokens[1..], policy)
}

pub(crate) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "composer" => Some(is_safe_composer(tokens)),
        _ => None,
    }
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![CommandDoc::handler("composer",
        "Subcommands: about, audit, check-platform-reqs, diagnose, fund, help, info, \
         licenses, outdated, show, suggests.")]
}

#[cfg(test)]
pub(super) const REGISTRY: &[super::CommandEntry] = &[
    super::CommandEntry::Subcommand { cmd: "composer", subs: &[
        super::SubEntry::Policy { name: "show" },
        super::SubEntry::Policy { name: "info" },
        super::SubEntry::Policy { name: "outdated" },
        super::SubEntry::Policy { name: "audit" },
        super::SubEntry::Policy { name: "about" },
        super::SubEntry::Policy { name: "check-platform-reqs" },
        super::SubEntry::Policy { name: "diagnose" },
        super::SubEntry::Policy { name: "fund" },
        super::SubEntry::Policy { name: "help" },
        super::SubEntry::Policy { name: "licenses" },
        super::SubEntry::Policy { name: "suggests" },
    ]},
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        composer_show: "composer show",
        composer_show_package: "composer show laravel/framework",
        composer_show_tree: "composer show --tree",
        composer_show_installed: "composer show --installed",
        composer_show_latest: "composer show --latest",
        composer_show_all: "composer show --all",
        composer_show_format: "composer show --format json",
        composer_info: "composer info",
        composer_diagnose: "composer diagnose",
        composer_outdated: "composer outdated",
        composer_outdated_direct: "composer outdated --direct",
        composer_outdated_strict: "composer outdated --strict",
        composer_licenses: "composer licenses",
        composer_check_platform_reqs: "composer check-platform-reqs",
        composer_suggests: "composer suggests",
        composer_fund: "composer fund",
        composer_audit: "composer audit",
        composer_audit_locked: "composer audit --locked",
        composer_audit_format: "composer audit --format json",
        composer_version: "composer --version",
        composer_about: "composer about",
        composer_help: "composer help",
    }

    denied! {
        composer_install_denied: "composer install",
        composer_update_denied: "composer update",
        composer_require_denied: "composer require laravel/framework",
        composer_remove_denied: "composer remove laravel/framework",
        composer_run_script_denied: "composer run-script test",
        bare_composer_denied: "composer",
    }
}
