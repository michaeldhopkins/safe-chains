use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static OTOOL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "-D", "-I", "-L", "-V", "-X", "-a", "-c", "-d",
        "-f", "-h", "-l", "-o", "-r", "-t", "-v", "-x",
    ]),
    standalone_short: b"DILVXacdfhlortvx",
    valued: WordSet::flags(&["-p", "-s"]),
    valued_short: b"ps",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "otool", policy: &OTOOL_POLICY, help_eligible: false, url: "https://ss64.com/mac/otool.html" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        otool_headers: "otool -h binary",
        otool_libs: "otool -L binary",
    }
}
