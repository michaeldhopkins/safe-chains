use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static COLUMN_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--fillrows", "--help", "--json", "--keep-empty-lines", "--table",
        "--table-noextreme", "--table-noheadings", "--table-right-all", "--version",
        "-J", "-L", "-R", "-V", "-e", "-h", "-n", "-t", "-x",
    ]),
    valued: WordSet::flags(&[
        "--output-separator", "--separator", "--table-columns",
        "--table-empty-lines", "--table-hide", "--table-name",
        "--table-order", "--table-right", "--table-truncate", "--table-wrap",
        "-E", "-H", "-O", "-W", "-c", "-d", "-o", "-r", "-s",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "column", policy: &COLUMN_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/column.1.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        column_file: "column file.txt",
        column_table: "column -t file.txt",
        column_separator: "column -s, file.txt",
        column_json: "column -J file.txt",
        column_bare: "column",
    }
}
