use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static UPTIME_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--pretty", "--since", "-p", "-s"]),
    standalone_short: b"ps",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "uptime", policy: &UPTIME_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#uptime-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        uptime_bare: "uptime",
        uptime_pretty: "uptime -p",
    }
}
