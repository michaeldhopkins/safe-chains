use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static IDENTIFY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--verbose", "-ping", "-quiet", "-regard-warnings",
        "-verbose",
    ]),
    standalone_short: b"",
    valued: WordSet::flags(&[
        "-channel", "-define", "-density", "-depth",
        "-features", "-format", "-fuzz", "-interlace",
        "-limit", "-list", "-log", "-moments",
        "-monitor", "-precision", "-seed", "-set",
        "-size", "-strip", "-unique",
        "-virtual-pixel",
    ]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "identify", policy: &IDENTIFY_POLICY, help_eligible: false, url: "https://imagemagick.org/script/identify.php" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        identify_file: "identify image.png",
    }
}
