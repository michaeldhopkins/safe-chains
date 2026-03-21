use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static COL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help", "--version",
        "-V", "-b", "-f", "-h", "-p", "-x",
    ]),
    valued: WordSet::flags(&["-l"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "col", policy: &COL_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/col.1.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        col_bare: "col",
        col_strip_backspaces: "col -b",
        col_flags: "col -bfx",
        col_lines: "col -l 200",
    }
}
