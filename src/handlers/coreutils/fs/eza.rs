use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static EZA_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--accessed", "--all", "--binary", "--blocks", "--blocksize",
        "--bytes", "--changed", "--classify", "--color-scale", "--color-scale-mode",
        "--context", "--created", "--dereference", "--extended", "--flags",
        "--follow-symlinks", "--git", "--git-ignore", "--git-repos", "--git-repos-no-status",
        "--group", "--group-directories-first", "--header", "--hyperlink", "--icons",
        "--inode", "--links", "--list-dirs", "--long", "--modified",
        "--mounts", "--no-filesize", "--no-git", "--no-icons", "--no-permissions",
        "--no-quotes", "--no-time", "--no-user", "--numeric", "--octal-permissions",
        "--oneline", "--only-dirs", "--only-files", "--recurse", "--reverse",
        "--tree", "-1", "-@", "-A", "-B",
        "-D", "-F", "-G", "-H", "-I",
        "-M", "-R", "-S", "-T", "-U",
        "-Z", "-a", "-b", "-d", "-f",
        "-g", "-h", "-i", "-l", "-m",
        "-r", "-s", "-u", "-x",
    ]),
    standalone_short: b"1@ABDFGHIMRSTUZabdfghilmrsux",
    valued: WordSet::flags(&[
        "--color", "--colour", "--git-ignore-glob", "--grid-columns",
        "--group-directories-first-dirs", "--ignore-glob", "--level",
        "--smart-group", "--sort", "--time", "--time-style",
        "--total-size", "--width",
        "-L", "-X", "-t", "-w",
    ]),
    valued_short: b"LXtw",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "exa", policy: &EZA_POLICY, help_eligible: false, url: "https://eza.rocks/" },
    FlatDef { name: "eza", policy: &EZA_POLICY, help_eligible: false, url: "https://eza.rocks/" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        eza_basic: "eza --long",
        exa_tree: "exa --tree",
        eza_long: "eza --long --git",
        eza_tree: "eza --tree",
    }
}
