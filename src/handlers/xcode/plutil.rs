use crate::command::{CommandDef, SubDef};
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static PLUTIL_LINT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h", "-s"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static PLUTIL_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static PLUTIL: CommandDef = CommandDef {
    name: "plutil",
    subs: &[
        SubDef::Policy { name: "-lint", policy: &PLUTIL_LINT_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "-p", policy: &PLUTIL_SIMPLE_POLICY, level: SafetyLevel::Inert },
        SubDef::Policy { name: "-type", policy: &PLUTIL_SIMPLE_POLICY, level: SafetyLevel::Inert },
    ],
    bare_flags: &["--help", "--version", "-V", "-h", "-help"],
    url: "https://ss64.com/mac/plutil.html",
    aliases: &[],
};

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        plutil_lint: "plutil -lint file.plist",
        plutil_lint_silent: "plutil -lint -s file.plist",
        plutil_print: "plutil -p file.plist",
        plutil_type: "plutil -type keypath file.plist",
        plutil_help: "plutil -help",
    }
}
