use crate::command::FlatDef;
use crate::policy::{FlagPolicy, FlagStyle};
use crate::parse::WordSet;

static WAIT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "wait", policy: &WAIT_POLICY, help_eligible: false, url: "https://pubs.opengroup.org/onlinepubs/9799919799/utilities/wait.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        wait_bare: "wait",
        wait_pid: "wait 1234",
        wait_multiple_pids: "wait 1234 5678",
        wait_in_compound: "echo foo & wait",
    }

    denied! {
        wait_flag_denied: "wait --bogus",
    }
}
