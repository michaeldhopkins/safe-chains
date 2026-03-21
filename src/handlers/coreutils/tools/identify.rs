use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static IDENTIFY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--verbose", "--version", "-V", "-h", "-ping", "-quiet", "-regard-warnings",
        "-verbose",
    ]),
    valued: WordSet::flags(&[
        "-channel", "-define", "-density", "-depth",
        "-features", "-format", "-fuzz", "-interlace",
        "-limit", "-list", "-log", "-moments",
        "-monitor", "-precision", "-seed", "-set",
        "-size", "-strip", "-unique",
        "-virtual-pixel",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "identify", policy: &IDENTIFY_POLICY, level: SafetyLevel::Inert, url: "https://imagemagick.org/script/identify.php", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        identify_file: "identify image.png",
    }
}
