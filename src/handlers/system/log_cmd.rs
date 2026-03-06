use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static LOG_SHOW_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--backtrace", "--debug", "--info", "--loss", "--mach-continuous-time",
        "--no-pager", "--signpost",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "--color", "--end", "--last", "--predicate",
        "--process", "--source", "--start", "--style", "--type",
    ]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static LOG_STREAM_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--backtrace", "--debug", "--info", "--loss",
        "--mach-continuous-time", "--signpost",
    ]),
    standalone_short: b"",
    valued: WordSet::new(&[
        "--color", "--level", "--predicate", "--process",
        "--source", "--style", "--timeout", "--type",
    ]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static LOG_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static LOG: CommandDef = CommandDef {
    name: "log",
    subs: &[
        SubDef::Policy { name: "show", policy: &LOG_SHOW_POLICY },
        SubDef::Policy { name: "stream", policy: &LOG_STREAM_POLICY },
        SubDef::Policy { name: "help", policy: &LOG_SIMPLE_POLICY },
        SubDef::Policy { name: "stats", policy: &LOG_SIMPLE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
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
