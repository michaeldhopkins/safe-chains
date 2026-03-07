use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static SLEEP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "sleep", policy: &SLEEP_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#sleep-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        sleep_duration: "sleep 1",
        sleep_multiple: "sleep 1s 2s",
    }

    denied! {
        sleep_bare_denied: "sleep",
    }
}
