use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

fn awk_has_dangerous_construct(token: &Token) -> bool {
    let code = token.content_outside_double_quotes();
    code.contains("system") || code.contains("getline") || code.contains('|') || code.contains('>')
}

static AWK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--characters-as-bytes", "--copyright", "--gen-pot",
        "--lint", "--no-optimize", "--optimize",
        "--posix", "--re-interval", "--sandbox",
        "--traditional", "--use-lc-numeric", "--version",
        "-C", "-N", "-O", "-P", "-S", "-V",
        "-b", "-c", "-g", "-r", "-s", "-t",
    ]),
    standalone_short: b"CNOPSVbcgrst",
    valued: WordSet::new(&[
        "--assign", "--field-separator",
        "-F", "-v",
    ]),
    valued_short: b"Fv",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn is_safe_awk(tokens: &[Token]) -> bool {
    for token in &tokens[1..] {
        if !token.starts_with("-") && awk_has_dangerous_construct(token) {
            return false;
        }
    }
    policy::check(tokens, &AWK_POLICY)
}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "awk" | "gawk" | "mawk" | "nawk" => Some(is_safe_awk(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("awk / gawk / mawk / nawk",
            format!("- Program validated: system, getline, |, > constructs checked\n{}", AWK_POLICY.describe())),
    ]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "awk", valid_prefix: Some("awk '{print}'") },
    crate::handlers::CommandEntry::Custom { cmd: "gawk", valid_prefix: Some("gawk '{print}'") },
    crate::handlers::CommandEntry::Custom { cmd: "mawk", valid_prefix: Some("mawk '{print}'") },
    crate::handlers::CommandEntry::Custom { cmd: "nawk", valid_prefix: Some("nawk '{print}'") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        awk_print_field: "awk '{print $1}' file.txt",
        awk_print_multiple_fields: "awk '{print $1, $3}' file.txt",
        awk_field_separator: "awk -F: '{print $1}' /etc/passwd",
        awk_pattern: "awk '/error/ {print $0}' log.txt",
        awk_nr: "awk 'NR==5' file.txt",
        awk_begin_end_safe: "awk 'BEGIN{n=0} {n++} END{print n}' file.txt",
        gawk_safe: "gawk '{print $2}' file.txt",
        awk_netstat_pipeline: "awk '{print $6}'",
        awk_string_literal_system: "awk 'BEGIN{print \"system failed\"}'",
        awk_string_literal_redirect: "awk '{print \">\"}'",
        awk_string_literal_pipe: "awk '{print \"a | b\"}'",
        awk_string_literal_getline: "awk 'BEGIN{print \"getline is a keyword\"}'",
    }

    denied! {
        awk_system_denied: "awk 'BEGIN{system(\"rm -rf /\")}'",
        awk_getline_denied: "awk '{getline line < \"/etc/shadow\"; print line}'",
        awk_pipe_output_denied: "awk '{print $0 | \"mail user@host\"}'",
        awk_redirect_denied: "awk '{print $0 > \"output.txt\"}'",
        awk_append_denied: "awk '{print $0 >> \"output.txt\"}'",
        awk_file_program_denied: "awk -f script.awk data.txt",
        gawk_system_denied: "gawk 'BEGIN{system(\"rm\")}'",
        awk_system_call_denied: "awk 'BEGIN{system(\"rm\")}'",
        awk_system_space_paren_denied: "awk 'BEGIN{system (\"rm\")}'",
        awk_pipe_outside_string_denied: "awk '{print $0 | \"cmd\"}'",
        awk_redirect_outside_string_denied: "awk '{print $0 > \"file\"}'",
        awk_system_trailing_help_denied: "awk 'BEGIN{system(\"rm\")}' --help",
        awk_system_trailing_version_denied: "awk 'BEGIN{system(\"rm\")}' --version",
    }
}
