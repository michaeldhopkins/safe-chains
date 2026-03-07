use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static DATE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--rfc-2822", "--rfc-email", "--universal", "--utc",
        "-R", "-j", "-n", "-u",
    ]),
    valued: WordSet::flags(&[
        "--date", "--iso-8601", "--reference", "--rfc-3339",
        "-I", "-d", "-f", "-r", "-v", "-z",
    ]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "date", policy: &DATE_POLICY, help_eligible: false, url: "https://www.gnu.org/software/coreutils/manual/coreutils.html#date-invocation" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        date_bare: "date",
        date_format: "date '+%Y-%m-%d'",
        date_utc: "date -u",
        date_reference: "date -r file.txt",
    }

    denied! {
        date_set_denied: "date -s '2025-01-01'",
        date_set_long_denied: "date --set='2025-01-01'",
        date_set_long_space_denied: "date --set '2025-01-01'",
    }
}
