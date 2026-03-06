use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static CAL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--monday", "--sunday", "--three", "--year",
        "-1", "-3", "-h", "-j", "-m", "-s", "-w", "-y",
    ]),
    standalone_short: b"13hjmswy",
    valued: WordSet::new(&[
        "-A", "-B", "-d", "-n",
    ]),
    valued_short: b"ABdn",
    bare: true,
    max_positional: Some(2),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "cal", policy: &CAL_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        cal_bare: "cal",
        cal_year: "cal -y",
        cal_three: "cal -3",
    }
}
