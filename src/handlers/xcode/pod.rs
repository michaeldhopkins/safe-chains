use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static POD_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static POD_SEARCH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--simple", "--stats", "--web"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static POD_INFO_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static POD_SPEC_CAT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&["--version"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static POD_SPEC_WHICH_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&["--version"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static POD: CommandDef = CommandDef {
    name: "pod",
    subs: &[
        SubDef::Policy { name: "env", policy: &POD_BARE_POLICY },
        SubDef::Policy { name: "info", policy: &POD_INFO_POLICY },
        SubDef::Policy { name: "list", policy: &POD_BARE_POLICY },
        SubDef::Policy { name: "search", policy: &POD_SEARCH_POLICY },
        SubDef::Nested {
            name: "spec",
            subs: &[
                SubDef::Policy { name: "cat", policy: &POD_SPEC_CAT_POLICY },
                SubDef::Policy { name: "which", policy: &POD_SPEC_WHICH_POLICY },
            ],
        },
    ],
    bare_flags: &[],
    help_eligible: true,
    url: "https://guides.cocoapods.org/terminal/commands.html",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        pod_search: "pod search AFNetworking",
        pod_search_simple: "pod search --simple AFNetworking",
        pod_search_stats: "pod search --stats AFNetworking",
        pod_search_web: "pod search --web AFNetworking",
        pod_list: "pod list",
        pod_info: "pod info AFNetworking",
        pod_env: "pod env",
        pod_spec_cat: "pod spec cat AFNetworking",
        pod_spec_cat_version: "pod spec cat AFNetworking --version 4.0",
        pod_spec_which: "pod spec which AFNetworking",
        pod_spec_which_version: "pod spec which AFNetworking --version 4.0",
    }

    denied! {
        pod_bare_denied: "pod",
        pod_install_denied: "pod install",
        pod_update_denied: "pod update",
        pod_repo_denied: "pod repo update",
        pod_spec_bare_denied: "pod spec",
        pod_spec_create_denied: "pod spec create MyPod",
    }
}
