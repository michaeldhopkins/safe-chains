use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static FX_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--raw", "--slurp", "--themes", "--version",
        "-h", "-r", "-s",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Positional,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "fx", policy: &FX_POLICY, level: SafetyLevel::Inert, url: "https://github.com/antonmedv/fx", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        fx_bare: "fx",
        fx_file: "fx data.json",
        fx_filter: "fx data.json '.name'",
        fx_raw: "fx -r data.json",
        fx_slurp: "fx -s data.json",
    }
}
