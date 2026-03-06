use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static BASE64_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--decode", "--ignore-garbage",
        "-D", "-d", "-i",
    ]),
    standalone_short: b"Ddi",
    valued: WordSet::new(&["--wrap", "-b", "-w"]),
    valued_short: b"bw",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "base64", policy: &BASE64_POLICY, help_eligible: false },
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
