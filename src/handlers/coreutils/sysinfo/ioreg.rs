use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static IOREG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "-S", "-a", "-b", "-f", "-i", "-l", "-r",
        "-t", "-x",
    ]),
    valued: WordSet::flags(&[
        "-c", "-d", "-k", "-n", "-p", "-w",
    ]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "ioreg", policy: &IOREG_POLICY, help_eligible: false, url: "https://ss64.com/mac/ioreg.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        ioreg_bare: "ioreg",
        ioreg_tree: "ioreg -t",
    }
}
