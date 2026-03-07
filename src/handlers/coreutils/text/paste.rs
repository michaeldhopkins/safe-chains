use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static PASTE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--serial", "--zero-terminated",
        "-s", "-z",
    ]),
    valued: WordSet::flags(&[
        "--delimiters",
        "-d",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "paste", policy: &PASTE_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#paste-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        paste_files: "paste file1 file2",
        paste_serial: "paste -s file",
        paste_delim: "paste -d, file1 file2",
        paste_bare: "paste",
    }
}
