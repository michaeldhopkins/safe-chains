use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static NROFF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "-S", "-c", "-h", "-i", "-k", "-p", "-q", "-t",
    ]),
    valued: WordSet::flags(&[
        "-M", "-P", "-T", "-d", "-m", "-n", "-o", "-r", "-w",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "nroff", policy: &NROFF_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/nroff.1.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        nroff_file: "nroff -man page.1",
        nroff_macro: "nroff -m mandoc page.1",
        nroff_term: "nroff -T ascii page.1",
    }
}
