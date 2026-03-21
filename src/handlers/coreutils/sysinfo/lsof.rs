use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static LSOF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--version",
        "-C", "-G", "-M", "-N", "-O", "-P", "-R",
        "-U", "-V", "-X", "-b", "-h",
        "-l", "-n", "-t", "-w", "-x",
    ]),
    valued: WordSet::flags(&[
        "-F", "-S", "-T", "-a", "-c", "-d", "-g",
        "-i", "-k", "-o", "-p", "-r", "-s", "-u",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "lsof", policy: &LSOF_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man8/lsof.8.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        lsof_bare: "lsof",
        lsof_port: "lsof -i :8080",
    }
}
