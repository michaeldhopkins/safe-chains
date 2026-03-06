use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static FMT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--crown-margin", "--split-only", "--tagged-paragraph",
        "--uniform-spacing",
        "-c", "-m", "-n", "-s", "-u",
    ]),
    standalone_short: b"cmnsu",
    valued: WordSet::new(&[
        "--goal", "--prefix", "--width",
        "-d", "-g", "-l", "-p", "-t", "-w",
    ]),
    valued_short: b"dglptw",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "fmt", policy: &FMT_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        fmt_file: "fmt file.txt",
        fmt_width: "fmt -w 72 file.txt",
        fmt_split: "fmt -s file.txt",
        fmt_bare: "fmt",
    }
}
