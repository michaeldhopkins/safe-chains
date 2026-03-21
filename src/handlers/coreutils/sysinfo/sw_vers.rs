use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SW_VERS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--buildVersion", "--help", "--productName",
        "--productVersion", "--productVersionExtra", "--version",
        "-V", "-h",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "sw_vers", policy: &SW_VERS_POLICY, level: SafetyLevel::Inert, url: "https://ss64.com/mac/sw_vers.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        sw_vers_bare: "sw_vers",
        sw_vers_name: "sw_vers --productName",
    }
}
