use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static EXPAND_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--initial", "--version",
        "-V", "-h", "-i",
    ]),
    valued: WordSet::flags(&[
        "--tabs",
        "-t",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "expand", policy: &EXPAND_POLICY, level: SafetyLevel::Inert, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#expand-invocation", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        expand_file: "expand file.txt",
        expand_initial: "expand -i file.txt",
        expand_tabs: "expand -t 4 file.txt",
        expand_bare: "expand",
    }
}
