use crate::parse::{Segment, Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

fn expr_has_exec(token: &Token) -> bool {
    let bytes = token.as_bytes();
    if bytes == b"e"
        || (bytes.last() == Some(&b'e')
            && bytes.len() >= 2
            && matches!(bytes[bytes.len() - 2], b'0'..=b'9' | b'/' | b'$'))
    {
        return true;
    }
    if bytes.len() < 4 || bytes[0] != b's' {
        return false;
    }
    let delim = bytes[1];
    let mut count = 0;
    let mut escaped = false;
    let mut flags_start = None;
    for (i, &b) in bytes[2..].iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if b == b'\\' {
            escaped = true;
            continue;
        }
        if b == delim {
            count += 1;
            if count == 2 {
                flags_start = Some(i + 3);
                break;
            }
        }
    }
    flags_start.is_some_and(|start| start < bytes.len() && bytes[start..].contains(&b'e'))
}

fn sed_has_exec_modifier(tokens: &[Token]) -> bool {
    let mut i = 1;
    let mut saw_script = false;

    while i < tokens.len() {
        let token = &tokens[i];

        if *token == "-e" || *token == "--expression" {
            if tokens.get(i + 1).is_some_and(expr_has_exec) {
                return true;
            }
            saw_script = true;
            i += 2;
            continue;
        }

        if token.starts_with("-") {
            i += 1;
            continue;
        }

        if !saw_script {
            if expr_has_exec(token) {
                return true;
            }
            saw_script = true;
        }
        i += 1;
    }
    false
}

static SED_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::new(&[
        "--debug", "--posix", "--quiet", "--sandbox",
        "--silent", "--unbuffered",
        "-E", "-n", "-r", "-u", "-z",
    ]),
    standalone_short: b"Enruz",
    valued: WordSet::new(&[
        "--expression", "--file", "--line-length",
        "-e", "-f", "-l",
    ]),
    valued_short: b"efl",
    bare: false,
    max_positional: None,
    flag_style: FlagStyle::Strict,
};

fn is_safe_sed(tokens: &[Token]) -> bool {
    !sed_has_exec_modifier(tokens) && policy::check(tokens, &SED_POLICY)
}

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token], _is_safe: &dyn Fn(&Segment) -> bool) -> Option<bool> {
    match cmd {
        "sed" => Some(is_safe_sed(tokens)),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("sed", format!("{}\n- Inline expressions validated for safety", SED_POLICY.describe())),
    ]
}

#[cfg(test)]
pub(in crate::handlers::coreutils) const REGISTRY: &[crate::handlers::CommandEntry] = &[
    crate::handlers::CommandEntry::Custom { cmd: "sed", valid_prefix: Some("sed 's/a/b/'") },
];

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    fn check(cmd: &str) -> bool { is_safe_command(cmd) }

    safe! {
        sed_substitute: "sed 's/foo/bar/'",
        sed_n_flag: "sed -n 's/foo/bar/p'",
        sed_e_flag: "sed -e 's/foo/bar/' -e 's/baz/qux/'",
        sed_extended: "sed -E 's/[0-9]+/NUM/g'",
        sed_filename_starting_with_e_allowed: "sed 's/foo/bar/' error.log",
        sed_filename_ending_with_e_allowed: "sed 's/foo/bar/' Makefile",
        sed_no_exec_allowed: "sed 's/foo/bar/g'",
        sed_no_exec_print_allowed: "sed 's/foo/bar/gp'",
        sed_filename_1e_after_script: "sed 's/foo/bar/' 1e",
        sed_expression_flag_with_filename: "sed -e 's/foo/bar/' filename",
        sed_expression_flag_then_safe_filename: "sed -e 's/foo/bar/' 1e 2e",
    }

    denied! {
        sed_inplace_denied: "sed -i 's/foo/bar/' file.txt",
        sed_in_place_long_denied: "sed --in-place 's/foo/bar/' file.txt",
        sed_inplace_backup_denied: "sed -i.bak 's/foo/bar/' file.txt",
        sed_ni_combined_denied: "sed -ni 's/foo/bar/p' file.txt",
        sed_in_combined_denied: "sed -in 's/foo/bar/' file.txt",
        sed_in_place_eq_denied: "sed --in-place=.bak 's/foo/bar/' file.txt",
        sed_exec_modifier_denied: "sed 's/test/touch \\/tmp\\/pwned/e'",
        sed_exec_with_global_denied: "sed 's/foo/bar/ge'",
        sed_exec_alternate_delim_denied: "sed 's|test|touch /tmp/pwned|e'",
        sed_exec_via_e_flag_denied: "sed -e 's/test/touch tmp/e'",
        sed_exec_with_w_flag_denied: "sed 's/test/cmd/we'",
        sed_standalone_e_command_denied: "sed e",
        sed_address_e_command_denied: "sed 1e",
        sed_regex_address_e_denied: "sed '/pattern/e'",
        sed_range_address_e_denied: "sed '1,5e'",
        sed_dollar_address_e_denied: "sed '$e'",
        sed_e_via_flag_denied: "sed -e e",
        sed_expression_flag_exec_denied: "sed -e 's/foo/bar/e'",
        sed_multiple_expressions_exec_denied: "sed -e 's/foo/bar/' -e 's/x/y/e'",
        sed_inplace_trailing_help_denied: "sed -i 's/foo/bar/' file --help",
        sed_inplace_trailing_version_denied: "sed -i 's/foo/bar/' file --version",
    }
}
