use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SYSTEM_PROFILER_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--json", "--version", "--xml", "-V", "-h", "-json", "-listDataTypes",
        "-nospinner", "-xml",
    ]),
    valued: WordSet::flags(&["-detailLevel", "-timeout"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "system_profiler", policy: &SYSTEM_PROFILER_POLICY, level: SafetyLevel::Inert, url: "https://ss64.com/mac/system_profiler.html", aliases: &[] },
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
