use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static ICONV_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--list", "--silent",
        "-c", "-l", "-s",
    ]),
    standalone_short: b"cls",
    valued: WordSet::new(&[
        "--from-code", "--to-code",
        "-f", "-t",
    ]),
    valued_short: b"ft",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "iconv", policy: &ICONV_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        iconv_convert: "iconv -f UTF-8 -t ASCII file.txt",
        iconv_list: "iconv -l",
        iconv_silent: "iconv -s -f LATIN1 -t UTF-8 file",
    }

    denied! {
        iconv_output_denied: "iconv -o output.txt file",
    }
}
