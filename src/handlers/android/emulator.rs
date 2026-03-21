use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static EMULATOR_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--version", "-V", "-h", "-help", "-list-avds", "-version"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub static DEFS: &[FlatDef] = &[
    FlatDef {
        name: "emulator",
        policy: &EMULATOR_POLICY,
        level: SafetyLevel::Inert,
        url: "https://developer.android.com/studio/run/emulator-commandline",
        aliases: &[],
    },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        emulator_list_avds: "emulator -list-avds",
        emulator_version: "emulator -version",
        emulator_help: "emulator -help",
    }

    denied! {
        emulator_bare_denied: "emulator",
        emulator_launch_denied: "emulator @Pixel_6",
        emulator_wipe_denied: "emulator -wipe-data @Pixel_6",
        emulator_unknown_denied: "emulator --unknown",
    }
}
