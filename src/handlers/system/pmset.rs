use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

static PMSET_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[]),
    standalone_short: b"",
    valued: WordSet::flags(&[]),
    valued_short: b"",
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn is_safe_pmset(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if tokens[1] != "-g" {
        return false;
    }
    policy::check(&tokens[2..], &PMSET_POLICY)
}

pub(in crate::handlers::system) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "pmset" => Some(is_safe_pmset(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::system) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("pmset",
            "https://ss64.com/mac/pmset.html",
            "Allowed: -g (get/display settings only)."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::system) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Positional { cmd: "pmset" },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        pmset_get: "pmset -g",
        pmset_get_assertions: "pmset -g assertions",
        pmset_get_batt: "pmset -g batt",
    }

    denied! {
        pmset_sleep_denied: "pmset sleepnow",
        pmset_set_denied: "pmset -a displaysleep 10",
        bare_pmset_denied: "pmset",
    }
}
