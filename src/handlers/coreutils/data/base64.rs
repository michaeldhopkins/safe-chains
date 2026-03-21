use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static BASE64_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--decode", "--help", "--ignore-garbage", "--version",
        "-D", "-V", "-d", "-h", "-i",
    ]),
    valued: WordSet::flags(&["--wrap", "-b", "-w"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "base64", policy: &BASE64_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#base64-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        base64_decode: "base64 -d file.txt",
        base64_encode: "base64 file.txt",
    }
}
