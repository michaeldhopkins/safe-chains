use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static IOTOP_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--accumulated", "--batch", "--kilobytes", "--only",
        "--processes", "--quiet",
        "-P", "-a", "-b", "-k", "-o", "-q", "-t",
    ]),
    standalone_short: b"Pabkoqt",
    valued: WordSet::new(&[
        "--delay", "--iter", "--pid", "--user",
        "-d", "-n", "-p", "-u",
    ]),
    valued_short: b"dnpu",
    bare: true,
    max_positional: Some(0),
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "iotop", policy: &IOTOP_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        iotop_batch: "iotop -b -n 1",
    }
}
