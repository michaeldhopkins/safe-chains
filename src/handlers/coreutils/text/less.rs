use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static LESS_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--QUIT-AT-EOF", "--RAW-CONTROL-CHARS", "--chop-long-lines",
        "--help", "--ignore-case", "--no-init", "--quiet", "--quit-at-eof",
        "--quit-if-one-screen", "--raw-control-chars", "--silent",
        "--squeeze-blank-lines", "--version",
        "-E", "-F", "-G", "-I", "-J", "-K", "-L", "-M", "-N",
        "-Q", "-R", "-S", "-V", "-W", "-X",
        "-a", "-c", "-e", "-f", "-g", "-i", "-m", "-n", "-q", "-r", "-s", "-w",
    ]),
    valued: WordSet::flags(&[
        "--LINE-NUMBERS", "--LONG-PROMPT", "--pattern", "--prompt",
        "--shift", "--tabs", "--tag", "--window",
        "-P", "-b", "-h", "-j", "-p", "-t", "-x", "-y", "-z",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "less", policy: &LESS_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/less.1.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        less_file: "less file.txt",
        less_bare: "less",
        less_line_numbers: "less -N file.txt",
        less_chop: "less -S file.txt",
        less_raw: "less -R file.txt",
        less_quit_one_screen: "less -FX file.txt",
        less_ignore_case: "less -i file.txt",
        less_pattern: "less -p pattern file.txt",
        less_tabs: "less --tabs=4 file.txt",
    }

    denied! {
        less_log_file: "less -o output.log file.txt",
        less_log_file_long: "less --log-file=output.log file.txt",
    }
}
