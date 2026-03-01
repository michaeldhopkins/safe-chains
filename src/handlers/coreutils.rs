use crate::parse::{Segment, Token, WordSet, has_flag};

static FIND_DANGEROUS_FLAGS: WordSet = WordSet::new(&[
    "-delete",
    "-fls",
    "-fprint",
    "-fprint0",
    "-fprintf",
    "-ok",
    "-okdir",
]);

pub fn is_safe_find(tokens: &[Token], is_safe: &dyn Fn(&Segment) -> bool) -> bool {
    let mut i = 1;
    while i < tokens.len() {
        if FIND_DANGEROUS_FLAGS.contains(&tokens[i]) {
            return false;
        }
        if tokens[i] == "-exec" || tokens[i] == "-execdir" {
            let cmd_start = i + 1;
            let cmd_end = tokens[cmd_start..]
                .iter()
                .position(|t| *t == ";" || *t == "+")
                .map(|p| cmd_start + p)
                .unwrap_or(tokens.len());
            if cmd_start >= cmd_end {
                return false;
            }
            let exec_cmd = Segment::from_tokens_replacing(&tokens[cmd_start..cmd_end], "{}", "file");
            if !is_safe(&exec_cmd) {
                return false;
            }
            i = cmd_end + 1;
            continue;
        }
        i += 1;
    }
    true
}

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

pub fn is_safe_sed(tokens: &[Token]) -> bool {
    !has_flag(tokens, Some("-i"), Some("--in-place")) && !sed_has_exec_modifier(tokens)
}

pub fn is_safe_sort(tokens: &[Token]) -> bool {
    !has_flag(tokens, Some("-o"), Some("--output"))
        && !has_flag(tokens, None, Some("--compress-program"))
}

pub fn is_safe_yq(tokens: &[Token]) -> bool {
    !has_flag(tokens, Some("-i"), Some("--inplace"))
}

pub fn is_safe_xmllint(tokens: &[Token]) -> bool {
    !has_flag(tokens, None, Some("--output"))
}

fn awk_has_dangerous_construct(token: &Token) -> bool {
    let code = token.content_outside_double_quotes();
    code.contains("system") || code.contains("getline") || code.contains('|') || code.contains('>')
}

pub fn is_safe_awk(tokens: &[Token]) -> bool {
    if has_flag(tokens, Some("-f"), None) {
        return false;
    }
    for token in &tokens[1..] {
        if token.starts_with("-") {
            continue;
        }
        if awk_has_dangerous_construct(token) {
            return false;
        }
    }
    true
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("find",
            "Safe unless dangerous flags: -delete, -ok, -okdir, -fls, -fprint, -fprint0, -fprintf. \
             -exec/-execdir allowed when the executed command is itself safe."),
        CommandDoc::handler("sed",
            "Safe unless -i/--in-place flag or 'e' modifier on substitutions (executes replacement as shell command)."),
        CommandDoc::handler("sort",
            "Safe unless -o/--output or --compress-program flag."),
        CommandDoc::handler("yq",
            "Safe unless -i/--inplace flag."),
        CommandDoc::handler("awk / gawk / mawk / nawk",
            "Safe unless program contains system, getline, |, >, >>, or -f flag (file-based program)."),
        CommandDoc::handler("xmllint",
            "Safe unless --output flag."),
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        find_name: "find . -name '*.rb'",
        find_type_name: "find . -type f -name '*.py'",
        find_maxdepth: "find /tmp -maxdepth 2",
        find_print: "find . -name '*.log' -print",
        find_print0: "find . -name '*.log' -print0",
        find_exec_grep_l: "find . -name '*.rb' -exec grep -l pattern {} \\;",
        find_exec_grep_l_plus: "find . -name '*.rb' -exec grep -l pattern {} +",
        find_exec_cat: "find . -exec cat {} \\;",
        find_execdir_cat: "find . -execdir cat {} \\;",
        find_execdir_grep: "find . -execdir grep pattern {} \\;",
        find_exec_grep_safe: "find . -name '*.py' -exec grep pattern {} +",
        find_exec_nested_bash_safe: "find . -exec bash -c 'git status' \\;",
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
        sort_basic: "sort file.txt",
        sort_reverse: "sort -r file.txt",
        sort_n_u: "sort -n -u file.txt",
        sort_field: "sort -t: -k2 /etc/passwd",
        yq_read: "yq '.key' file.yaml",
        yq_eval: "yq eval '.metadata.name' deployment.yaml",
        xmllint_read: "xmllint --xpath '//name' file.xml",
        xmllint_format: "xmllint --format file.xml",
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
        find_delete_denied: "find . -name '*.tmp' -delete",
        find_exec_rm: "find . -exec rm {} \\;",
        find_exec_rm_rf: "find . -exec rm -rf {} +",
        find_execdir_unsafe_denied: "find . -execdir rm {} \\;",
        find_ok_denied: "find . -ok rm {} \\;",
        find_okdir_denied: "find . -okdir rm {} \\;",
        find_exec_nested_bash_chain_denied: "find . -exec bash -c 'ls && rm -rf /' \\;",
        find_type_delete_denied: "find . -type f -name '*.bak' -delete",
        find_fprint_denied: "find . -fprint /tmp/list.txt",
        find_fprint0_denied: "find . -fprint0 /tmp/list.txt",
        find_fls_denied: "find . -fls /tmp/list.txt",
        find_fprintf_denied: "find . -fprintf /tmp/list.txt '%p'",
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
        sort_output_denied: "sort -o output.txt file.txt",
        sort_output_long_denied: "sort --output=result.txt file.txt",
        sort_output_long_space_denied: "sort --output result.txt file.txt",
        sort_rno_combined_denied: "sort -rno sorted.txt file.txt",
        sort_compress_program_denied: "sort --compress-program sh file.txt",
        sort_compress_program_eq_denied: "sort --compress-program=gzip file.txt",
        yq_inplace_denied: "yq -i '.key = \"value\"' file.yaml",
        yq_inplace_long_denied: "yq --inplace '.key = \"value\"' file.yaml",
        xmllint_output_denied: "xmllint --output result.xml file.xml",
        xmllint_output_eq_denied: "xmllint --output=result.xml file.xml",
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
    }
}
