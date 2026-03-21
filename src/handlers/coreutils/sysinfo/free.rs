use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static FREE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--bytes", "--gibi", "--giga", "--help", "--human", "--kibi", "--kilo",
        "--lohi", "--mebi", "--mega", "--si", "--tebi", "--tera",
        "--total", "--version", "--wide",
        "-V", "-b", "-g", "-h", "-k", "-l", "-m", "-t", "-v", "-w",
    ]),
    valued: WordSet::flags(&[
        "--count", "--seconds",
        "-c", "-s",
    ]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "free", policy: &FREE_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/free.1.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        free_bare: "free",
        free_human: "free -h",
        free_total: "free -t",
        free_wide: "free -w",
        free_mega: "free -m",
        free_count: "free -c 3 -s 1",
    }

    denied! {
        free_positional: "free something",
    }
}
