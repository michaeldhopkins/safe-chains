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
    use crate::docs::{CommandDoc, DocKind};
    vec![
        CommandDoc {
            name: "find",
            kind: DocKind::Handler,
            description: "Safe unless dangerous flags: -delete, -ok, -okdir, -fls, -fprint, -fprint0, -fprintf. \
                          -exec/-execdir allowed when the executed command is itself safe.",
        },
        CommandDoc {
            name: "sed",
            kind: DocKind::Handler,
            description: "Safe unless -i/--in-place flag or 'e' modifier on substitutions (executes replacement as shell command).",
        },
        CommandDoc {
            name: "sort",
            kind: DocKind::Handler,
            description: "Safe unless -o/--output or --compress-program flag.",
        },
        CommandDoc {
            name: "yq",
            kind: DocKind::Handler,
            description: "Safe unless -i/--inplace flag.",
        },
        CommandDoc {
            name: "awk / gawk / mawk / nawk",
            kind: DocKind::Handler,
            description: "Safe unless program contains system, getline, |, >, >>, or -f flag (file-based program).",
        },
        CommandDoc {
            name: "xmllint",
            kind: DocKind::Handler,
            description: "Safe unless --output flag.",
        },
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    #[test]
    fn find_name() {
        assert!(check("find . -name '*.rb'"));
    }

    #[test]
    fn find_type_name() {
        assert!(check("find . -type f -name '*.py'"));
    }

    #[test]
    fn find_maxdepth() {
        assert!(check("find /tmp -maxdepth 2"));
    }

    #[test]
    fn find_print() {
        assert!(check("find . -name '*.log' -print"));
    }

    #[test]
    fn find_print0() {
        assert!(check("find . -name '*.log' -print0"));
    }

    #[test]
    fn find_delete_denied() {
        assert!(!check("find . -name '*.tmp' -delete"));
    }

    #[test]
    fn find_exec_safe_command() {
        assert!(check("find . -name '*.rb' -exec grep -l pattern {} \\;"));
        assert!(check("find . -name '*.rb' -exec grep -l pattern {} +"));
        assert!(check("find . -exec cat {} \\;"));
    }

    #[test]
    fn find_execdir_safe_command() {
        assert!(check("find . -execdir cat {} \\;"));
        assert!(check("find . -execdir grep pattern {} \\;"));
    }

    #[test]
    fn find_exec_unsafe_denied() {
        assert!(!check("find . -exec rm {} \\;"));
        assert!(!check("find . -exec rm -rf {} +"));
    }

    #[test]
    fn find_execdir_unsafe_denied() {
        assert!(!check("find . -execdir rm {} \\;"));
    }

    #[test]
    fn find_ok_denied() {
        assert!(!check("find . -ok rm {} \\;"));
    }

    #[test]
    fn find_okdir_denied() {
        assert!(!check("find . -okdir rm {} \\;"));
    }

    #[test]
    fn find_exec_grep_safe() {
        assert!(check("find . -name '*.py' -exec grep pattern {} +"));
    }

    #[test]
    fn find_exec_nested_bash_chain_denied() {
        assert!(!check("find . -exec bash -c 'ls && rm -rf /' \\;"));
    }

    #[test]
    fn find_exec_nested_bash_safe() {
        assert!(check("find . -exec bash -c 'git status' \\;"));
    }

    #[test]
    fn find_type_delete_denied() {
        assert!(!check("find . -type f -name '*.bak' -delete"));
    }

    #[test]
    fn find_fprint_denied() {
        assert!(!check("find . -fprint /tmp/list.txt"));
    }

    #[test]
    fn find_fprint0_denied() {
        assert!(!check("find . -fprint0 /tmp/list.txt"));
    }

    #[test]
    fn find_fls_denied() {
        assert!(!check("find . -fls /tmp/list.txt"));
    }

    #[test]
    fn find_fprintf_denied() {
        assert!(!check("find . -fprintf /tmp/list.txt '%p'"));
    }

    #[test]
    fn sed_substitute() {
        assert!(check("sed 's/foo/bar/'"));
    }

    #[test]
    fn sed_n_flag() {
        assert!(check("sed -n 's/foo/bar/p'"));
    }

    #[test]
    fn sed_e_flag() {
        assert!(check("sed -e 's/foo/bar/' -e 's/baz/qux/'"));
    }

    #[test]
    fn sed_extended() {
        assert!(check("sed -E 's/[0-9]+/NUM/g'"));
    }

    #[test]
    fn sed_inplace_denied() {
        assert!(!check("sed -i 's/foo/bar/' file.txt"));
    }

    #[test]
    fn sed_in_place_long_denied() {
        assert!(!check("sed --in-place 's/foo/bar/' file.txt"));
    }

    #[test]
    fn sed_inplace_backup_denied() {
        assert!(!check("sed -i.bak 's/foo/bar/' file.txt"));
    }

    #[test]
    fn sed_ni_combined_denied() {
        assert!(!check("sed -ni 's/foo/bar/p' file.txt"));
    }

    #[test]
    fn sed_in_combined_denied() {
        assert!(!check("sed -in 's/foo/bar/' file.txt"));
    }

    #[test]
    fn sed_in_place_eq_denied() {
        assert!(!check("sed --in-place=.bak 's/foo/bar/' file.txt"));
    }

    #[test]
    fn sed_exec_modifier_denied() {
        assert!(!check("sed 's/test/touch \\/tmp\\/pwned/e'"));
    }

    #[test]
    fn sed_exec_with_global_denied() {
        assert!(!check("sed 's/foo/bar/ge'"));
    }

    #[test]
    fn sed_exec_alternate_delim_denied() {
        assert!(!check("sed 's|test|touch /tmp/pwned|e'"));
    }

    #[test]
    fn sed_exec_via_e_flag_denied() {
        assert!(!check("sed -e 's/test/touch tmp/e'"));
    }

    #[test]
    fn sed_exec_with_w_flag_denied() {
        assert!(!check("sed 's/test/cmd/we'"));
    }

    #[test]
    fn sed_standalone_e_command_denied() {
        assert!(!check("sed e"));
    }

    #[test]
    fn sed_address_e_command_denied() {
        assert!(!check("sed 1e"));
    }

    #[test]
    fn sed_regex_address_e_denied() {
        assert!(!check("sed '/pattern/e'"));
    }

    #[test]
    fn sed_range_address_e_denied() {
        assert!(!check("sed '1,5e'"));
    }

    #[test]
    fn sed_dollar_address_e_denied() {
        assert!(!check("sed '$e'"));
    }

    #[test]
    fn sed_e_via_flag_denied() {
        assert!(!check("sed -e e"));
    }

    #[test]
    fn sed_filename_starting_with_e_allowed() {
        assert!(check("sed 's/foo/bar/' error.log"));
    }

    #[test]
    fn sed_filename_ending_with_e_allowed() {
        assert!(check("sed 's/foo/bar/' Makefile"));
    }

    #[test]
    fn sed_no_exec_allowed() {
        assert!(check("sed 's/foo/bar/g'"));
    }

    #[test]
    fn sed_no_exec_print_allowed() {
        assert!(check("sed 's/foo/bar/gp'"));
    }

    #[test]
    fn sort_basic() {
        assert!(check("sort file.txt"));
    }

    #[test]
    fn sort_reverse() {
        assert!(check("sort -r file.txt"));
    }

    #[test]
    fn sort_n_u() {
        assert!(check("sort -n -u file.txt"));
    }

    #[test]
    fn sort_field() {
        assert!(check("sort -t: -k2 /etc/passwd"));
    }

    #[test]
    fn sort_output_denied() {
        assert!(!check("sort -o output.txt file.txt"));
    }

    #[test]
    fn sort_output_long_denied() {
        assert!(!check("sort --output=result.txt file.txt"));
    }

    #[test]
    fn sort_output_long_space_denied() {
        assert!(!check("sort --output result.txt file.txt"));
    }

    #[test]
    fn sort_rno_combined_denied() {
        assert!(!check("sort -rno sorted.txt file.txt"));
    }

    #[test]
    fn sort_compress_program_denied() {
        assert!(!check("sort --compress-program sh file.txt"));
    }

    #[test]
    fn sort_compress_program_eq_denied() {
        assert!(!check("sort --compress-program=gzip file.txt"));
    }

    #[test]
    fn yq_read() {
        assert!(check("yq '.key' file.yaml"));
    }

    #[test]
    fn yq_eval() {
        assert!(check("yq eval '.metadata.name' deployment.yaml"));
    }

    #[test]
    fn yq_inplace_denied() {
        assert!(!check("yq -i '.key = \"value\"' file.yaml"));
    }

    #[test]
    fn yq_inplace_long_denied() {
        assert!(!check("yq --inplace '.key = \"value\"' file.yaml"));
    }

    #[test]
    fn xmllint_read() {
        assert!(check("xmllint --xpath '//name' file.xml"));
    }

    #[test]
    fn xmllint_format() {
        assert!(check("xmllint --format file.xml"));
    }

    #[test]
    fn xmllint_output_denied() {
        assert!(!check("xmllint --output result.xml file.xml"));
    }

    #[test]
    fn xmllint_output_eq_denied() {
        assert!(!check("xmllint --output=result.xml file.xml"));
    }

    #[test]
    fn awk_print_field() {
        assert!(check("awk '{print $1}' file.txt"));
    }

    #[test]
    fn awk_print_multiple_fields() {
        assert!(check("awk '{print $1, $3}' file.txt"));
    }

    #[test]
    fn awk_field_separator() {
        assert!(check("awk -F: '{print $1}' /etc/passwd"));
    }

    #[test]
    fn awk_pattern() {
        assert!(check("awk '/error/ {print $0}' log.txt"));
    }

    #[test]
    fn awk_nr() {
        assert!(check("awk 'NR==5' file.txt"));
    }

    #[test]
    fn awk_begin_end_safe() {
        assert!(check("awk 'BEGIN{n=0} {n++} END{print n}' file.txt"));
    }

    #[test]
    fn awk_system_denied() {
        assert!(!check("awk 'BEGIN{system(\"rm -rf /\")}'"));
    }

    #[test]
    fn awk_getline_denied() {
        assert!(!check("awk '{getline line < \"/etc/shadow\"; print line}'"));
    }

    #[test]
    fn awk_pipe_output_denied() {
        assert!(!check("awk '{print $0 | \"mail user@host\"}'"));
    }

    #[test]
    fn awk_redirect_denied() {
        assert!(!check("awk '{print $0 > \"output.txt\"}'"));
    }

    #[test]
    fn awk_append_denied() {
        assert!(!check("awk '{print $0 >> \"output.txt\"}'"));
    }

    #[test]
    fn awk_file_program_denied() {
        assert!(!check("awk -f script.awk data.txt"));
    }

    #[test]
    fn gawk_safe() {
        assert!(check("gawk '{print $2}' file.txt"));
    }

    #[test]
    fn gawk_system_denied() {
        assert!(!check("gawk 'BEGIN{system(\"rm\")}'"));
    }

    #[test]
    fn awk_netstat_pipeline() {
        assert!(check("awk '{print $6}'"));
    }

    #[test]
    fn awk_string_literal_system() {
        assert!(check("awk 'BEGIN{print \"system failed\"}'"));
    }

    #[test]
    fn awk_string_literal_redirect() {
        assert!(check("awk '{print \">\"}'"));
    }

    #[test]
    fn awk_string_literal_pipe() {
        assert!(check("awk '{print \"a | b\"}'"));
    }

    #[test]
    fn awk_string_literal_getline() {
        assert!(check("awk 'BEGIN{print \"getline is a keyword\"}'"));
    }

    #[test]
    fn awk_system_call_denied() {
        assert!(!check("awk 'BEGIN{system(\"rm\")}'"));
    }

    #[test]
    fn awk_system_space_paren_denied() {
        assert!(!check("awk 'BEGIN{system (\"rm\")}'"));
    }

    #[test]
    fn awk_pipe_outside_string_denied() {
        assert!(!check("awk '{print $0 | \"cmd\"}'"));
    }

    #[test]
    fn awk_redirect_outside_string_denied() {
        assert!(!check("awk '{print $0 > \"file\"}'"));
    }

    #[test]
    fn sed_filename_1e_after_script() {
        assert!(check("sed 's/foo/bar/' 1e"));
    }

    #[test]
    fn sed_expression_flag_with_filename() {
        assert!(check("sed -e 's/foo/bar/' filename"));
    }

    #[test]
    fn sed_expression_flag_exec_denied() {
        assert!(!check("sed -e 's/foo/bar/e'"));
    }

    #[test]
    fn sed_multiple_expressions_exec_denied() {
        assert!(!check("sed -e 's/foo/bar/' -e 's/x/y/e'"));
    }

    #[test]
    fn sed_expression_flag_then_safe_filename() {
        assert!(check("sed -e 's/foo/bar/' 1e 2e"));
    }
}
