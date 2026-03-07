use crate::command::{CommandDef, SubDef};
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static PLUTIL_LINT_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["-s"]),
    standalone_short: b"s",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

static PLUTIL_SIMPLE_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub(crate) static PLUTIL: CommandDef = CommandDef {
    name: "plutil",
    subs: &[
        SubDef::Policy { name: "-lint", policy: &PLUTIL_LINT_POLICY },
        SubDef::Policy { name: "-p", policy: &PLUTIL_SIMPLE_POLICY },
        SubDef::Policy { name: "-type", policy: &PLUTIL_SIMPLE_POLICY },
    ],
    bare_flags: &["-help"],
    help_eligible: true,
    url: "https://ss64.com/mac/plutil.html",
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
