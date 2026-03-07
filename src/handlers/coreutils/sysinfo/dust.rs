use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DUST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--bars-on-right", "--files0-from", "--ignore-all-in-file", "--invert-filter", "--no-colors",
        "--no-percent-bars", "--only-dir", "--only-file", "--skip-total", "-D",
        "-F", "-H", "-P", "-R", "-S",
        "-b", "-c", "-f", "-i", "-p",
        "-r", "-s",
    ]),
    standalone_short: b"DFHPbcfiprRsS",
    valued: WordSet::flags(&[
        "--depth", "--exclude", "--filter", "--terminal_width",
        "-M", "-X", "-d", "-e", "-n", "-t", "-v", "-w", "-z",
    ]),
    valued_short: b"MXdentvwz",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "dust", policy: &DUST_POLICY, help_eligible: false, url: "https://github.com/bootandy/dust#readme" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        dust_bare: "dust",
        dust_depth: "dust -d 2",
    }
}
