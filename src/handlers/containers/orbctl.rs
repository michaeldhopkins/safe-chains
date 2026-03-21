use crate::command::{CommandDef, SubDef};
use crate::verdict::{SafetyLevel, Verdict};
use crate::parse::{Token, WordSet};
use crate::policy::{FlagPolicy, FlagStyle};

static ORBCTL_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--quiet", "--running",
        "-h", "-q", "-r",
    ]),
    valued: WordSet::flags(&["--format", "-f"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static ORBCTL_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&["--format", "-f"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static ORBCTL_LOGS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--all", "--help", "-a", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static ORBCTL_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static ORBCTL_UPDATE_CHECK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--check", "--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn check_doctor(tokens: &[Token]) -> Verdict {
    if tokens.iter().all(|t| t != "--fix" && t != "-f") { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }

}

fn check_default_bare(tokens: &[Token]) -> Verdict {
    if tokens.len() == 1 { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }

}

static ORBCTL_SUBS: &[SubDef] = &[
    SubDef::Nested { name: "config", subs: &[
        SubDef::Policy { name: "get", policy: &ORBCTL_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "show", policy: &ORBCTL_SIMPLE_POLICY, level: SafetyLevel::Inert },
    ]},
    SubDef::Custom { name: "default", check: check_default_bare, doc: "bare invocation only (reads current default)", test_suffix: None },
    SubDef::Custom { name: "doctor", check: check_doctor, doc: "read-only check (rejects --fix)", test_suffix: None },
    SubDef::Policy { name: "info", policy: &ORBCTL_INFO_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "list", policy: &ORBCTL_LIST_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "logs", policy: &ORBCTL_LOGS_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "status", policy: &ORBCTL_SIMPLE_POLICY, level: SafetyLevel::Inert },
    SubDef::Guarded { name: "update", guard_short: None, guard_long: "--check", policy: &ORBCTL_UPDATE_CHECK_POLICY, level: SafetyLevel::Inert },
    SubDef::Policy { name: "version", policy: &ORBCTL_SIMPLE_POLICY, level: SafetyLevel::Inert },
];

pub(crate) static ORBCTL: CommandDef = CommandDef {
    name: "orbctl",
    subs: ORBCTL_SUBS,
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://docs.orbstack.dev/cli",
    aliases: &["orb"],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        help: "orbctl --help",
        help_short: "orbctl -h",
        version: "orbctl --version",
        version_short: "orbctl -V",
        version_sub: "orbctl version",
        status: "orbctl status",
        list: "orbctl list",
        list_quiet: "orbctl list -q",
        list_running: "orbctl list -r",
        list_format: "orbctl list -f json",
        info: "orbctl info myvm",
        info_format: "orbctl info myvm -f json",
        logs: "orbctl logs myvm",
        logs_all: "orbctl logs myvm -a",
        config_get: "orbctl config get some.key",
        config_show: "orbctl config show",
        update_check: "orbctl update --check",
        doctor: "orbctl doctor",
        default_bare: "orbctl default",
        orb_alias_status: "orb status",
        orb_alias_list: "orb list",
        orb_alias_version: "orb version",
    }

    denied! {
        bare: "orbctl",
        start: "orbctl start myvm",
        stop: "orbctl stop myvm",
        restart: "orbctl restart myvm",
        create: "orbctl create ubuntu",
        delete: "orbctl delete myvm",
        rename: "orbctl rename old new",
        reset: "orbctl reset",
        run: "orbctl run uname",
        push: "orbctl push file.txt",
        pull: "orbctl pull file.txt",
        export: "orbctl export myvm out.tar.zst",
        import: "orbctl import in.tar.zst",
        login: "orbctl login",
        logout: "orbctl logout",
        update_no_check: "orbctl update",
        doctor_fix: "orbctl doctor --fix",
        doctor_fix_short: "orbctl doctor -f",
        default_set: "orbctl default myvm",
        config_set: "orbctl config set key val",
        config_reset: "orbctl config reset",
        config_docker: "orbctl config docker",
        clone: "orbctl clone old new",
        debug: "orbctl debug container",
        unknown: "orbctl unknown",
    }
}
