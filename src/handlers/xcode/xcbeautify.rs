use crate::command::FlatDef;
use crate::parse::WordSet;
use crate::policy::{FlagPolicy, FlagStyle};

static XCBEAUTIFY_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--is-ci", "--quiet", "--quieter", "-q"]),
    valued: WordSet::flags(&["--renderer"]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub static DEFS: &[FlatDef] = &[
    FlatDef {
        name: "xcbeautify",
        policy: &XCBEAUTIFY_POLICY,
        help_eligible: false,
        url: "https://github.com/cpisciotta/xcbeautify",
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
