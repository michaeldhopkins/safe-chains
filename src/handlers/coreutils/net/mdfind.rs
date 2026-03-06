use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static MDFIND_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-0", "-count", "-interpret", "-literal", "-live",
    ]),
    standalone_short: b"0",
    valued: WordSet::new(&["-attr", "-name", "-onlyin", "-s"]),
    valued_short: b"s",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "mdfind", policy: &MDFIND_POLICY, help_eligible: false },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        mdfind_query: "mdfind 'kMDItemContentType == public.image'",
        mdfind_name: "mdfind -name README",
    }
}
