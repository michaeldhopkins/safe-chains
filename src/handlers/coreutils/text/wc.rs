use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static WC_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--bytes", "--chars", "--lines", "--max-line-length", "--words",
        "--zero-terminated",
        "-L", "-c", "-l", "-m", "-w",
    ]),
    standalone_short: b"Lclmw",
    valued: WordSet::flags(&["--files0-from"]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "wc", policy: &WC_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#wc-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        wc_default: "wc file.txt",
        wc_lines: "wc -l file.txt",
        wc_words: "wc -w file.txt",
        wc_chars: "wc -m file.txt",
        wc_combined: "wc -lw file.txt",
        wc_bare: "wc",
    }
}
