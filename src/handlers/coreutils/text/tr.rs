use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static TR_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--complement", "--delete", "--squeeze-repeats", "--truncate-set1",
        "-C", "-c", "-d", "-s",
    ]),
    standalone_short: b"Ccds",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "tr", policy: &TR_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#tr-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        tr_lower: "tr A-Z a-z",
        tr_delete: "tr -d '\\n'",
        tr_squeeze: "tr -s ' '",
    }
}
