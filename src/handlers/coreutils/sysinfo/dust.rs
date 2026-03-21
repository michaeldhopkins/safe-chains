use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DUST_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--bars-on-right", "--files0-from", "--help", "--ignore-all-in-file", "--invert-filter", "--no-colors",
        "--no-percent-bars", "--only-dir", "--only-file", "--skip-total", "--version", "-D",
        "-F", "-H", "-P", "-R", "-S", "-V",
        "-b", "-c", "-f", "-h", "-i", "-p",
        "-r", "-s",
    ]),
    valued: WordSet::flags(&[
        "--depth", "--exclude", "--filter", "--terminal_width",
        "-M", "-X", "-d", "-e", "-n", "-t", "-v", "-w", "-z",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "dust", policy: &DUST_POLICY, level: SafetyLevel::Inert, url: "https://github.com/bootandy/dust#readme", aliases: &[] },
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
