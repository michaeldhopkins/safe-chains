use crate::verdict::{SafetyLevel, Verdict};
use crate::parse::{Token, WordSet};
use crate::policy::{self, FlagPolicy, FlagStyle};

fn strip_regex_literals(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' {
            result.push(b' ');
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'/' {
                    i += 1;
                    break;
                }
                i += 1;
            }
        } else {
            result.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(result).unwrap_or_default()
}

fn has_redirect(code: &str) -> bool {
    let bytes = code.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'>' && !(i + 1 < bytes.len() && bytes[i + 1] == b'=') {
            return true;
        }
    }
    false
}

fn awk_has_dangerous_construct(token: &Token) -> bool {
    let code = token.content_outside_double_quotes();
    if code.contains("system") || code.contains("getline") {
        return true;
    }
    let stripped = strip_regex_literals(&code);
    stripped.contains('|') || has_redirect(&stripped)
}

static AWK_POLICY: FlagPolicy = FlagPolicy {
    standalone: WordSet::flags(&[
        "--characters-as-bytes", "--copyright", "--gen-pot",
        "--lint", "--no-optimize", "--optimize",
        "--posix", "--re-interval", "--sandbox",
        "--traditional", "--use-lc-numeric", "--version",
        "-C", "-N", "-O", "-P", "-S", "-V",
        "-b", "-c", "-g", "-r", "-s", "-t",
    ]),
    valued: WordSet::flags(&[
        "--assign", "--field-separator",
        "-F", "-v",
    ]),
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

pub(in crate::handlers::coreutils) fn dispatch(cmd: &str, tokens: &[Token]) -> Option<Verdict> {
    match cmd {
        "awk" | "gawk" | "mawk" | "nawk" => Some(if is_safe_awk(tokens) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }),
        _ => None,
    }
}

pub(in crate::handlers::coreutils) fn command_docs() -> Vec<crate::docs::CommandDoc> {
    vec![
        crate::docs::CommandDoc::handler("awk / gawk / mawk / nawk",
            "https://www.gnu.org/software/gawk/manual/gawk.html",
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
        awk_comparison_gte: "awk 'NR>=10 {print}' file.txt",
        awk_comparison_gte_complex: "awk '{if(length($0)>=80) print NR\": \"$0}' file.txt",
        awk_multiple_comparisons: "awk 'NR>=5 && NR<=20' file.txt",
        awk_division: "awk '{print $1/100}' file.txt",
        awk_multiple_divisions: "awk '{avg=$1/10; pct=avg/total*100; print pct}' file.txt",
        awk_modulo_and_division: "awk '{print $1%10, $1/10}' file.txt",

        awk_string_literal_system: "awk 'BEGIN{print \"system failed\"}'",
        awk_string_literal_redirect: "awk '{print \">\"}'",
        awk_string_literal_pipe: "awk '{print \"a | b\"}'",
        awk_string_literal_getline: "awk 'BEGIN{print \"getline is a keyword\"}'",

        awk_regex_alternation: "awk '/foo|bar/ {print}' file.txt",
        awk_regex_multi_alt: "awk '/^def |^class |^end/ {print}' file.rb",
        awk_regex_redirect_char: "awk '/a>b/ {print}' file.txt",
        awk_regex_complex: "awk '/^  def /{m=$0; l=NR} NR-l>=10 && /^  def |^class |^end/{print}' file.rb",
        awk_regex_single_char: "awk '/^#/ {print}' file.txt",
        awk_regex_escaped_slash: "awk '/path\\/to/ {print}' file.txt",
        awk_regex_pipe_and_gte: "awk '/error|warning/ && NR>=10 {print}' log.txt",
        awk_regex_empty: "awk '/^$/ {print NR}' file.txt",
        awk_regex_pipe_in_match: "awk '$0 ~ /foo|bar/ {print}' file.txt",
        awk_regex_multiple_patterns: "awk '/start/,/end/ {print}' file.txt",
        awk_regex_redirect_in_char_class: "awk '/[><=]/ {print}' file.txt",
        awk_regex_pipe_in_char_class: "awk '/[|&]/ {print}' file.txt",
        awk_regex_mixed_with_math: "awk '/error|warn/ {c++} END{print c>=0 ? c : 0}' log.txt",
        awk_no_program_just_flag: "awk --version",
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
        awk_system_between_division_denied: "awk '{x=1/2;system(\"rm\");y=3/4}' file",
        awk_getline_in_regex_context_denied: "awk '/foo/ {getline; print}' file",
        awk_getline_from_cmd_denied: "awk 'BEGIN{cmd=\"date\"; cmd | getline d; print d}'",
        awk_pipe_bare_denied: "awk '{cmd=\"sort\"; print $0 | cmd}' file",
        awk_redirect_bare_var_denied: "awk '{f=\"out.txt\"; print $0 > f}' file",
        awk_system_in_function_denied: "awk 'function run(){system(\"rm\")} BEGIN{run()}'",
        awk_getline_from_pipe_denied: "awk 'BEGIN{\"date\" | getline d; print d}'",
        awk_append_bare_denied: "awk '{print >> \"log.txt\"}' file",
        awk_redirect_no_space_denied: "awk '{print >\"out\"}' file",
        awk_pipe_no_space_denied: "awk '{print|\"cmd\"}' file",
    }
}
