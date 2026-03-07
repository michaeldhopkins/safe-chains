use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static PS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--cumulative", "--deselect", "--forest", "--headers", "--info",
        "--no-headers", "-A", "-C", "-H", "-L",
        "-M", "-N", "-S", "-T", "-Z",
        "-a", "-c", "-d", "-e", "-f",
        "-j", "-l", "-m", "-r", "-v",
        "-w", "-x",
    ]),
    standalone_short: b"ACHLMNSTZacdefjlmrvwx",
    valued: WordSet::flags(&[
        "--cols", "--columns", "--format", "--group", "--pid",
        "--ppid", "--rows", "--sid", "--sort", "--tty", "--user",
        "--width",
        "-G", "-O", "-U", "-g", "-n", "-o", "-p", "-s",
        "-t", "-u",
    ]),
    valued_short: b"GOUnopstug",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "ps", policy: &PS_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/ps.1.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        ps_bare: "ps",
        ps_aux: "ps aux",
        ps_ef: "ps -ef",
    }
}
