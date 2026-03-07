use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static FLYCTL_STATUS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json", "-j"]),
    valued: WordSet::flags(&["--app", "-a"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static FLYCTL_LOGS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&["--app", "--instance", "--region", "-a", "-i", "-r"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static FLYCTL_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static FLYCTL_RELEASES_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json", "-j"]),
    valued: WordSet::flags(&["--app", "-a"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static FLYCTL_APP_JSON_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json", "-j"]),
    valued: WordSet::flags(&["--app", "-a"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static FLYCTL_SUBS: &[SubDef] = &[
    SubDef::Nested { name: "apps", subs: &[
        SubDef::Policy { name: "list", policy: &FLYCTL_BARE_POLICY },
    ]},
    SubDef::Nested { name: "config", subs: &[
        SubDef::Policy { name: "show", policy: &FLYCTL_APP_JSON_POLICY },
    ]},
    SubDef::Nested { name: "ips", subs: &[
        SubDef::Policy { name: "list", policy: &FLYCTL_APP_JSON_POLICY },
    ]},
    SubDef::Policy { name: "logs", policy: &FLYCTL_LOGS_POLICY },
    SubDef::Nested { name: "platform", subs: &[
        SubDef::Policy { name: "regions", policy: &FLYCTL_BARE_POLICY },
    ]},
    SubDef::Nested { name: "regions", subs: &[
        SubDef::Policy { name: "list", policy: &FLYCTL_APP_JSON_POLICY },
    ]},
    SubDef::Policy { name: "releases", policy: &FLYCTL_RELEASES_POLICY },
    SubDef::Nested { name: "services", subs: &[
        SubDef::Policy { name: "list", policy: &FLYCTL_APP_JSON_POLICY },
    ]},
    SubDef::Policy { name: "status", policy: &FLYCTL_STATUS_POLICY },
    SubDef::Policy { name: "version", policy: &FLYCTL_BARE_POLICY },
];

pub(crate) static FLYCTL: CommandDef = CommandDef {
    name: "flyctl",
    subs: FLYCTL_SUBS,
    bare_flags: &[],
    help_eligible: true,
    url: "https://fly.io/docs/flyctl/",
};

pub(crate) static FLY: CommandDef = CommandDef {
    name: "fly",
    subs: FLYCTL_SUBS,
    bare_flags: &[],
    help_eligible: true,
    url: "https://fly.io/docs/flyctl/",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        flyctl_status: "flyctl status",
        flyctl_status_app: "flyctl status --app my-app",
        flyctl_status_json: "flyctl status -j",
        flyctl_logs: "flyctl logs",
        flyctl_logs_app: "flyctl logs --app my-app",
        flyctl_logs_region: "flyctl logs -r iad",
        flyctl_apps_list: "flyctl apps list",
        flyctl_releases: "flyctl releases",
        flyctl_releases_app: "flyctl releases --app my-app",
        flyctl_config_show: "flyctl config show",
        flyctl_config_show_app: "flyctl config show --app my-app",
        flyctl_regions_list: "flyctl regions list",
        flyctl_version: "flyctl version",
        flyctl_ips_list: "flyctl ips list",
        flyctl_services_list: "flyctl services list",
        flyctl_platform_regions: "flyctl platform regions",
        fly_status: "fly status",
        fly_logs: "fly logs",
        fly_apps_list: "fly apps list",
        fly_version: "fly version",
        flyctl_help: "flyctl --help",
        fly_help: "fly --help",
    }

    denied! {
        flyctl_deploy: "flyctl deploy",
        flyctl_destroy: "flyctl apps destroy my-app",
        flyctl_bare: "flyctl",
        fly_bare: "fly",
    }
}
