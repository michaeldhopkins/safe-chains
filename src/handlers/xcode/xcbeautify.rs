use crate::command::FlatDef;
use crate::verdict::SafetyLevel;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static XCBEAUTIFY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "--is-ci", "--quiet", "--quieter", "--version", "-V", "-h", "-q"]),
    valued: WordSet::flags(&["--renderer"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub static DEFS: &[FlatDef] = &[
    FlatDef {
        name: "xcbeautify",
        policy: &XCBEAUTIFY_POLICY,
        level: SafetyLevel::Inert,
        url: "https://github.com/cpisciotta/xcbeautify",
        aliases: &[],
    },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        xcbeautify_bare: "xcbeautify",
        xcbeautify_quiet: "xcbeautify --quiet",
        xcbeautify_quieter: "xcbeautify --quieter",
        xcbeautify_is_ci: "xcbeautify --is-ci",
        xcbeautify_renderer: "xcbeautify --renderer terminal",
        xcbeautify_short_q: "xcbeautify -q",
    }

    denied! {
        xcbeautify_unknown_denied: "xcbeautify --unknown-flag",
    }
}
