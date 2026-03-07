use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static FOLD_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--bytes", "--spaces",
        "-b", "-s",
    ]),
    valued: WordSet::flags(&[
        "--width",
        "-w",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "fold", policy: &FOLD_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#fold-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        fold_file: "fold file.txt",
        fold_width: "fold -w 80 file.txt",
        fold_bytes: "fold -b file.txt",
        fold_spaces: "fold -s file.txt",
        fold_bare: "fold",
    }
}
