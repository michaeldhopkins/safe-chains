use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SYSTEM_PROFILER_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--json", "--xml", "-json", "-listDataTypes",
        "-nospinner", "-xml",
    ]),
    standalone_short: b"",
    valued: WordSet::flags(&["-detailLevel", "-timeout"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "system_profiler", policy: &SYSTEM_PROFILER_POLICY, help_eligible: false, url: "https://ss64.com/mac/system_profiler.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        system_profiler_bare: "system_profiler",
        system_profiler_hw: "system_profiler SPHardwareDataType",
    }
}
