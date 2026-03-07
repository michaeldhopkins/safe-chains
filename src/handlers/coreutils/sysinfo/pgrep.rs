use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static PGREP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--count", "--delimiter", "--full", "--inverse",
        "--lightweight", "--list-full", "--list-name",
        "--newest", "--oldest",
        "-L", "-a", "-c", "-f", "-i", "-l", "-n",
        "-o", "-v", "-w", "-x",
    ]),
    valued: WordSet::flags(&[
        "--euid", "--group", "--parent", "--pgroup", "--pidfile",
        "--session", "--terminal", "--uid", "-F", "-G",
        "-P", "-U", "-d", "-g", "-s",
        "-t", "-u",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "pgrep", policy: &PGREP_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/pgrep.1.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        pgrep_name: "pgrep firefox",
        pgrep_full: "pgrep -f 'python.*server'",
    }
}
