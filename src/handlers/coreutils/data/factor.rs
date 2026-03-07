use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static FACTOR_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--exponents",
        "-h",
    ]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "factor", policy: &FACTOR_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#factor-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        factor_number: "factor 42",
        factor_multiple: "factor 42 100",
        factor_bare: "factor",
    }
}
