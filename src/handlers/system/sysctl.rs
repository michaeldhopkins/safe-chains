use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

static SYSCTL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "-A", "-N", "-X", "-a", "-b", "-d", "-e", "-h",
        "-l", "-n", "-o", "-q", "-x",
    ]),
    standalone_short: b"ANXabdehlnoqx",
    valued: WordSet::new(&["-B", "-r"]),
    valued_short: b"Br",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn is_safe_sysctl(tokens: &[Token]) -> bool {
    if tokens.len() < 2 {
        return false;
    }
    if tokens[1..].iter().any(|t| t.contains("=")) {
        return false;
    }
    policy::check(tokens, &SYSCTL_POLICY)
}

pub(in crate::handlers::system) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "sysctl" => Some(is_safe_sysctl(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::system) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("sysctl",
            "Read-only usage."),
    ]
}

#[cfg(test)]
pub(in crate::handlers::system) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "sysctl", valid_prefix: Some("sysctl kern.maxproc") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        sysctl_read: "sysctl kern.maxproc",
        sysctl_all: "sysctl -a",
        sysctl_names: "sysctl -N -a",
    }

    denied! {
        sysctl_write_denied: "sysctl -w kern.maxproc=2048",
        sysctl_write_long_denied: "sysctl --write kern.maxproc=2048",
        sysctl_assign_denied: "sysctl kern.maxproc=2048",
    }
}
