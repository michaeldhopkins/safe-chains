use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static ICONV_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--list", "--silent", "--version",
        "-V", "-c", "-h", "-l", "-s",
    ]),
    valued: WordSet::flags(&[
        "--from-code", "--to-code",
        "-f", "-t",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "iconv", policy: &ICONV_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/iconv.1.html", aliases: &[] },
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
