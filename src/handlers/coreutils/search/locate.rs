use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static LOCATE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--all", "--basename", "--count", "--existing",
        "--follow", "--help", "--ignore-case", "--null", "--quiet",
        "--statistics", "--version", "--wholename",
        "-0", "-A", "-S", "-V", "-b", "-c", "-e", "-h", "-i", "-q", "-w",
    ]),
    valued: WordSet::flags(&[
        "--database", "--limit",
        "-d", "-l", "-n",
    ]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "locate", policy: &LOCATE_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/locate.1.html", aliases: &["mlocate", "plocate"] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        locate_pattern: "locate filename",
        locate_ignore_case: "locate -i filename",
        locate_count: "locate -c filename",
        locate_limit: "locate -l 10 filename",
        locate_stats: "locate -S",
        locate_basename: "locate -b filename",
        mlocate_pattern: "mlocate filename",
        plocate_pattern: "plocate filename",
    }

    denied! {
        locate_bare: "locate",
    }
}
