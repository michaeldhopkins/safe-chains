use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

static SPCTL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--assess", "--verbose",
        "-a", "-v",
    ]),
    standalone_short: b"av",
    valued: WordSet::new(&[
        "--context", "--type",
    ]),
    valued_short: b"t",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

pub fn is_safe_spctl(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    static SPCTL_SAFE: WordSet = WordSet::new(&["--assess", "-a"]);
    if !tokens[1..].iter().any(|t| SPCTL_SAFE.contains(t)) {
        return false;
    }
    policy::check(tokens, &SPCTL_POLICY)
}

pub(in crate::handlers::xcode) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    if cmd == "spctl" {
        Some(is_safe_spctl(tokens))
    } else {
        None
    }
}

pub(in crate::handlers::xcode) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("spctl",
            "Requires --assess/-a."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::xcode) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "spctl", valid_prefix: Some("spctl --assess /tmp/binary") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        spctl_assess: "spctl --assess -v /tmp/binary",
        spctl_assess_short: "spctl -a /tmp/binary",
        spctl_assess_type: "spctl --assess --type execute -v /tmp/binary",
    }

    denied! {
        spctl_add_denied: "spctl --add /tmp/binary",
        spctl_remove_denied: "spctl --remove /tmp/binary",
        spctl_enable_denied: "spctl --enable",
        spctl_master_disable_denied: "spctl --master-disable",
        spctl_no_args_denied: "spctl",
    }
}
