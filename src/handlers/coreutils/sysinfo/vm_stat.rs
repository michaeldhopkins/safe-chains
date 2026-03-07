use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static VM_STAT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&["-c"]),
    valued_short: b"c",
    bare: true,
    max_positional: Some(1),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "vm_stat", policy: &VM_STAT_POLICY, help_eligible: false, url: "https://ss64.com/mac/vm_stat.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        vm_stat_bare: "vm_stat",
        vm_stat_interval: "vm_stat 5",
    }
}
