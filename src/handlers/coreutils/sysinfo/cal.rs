use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static CAL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--monday", "--sunday", "--three", "--year",
        "-1", "-3", "-h", "-j", "-m", "-s", "-w", "-y",
    ]),
    valued: WordSet::flags(&[
        "-A", "-B", "-d", "-n",
    ]),
    bare: true,
    max_positional: Some(2),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "cal", policy: &CAL_POLICY, help_eligible: false, url: "https://man7.org/linux/man-pages/man1/cal.1.html" },
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
