use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static HEROKU_APPS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--all", "--json"]),
    standalone_short: b"a",
    valued: WordSet::flags(&["--space", "--team"]),
    valued_short: b"st",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static HEROKU_APPS_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json", "--shell"]),
    standalone_short: b"s",
    valued: WordSet::flags(&["--app"]),
    valued_short: b"a",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static HEROKU_CONFIG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json", "--shell"]),
    standalone_short: b"js",
    valued: WordSet::flags(&["--app"]),
    valued_short: b"a",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static HEROKU_LOGS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--force-colors", "--tail"]),
    standalone_short: b"t",
    valued: WordSet::flags(&["--app", "--dyno", "--num", "--source"]),
    valued_short: b"adns",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static HEROKU_PS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json"]),
    standalone_short: b"j",
    valued: WordSet::flags(&["--app"]),
    valued_short: b"a",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static HEROKU_RELEASES_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json"]),
    standalone_short: b"j",
    valued: WordSet::flags(&["--app", "--num"]),
    valued_short: b"an",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static HEROKU_REGIONS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json"]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static HEROKU_STATUS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json"]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static HEROKU_BUILDPACKS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&["--app"]),
    valued_short: b"a",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static HEROKU_ADDONS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--all", "--json"]),
    standalone_short: b"A",
    valued: WordSet::flags(&["--app"]),
    valued_short: b"a",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static HEROKU: CommandDef = CommandDef {
    name: "heroku",
    subs: &[
        SubDef::Policy { name: "addons", policy: &HEROKU_ADDONS_POLICY },
        SubDef::Policy { name: "apps", policy: &HEROKU_APPS_POLICY },
        SubDef::Policy { name: "apps:info", policy: &HEROKU_APPS_INFO_POLICY },
        SubDef::Policy { name: "buildpacks", policy: &HEROKU_BUILDPACKS_POLICY },
        SubDef::Policy { name: "config", policy: &HEROKU_CONFIG_POLICY },
        SubDef::Policy { name: "logs", policy: &HEROKU_LOGS_POLICY },
        SubDef::Policy { name: "ps", policy: &HEROKU_PS_POLICY },
        SubDef::Policy { name: "regions", policy: &HEROKU_REGIONS_POLICY },
        SubDef::Policy { name: "releases", policy: &HEROKU_RELEASES_POLICY },
        SubDef::Policy { name: "status", policy: &HEROKU_STATUS_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://devcenter.heroku.com/articles/heroku-cli-commands",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        heroku_apps: "heroku apps",
        heroku_apps_all: "heroku apps --all",
        heroku_apps_json: "heroku apps --json",
        heroku_apps_team: "heroku apps --team my-team",
        heroku_apps_info: "heroku apps:info",
        heroku_apps_info_app: "heroku apps:info --app my-app",
        heroku_apps_info_json: "heroku apps:info --json",
        heroku_config: "heroku config",
        heroku_config_app: "heroku config --app my-app",
        heroku_config_json: "heroku config -j",
        heroku_config_shell: "heroku config -s",
        heroku_logs: "heroku logs",
        heroku_logs_app: "heroku logs --app my-app",
        heroku_logs_tail: "heroku logs -t",
        heroku_logs_num: "heroku logs -n 100",
        heroku_ps: "heroku ps",
        heroku_ps_app: "heroku ps --app my-app",
        heroku_releases: "heroku releases",
        heroku_releases_json: "heroku releases -j",
        heroku_regions: "heroku regions",
        heroku_regions_json: "heroku regions --json",
        heroku_status: "heroku status",
        heroku_status_json: "heroku status --json",
        heroku_buildpacks: "heroku buildpacks",
        heroku_buildpacks_app: "heroku buildpacks --app my-app",
        heroku_addons: "heroku addons",
        heroku_addons_all: "heroku addons --all",
        heroku_addons_app: "heroku addons --app my-app",
        heroku_help: "heroku --help",
    }

    denied! {
        heroku_run: "heroku run bash",
        heroku_config_set: "heroku config:set FOO=bar",
        heroku_bare: "heroku",
    }
}
