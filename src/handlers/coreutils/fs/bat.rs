use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static BAT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--diff", "--help", "--list-languages", "--list-themes", "--no-config",
        "--number", "--plain", "--show-all", "--version",
        "-A", "-P", "-V", "-d", "-h", "-n", "-p", "-u",
    ]),
    valued: WordSet::flags(&[
        "--color", "--decorations", "--diff-context", "--file-name",
        "--highlight-line", "--italic-text", "--language", "--line-range",
        "--map-syntax", "--paging", "--style", "--tabs",
        "--terminal-width", "--theme", "--wrap",
        "-H", "-l", "-m", "-r",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "bat", policy: &BAT_POLICY, level: SafetyLevel::Inert, url: "https://github.com/sharkdp/bat#readme", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        bat_file: "bat file.txt",
        bat_plain: "bat -p file.txt",
        bat_language: "bat -l rust file.txt",
        bat_line_range: "bat -r 10:20 file.txt",
        bat_theme: "bat --theme=gruvbox file.txt",
        bat_number: "bat -n file.txt",
        bat_bare: "bat",
    }

    denied! {
        bat_pager_denied: "bat --pager 'rm -rf /' file",
    }
}
