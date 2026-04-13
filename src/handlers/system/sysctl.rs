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
    numeric_dash: false,
};

pub(crate) fn is_safe_sysctl(tokens: &[Token]) -> Verdict {
    if tokens.len() < 2 {
        return Verdict::Denied;
    }
    if tokens[1..].iter().any(|t| t.contains("=")) {
        return Verdict::Denied;
    }
    if policy::check(tokens, &SYSCTL_POLICY) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }
}
