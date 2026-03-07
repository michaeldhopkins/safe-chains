use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static AGVTOOL_BARE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static AGVTOOL: CommandDef = CommandDef {
    name: "agvtool",
    subs: &[
        SubDef::Policy { name: "mvers", policy: &AGVTOOL_BARE_POLICY },
        SubDef::Policy { name: "vers", policy: &AGVTOOL_BARE_POLICY },
        SubDef::Policy { name: "what-marketing-version", policy: &AGVTOOL_BARE_POLICY },
        SubDef::Policy { name: "what-version", policy: &AGVTOOL_BARE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: false,
    url: "https://developer.apple.com/library/archive/qa/qa1827/_index.html",
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        agvtool_what_version: "agvtool what-version",
        agvtool_vers: "agvtool vers",
        agvtool_what_marketing_version: "agvtool what-marketing-version",
        agvtool_mvers: "agvtool mvers",
    }

    denied! {
        agvtool_bare_denied: "agvtool",
        agvtool_new_version_denied: "agvtool new-version 2.0",
        agvtool_new_marketing_denied: "agvtool new-marketing-version 2.0",
        agvtool_bump_denied: "agvtool bump",
    }
}
