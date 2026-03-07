use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static LASTLOG_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&["--before", "--time", "--user", "-b", "-t", "-u"]),
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "lastlog", policy: &LASTLOG_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man8/lastlog.8.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        lastlog_bare: "lastlog",
        lastlog_user: "lastlog -u root",
    }

    denied! {
        lastlog_clear_denied: "lastlog -C",
        lastlog_set_denied: "lastlog -S",
        lastlog_clear_long_denied: "lastlog --clear",
        lastlog_set_long_denied: "lastlog --set",
    }
}
