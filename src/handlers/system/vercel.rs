use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static VERCEL_LIST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json"]),
    standalone_short: b"j",
    valued: WordSet::flags(&["--meta", "--next", "--scope"]),
    valued_short: b"mS",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static VERCEL_INSPECT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json"]),
    standalone_short: b"j",
    valued: WordSet::flags(&["--scope", "--timeout"]),
    valued_short: b"ST",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static VERCEL_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static VERCEL_PROJECT_LS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--json"]),
    standalone_short: b"j",
    valued: WordSet::flags(&["--scope"]),
    valued_short: b"S",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static VERCEL: CommandDef = CommandDef {
    name: "vercel",
    subs: &[
        SubDef::Policy { name: "inspect", policy: &VERCEL_INSPECT_POLICY },
        SubDef::Policy { name: "list", policy: &VERCEL_LIST_POLICY },
        SubDef::Nested { name: "project", subs: &[
            SubDef::Policy { name: "ls", policy: &VERCEL_PROJECT_LS_POLICY },
        ]},
        SubDef::Policy { name: "whoami", policy: &VERCEL_BARE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://vercel.com/docs/cli",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        vercel_list: "vercel list",
        vercel_list_json: "vercel list -j",
        vercel_list_scope: "vercel list --scope my-team",
        vercel_inspect: "vercel inspect https://my-app.vercel.app",
        vercel_inspect_json: "vercel inspect -j https://my-app.vercel.app",
        vercel_whoami: "vercel whoami",
        vercel_project_ls: "vercel project ls",
        vercel_project_ls_json: "vercel project ls -j",
        vercel_help: "vercel --help",
    }

    denied! {
        vercel_deploy: "vercel deploy",
        vercel_env: "vercel env pull",
        vercel_bare: "vercel",
    }
}
