use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static CSRUTIL_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[]),
    standalone_short: b"",
    valued: WordSet::new(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static CSRUTIL: CommandDef = CommandDef {
    name: "csrutil",
    subs: &[
        SubDef::Policy { name: "authenticated-root", policy: &CSRUTIL_SIMPLE_POLICY },
        SubDef::Policy { name: "report", policy: &CSRUTIL_SIMPLE_POLICY },
        SubDef::Policy { name: "status", policy: &CSRUTIL_SIMPLE_POLICY },
    ],
    bare_flags: &[],
    help_eligible: true,
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        csrutil_status: "csrutil status",
        csrutil_report: "csrutil report",
        csrutil_authenticated_root: "csrutil authenticated-root",
    }
}
