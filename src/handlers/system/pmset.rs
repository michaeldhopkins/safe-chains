use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};
use crate::policy::{self, FlagPolicy, FlagStyle};

static PMSET_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&["--help", "-h"]),
    valued: WordSet::flags(&[]),
    bare: true,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn is_safe_pmset(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Denied;
    }
    if tokens[1] != "-g" {
        return Verdict::Denied;
    }
        if policy::check(&tokens[2..], &PMSET_POLICY) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }

}

pub(in crate::handlers::system) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
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
