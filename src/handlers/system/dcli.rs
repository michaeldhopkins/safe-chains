use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DCLI_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DCLI_DEVICES_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json"]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DCLI_TEAM_MEMBERS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--csv", "--human-readable"]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DCLI_TEAM_LOGS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--csv", "--human-readable"]),
    standalone_short: b"",
    valued: WordSet::flags(&["--end", "--start"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DCLI_TEAM_REPORT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DCLI_TEAM_DWI_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&["--count", "--offset", "--order-by"]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static DCLI_JSON_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json"]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static DCLI: CommandDef = CommandDef {
    name: "dcli",
    subs: &[
        SubDef::Nested {
            name: "accounts",
            subs: &[
                SubDef::Policy { name: "whoami", policy: &DCLI_BARE_POLICY },
            ],
        },
        SubDef::Nested {
            name: "devices",
            subs: &[
                SubDef::Policy { name: "list", policy: &DCLI_DEVICES_LIST_POLICY },
            ],
        },
        SubDef::Policy { name: "lock", policy: &DCLI_BARE_POLICY },
        SubDef::Policy { name: "sync", policy: &DCLI_BARE_POLICY },
        SubDef::Nested {
            name: "team",
            subs: &[
                SubDef::Nested {
                    name: "credentials",
                    subs: &[
                        SubDef::Policy { name: "list", policy: &DCLI_JSON_LIST_POLICY },
                    ],
                },
                SubDef::Policy { name: "dark-web-insights", policy: &DCLI_TEAM_DWI_POLICY },
                SubDef::Policy { name: "logs", policy: &DCLI_TEAM_LOGS_POLICY },
                SubDef::Policy { name: "members", policy: &DCLI_TEAM_MEMBERS_POLICY },
                SubDef::Nested {
                    name: "public-api",
                    subs: &[
                        SubDef::Policy { name: "list-keys", policy: &DCLI_JSON_LIST_POLICY },
                    ],
                },
                SubDef::Policy { name: "report", policy: &DCLI_TEAM_REPORT_POLICY },
            ],
        },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://cli.dashlane.com/",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        dcli_sync: "dcli sync",
        dcli_lock: "dcli lock",
        dcli_help: "dcli --help",
        dcli_accounts_whoami: "dcli accounts whoami",
        dcli_devices_list: "dcli devices list",
        dcli_devices_list_json: "dcli devices list --json",
        dcli_team_members: "dcli team members",
        dcli_team_members_csv: "dcli team members --csv",
        dcli_team_members_human: "dcli team members --human-readable",
        dcli_team_logs: "dcli team logs",
        dcli_team_logs_csv: "dcli team logs --csv",
        dcli_team_logs_start: "dcli team logs --start 2024-01-01",
        dcli_team_logs_end: "dcli team logs --end 2024-12-31",
        dcli_team_logs_human: "dcli team logs --human-readable",
        dcli_team_report: "dcli team report",
        dcli_team_report_days: "dcli team report 30",
        dcli_team_dwi_domain: "dcli team dark-web-insights example.com",
        dcli_team_dwi_count: "dcli team dark-web-insights example.com --count 10",
        dcli_team_dwi_offset: "dcli team dark-web-insights example.com --offset 5",
        dcli_team_dwi_order: "dcli team dark-web-insights example.com --order-by email",
        dcli_team_credentials_list: "dcli team credentials list",
        dcli_team_credentials_list_json: "dcli team credentials list --json",
        dcli_team_public_api_list_keys: "dcli team public-api list-keys",
        dcli_team_public_api_list_keys_json: "dcli team public-api list-keys --json",
    }

    denied! {
        dcli_bare_denied: "dcli",
        dcli_unknown_denied: "dcli xyzzy",
        dcli_accounts_bare_denied: "dcli accounts",
        dcli_devices_bare_denied: "dcli devices",
        dcli_team_bare_denied: "dcli team",
        dcli_team_dwi_bare_denied: "dcli team dark-web-insights",
        dcli_team_credentials_bare_denied: "dcli team credentials",
        dcli_team_public_api_bare_denied: "dcli team public-api",
        dcli_password_denied: "dcli password",
        dcli_note_denied: "dcli note",
        dcli_configure_denied: "dcli configure",
    }
}
