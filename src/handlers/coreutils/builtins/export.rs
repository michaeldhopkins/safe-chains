use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static EXPORT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["-f", "-n", "-p"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Positional,
};

pub(in crate::handlers::coreutils) static FLAT_DEFS: &[FlatDef] = &[
    FlatDef { name: "export", policy: &EXPORT_POLICY, level: SafetyLevel::Inert, url: "https://man7.org/linux/man-pages/man1/export.1p.html", aliases: &[] },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        export_bare: "export",
        export_var: "export FOO=bar",
        export_multiple: "export FOO=bar BAZ=qux",
        export_name_only: "export PATH",
        export_print: "export -p",
        export_n: "export -n FOO",
        export_safe_substitution: "export FOO=$(git rev-parse HEAD)",
    }

    denied! {
        export_unsafe_substitution: "export FOO=$(rm -rf /)",
    }
}
