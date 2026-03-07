use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static PRINTF_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "printf", policy: &PRINTF_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#printf-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        printf_format: "printf '%s\\n' hello",
        printf_number: "printf '%d' 42",
    }

    denied! {
        printf_bare_denied: "printf",
    }
}
