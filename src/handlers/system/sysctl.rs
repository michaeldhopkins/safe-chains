use crate::parse::Token;
use crate::registry;
use crate::verdict::Verdict;

pub(crate) fn is_safe_sysctl(tokens: &[Token]) -> Verdict {
    if tokens[1..].iter().any(|t| t.contains('=')) {
        return Verdict::Denied;
    }
    registry::try_fallback_grammar("sysctl", tokens).unwrap_or(Verdict::Denied)
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        sysctl_help: "sysctl --help",
        sysctl_all: "sysctl -a",
        sysctl_specific: "sysctl kern.ostype",
        sysctl_n: "sysctl -n kern.ostype",
        sysctl_with_b_value: "sysctl -B 100 kern.ostype",
        sysctl_no_typecast: "sysctl -X",
        sysctl_combined_flags: "sysctl -an",
    }

    denied! {
        sysctl_bare: "sysctl",
        sysctl_write_eq: "sysctl kern.ostype=Darwin",
        sysctl_write_dash_w: "sysctl -w kern.ostype=Darwin",
        sysctl_load_p: "sysctl -p /etc/sysctl.conf",
        sysctl_system: "sysctl --system",
        sysctl_unknown_flag: "sysctl --evil",
        sysctl_value_in_eq_form: "sysctl -B=100",
    }
}
