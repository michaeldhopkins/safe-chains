use crate::parse::{Token, WordSet};
use crate::verdict::{SafetyLevel, Verdict};
use crate::policy::{self, FlagPolicy, FlagStyle};

static SYSCTL_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--help",
        "-A", "-N", "-X", "-a", "-b", "-d", "-e", "-h",
        "-l", "-n", "-o", "-q", "-x",
    ]),
    valued: WordSet::flags(&["-B", "-r"]),
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn is_safe_sysctl(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Denied;
    }
    if tokens[1..].iter().any(|t| t.contains("=")) {
        return Verdict::Denied;
    }
        if policy::check(tokens, &SYSCTL_POLICY) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }

}

pub(in crate::handlers::system) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "sysctl" => Some(is_safe_sysctl(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::system) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("sysctl",
            "https://man7.org/linux/man-pages/man8/sysctl.8.html",
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
