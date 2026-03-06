use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

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

pub(crate) static COMPOSER: CommandDef = CommandDef {
    name: "composer",
    subs: &[
        SubDef::Policy { name: "about", policy: &COMPOSER_BARE_POLICY },
        SubDef::Policy { name: "audit", policy: &COMPOSER_AUDIT_POLICY },
        SubDef::Policy { name: "check-platform-reqs", policy: &COMPOSER_BARE_POLICY },
        SubDef::Policy { name: "diagnose", policy: &COMPOSER_BARE_POLICY },
        SubDef::Policy { name: "fund", policy: &COMPOSER_BARE_POLICY },
        SubDef::Policy { name: "help", policy: &COMPOSER_BARE_POLICY },
        SubDef::Policy { name: "info", policy: &COMPOSER_SHOW_POLICY },
        SubDef::Policy { name: "licenses", policy: &COMPOSER_BARE_POLICY },
        SubDef::Policy { name: "outdated", policy: &COMPOSER_OUTDATED_POLICY },
        SubDef::Policy { name: "show", policy: &COMPOSER_SHOW_POLICY },
        SubDef::Policy { name: "suggests", policy: &COMPOSER_BARE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

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
}
