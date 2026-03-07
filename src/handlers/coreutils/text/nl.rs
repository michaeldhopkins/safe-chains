use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static NL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--no-renumber",
        "-p",
    ]),
    standalone_short: b"p",
    valued: WordSet::flags(&[
        "--body-numbering", "--footer-numbering", "--header-numbering",
        "--join-blank-lines", "--line-increment", "--number-format",
        "--number-separator", "--number-width", "--section-delimiter",
        "--starting-line-number",
        "-b", "-d", "-f", "-h", "-i", "-l", "-n", "-s", "-v", "-w",
    ]),
    valued_short: b"bdfhilnsvw",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "nl", policy: &NL_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#nl-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        nl_file: "nl file.txt",
        nl_bare: "nl",
        nl_body: "nl -b a file.txt",
        nl_format: "nl -n rz file.txt",
        nl_width: "nl -w 4 file.txt",
    }
}
