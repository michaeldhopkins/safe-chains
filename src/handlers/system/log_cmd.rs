use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static LOG_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--backtrace", "--debug", "--help", "--info", "--loss", "--mach-continuous-time",
        "--no-pager", "--signpost",
        "-h",
    ]),
    valued: WordSet::flags(&[
        "--color", "--end", "--last", "--predicate",
        "--process", "--source", "--start", "--style", "--type",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static LOG_STREAM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--backtrace", "--debug", "--help", "--info", "--loss",
        "--mach-continuous-time", "--signpost",
        "-h",
    ]),
    valued: WordSet::flags(&[
        "--color", "--level", "--predicate", "--process",
        "--source", "--style", "--timeout", "--type",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static LOG_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static LOG: CommandDef = CommandDef {
    name: "log",
    subs: &[
        SubDef::Policy { name: "show", policy: &LOG_SHOW_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "stream", policy: &LOG_STREAM_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "help", policy: &LOG_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "stats", policy: &LOG_SIMPLE_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h"],
    url: "https://ss64.com/mac/log.html",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        log_help: "log help",
        log_show: "log show --predicate 'process == \"Safari\"' --last 1h",
        log_show_style: "log show --style compact",
        log_show_info: "log show --info",
        log_show_debug: "log show --debug",
        log_stats: "log stats",
        log_stream: "log stream --level debug",
        log_stream_process: "log stream --process Safari",
    }
}
