use crate::parse::{Token, WordSet};

static SAFE_PERL_WORDS: WordSet = WordSet::new(&[
    "ARGV", "BEGIN", "END", "STDERR", "STDIN", "STDOUT",
    "abs", "and", "atan2",
    "chomp", "chop", "chr", "close", "cmp", "cos",
    "defined", "delete", "die",
    "each", "else", "elsif", "eof", "eq", "exists", "exp",
    "for", "foreach",
    "ge", "grep", "gt",
    "hex",
    "if", "int",
    "join",
    "keys",
    "last", "lc", "lcfirst", "le", "length", "local", "log", "lt",
    "map", "my",
    "ne", "next", "no", "not",
    "oct", "or", "ord", "our",
    "pack", "pop", "pos", "print", "printf", "push",
    "qq", "qr", "qw",
    "ref", "return", "reverse", "rindex",
    "say", "scalar", "shift", "sin", "sort", "splice", "split", "sprintf", "sqrt", "substr",
    "tell", "tr",
    "uc", "ucfirst", "undef", "unless", "unpack", "unshift", "until",
    "values",
    "wantarray", "warn", "while",
]);

fn closing_delimiter(open: u8) -> u8 {
    match open {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        b'<' => b'>',
        _ => open,
    }
}

fn is_paired_delimiter(b: u8) -> bool {
    matches!(b, b'(' | b'[' | b'{' | b'<')
}

fn skip_delimited(bytes: &[u8], start: usize) -> Option<usize> {
    if start >= bytes.len() {
        return None;
    }
    let open = bytes[start];
    let close = closing_delimiter(open);
    let paired = is_paired_delimiter(open);
    let mut depth = 1u32;
    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if paired {
            if bytes[i] == open {
                depth += 1;
            } else if bytes[i] == close {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
        } else if bytes[i] == close {
            return Some(i + 1);
        }
        i += 1;
    }
    None
}

fn skip_regex_body(bytes: &[u8], start: usize, sections: usize) -> Option<(usize, &[u8])> {
    if start >= bytes.len() {
        return None;
    }
    let delim = bytes[start];
    let mut pos = start;

    if is_paired_delimiter(delim) {
        for _ in 0..sections {
            if pos >= bytes.len() {
                return None;
            }
            pos = skip_delimited(bytes, pos)?;
        }
    } else {
        pos += 1;
        for _ in 0..sections {
            loop {
                if pos >= bytes.len() {
                    return None;
                }
                if bytes[pos] == b'\\' {
                    pos += 2;
                    continue;
                }
                if bytes[pos] == delim {
                    pos += 1;
                    break;
                }
                pos += 1;
            }
        }
    }
    let flags_start = pos;
    while pos < bytes.len() && bytes[pos].is_ascii_alphabetic() {
        pos += 1;
    }
    Some((pos, &bytes[flags_start..pos]))
}

fn at_word_boundary(bytes: &[u8], i: usize) -> bool {
    i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_')
}

fn strip_regex_content(code: &str) -> String {
    let bytes = code.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b's'
            && at_word_boundary(bytes, i)
            && i + 1 < bytes.len()
            && !bytes[i + 1].is_ascii_alphanumeric()
            && bytes[i + 1] != b'_'
            && let Some((end, _)) = skip_regex_body(bytes, i + 1, 2)
        {
            result.push(b's');
            i = end;
            continue;
        }

        if (bytes[i] == b'm' || bytes[i] == b'y')
            && at_word_boundary(bytes, i)
            && i + 1 < bytes.len()
            && !bytes[i + 1].is_ascii_alphanumeric()
            && bytes[i + 1] != b'_'
            && let Some((end, _)) = skip_regex_body(bytes, i + 1, if bytes[i] == b'y' { 2 } else { 1 })
        {
            result.push(bytes[i]);
            i = end;
            continue;
        }

        if i + 1 < bytes.len()
            && bytes[i] == b't'
            && at_word_boundary(bytes, i)
            && bytes[i + 1] == b'r'
            && i + 2 < bytes.len()
            && !bytes[i + 2].is_ascii_alphanumeric()
            && bytes[i + 2] != b'_'
            && let Some((end, _)) = skip_regex_body(bytes, i + 2, 2)
        {
            result.extend_from_slice(b"tr");
            i = end;
            continue;
        }

        if bytes[i] == b'/' && is_regex_context(&result) {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() {
                if bytes[j] == b'\\' {
                    j += 2;
                    continue;
                }
                if bytes[j] == b'/' {
                    j += 1;
                    while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                        j += 1;
                    }
                    i = j;
                    break;
                }
                j += 1;
            }
            if i != j {
                i = bytes.len();
            }
            continue;
        }

        result.push(bytes[i]);
        i += 1;
    }

    String::from_utf8(result).unwrap_or_default()
}

fn is_regex_context(preceding: &[u8]) -> bool {
    let end = match preceding.iter().rposition(|b| !b.is_ascii_whitespace()) {
        Some(pos) => pos,
        None => return true,
    };
    let last = preceding[end];
    if matches!(last, b'~' | b'(' | b',' | b';' | b'!' | b'&' | b'|' | b'?' | b':' | b'=' | b'{') {
        return true;
    }
    if last.is_ascii_alphabetic() || last == b'_' {
        let start = preceding[..=end]
            .iter()
            .rposition(|b| !b.is_ascii_alphanumeric() && *b != b'_')
            .map(|p| p + 1)
            .unwrap_or(0);
        let word = std::str::from_utf8(&preceding[start..=end]).unwrap_or("");
        return matches!(
            word,
            "if" | "unless" | "while" | "until" | "and" | "or" | "not" | "for" | "foreach" | "return"
        );
    }
    false
}

fn has_substitution_eval(code: &str) -> bool {
    let bytes = code.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b's'
            && at_word_boundary(bytes, i)
            && i + 1 < bytes.len()
            && !bytes[i + 1].is_ascii_alphanumeric()
            && bytes[i + 1] != b'_'
            && let Some((_, flags)) = skip_regex_body(bytes, i + 1, 2)
            && flags.contains(&b'e')
        {
            return true;
        }
        i += 1;
    }
    false
}

fn perl_code_is_safe(token: &Token) -> bool {
    let no_strings = token.content_outside_double_quotes();
    if no_strings.contains('`') {
        return false;
    }
    if has_substitution_eval(&no_strings) {
        return false;
    }
    let stripped = strip_regex_content(&no_strings);
    let bytes = stripped.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' || bytes[i] == b'@' || bytes[i] == b'%' {
            i += 1;
            if i < bytes.len() && (bytes[i] == b'_' || bytes[i].is_ascii_alphabetic()) {
                while i < bytes.len() && (bytes[i] == b'_' || bytes[i].is_ascii_alphanumeric()) {
                    i += 1;
                }
            }
            continue;
        }
        if bytes[i] == b'_' || bytes[i].is_ascii_alphabetic() {
            let start = i;
            while i < bytes.len() && (bytes[i] == b'_' || bytes[i].is_ascii_alphanumeric()) {
                i += 1;
            }
            let word = &stripped[start..i];
            if word.len() > 1 && !SAFE_PERL_WORDS.contains(word) {
                return false;
            }
            continue;
        }
        i += 1;
    }
    true
}

pub fn is_safe_perl(tokens: &[Token]) -> bool {
    if tokens.len() == 2 && tokens[1].is_one_of(&["--version", "--help", "-v", "-V"]) {
        return true;
    }

    let mut has_code = false;
    let mut i = 1;
    while i < tokens.len() {
        let token = &tokens[i];
        if token.starts_with("--") || !token.starts_with("-") {
            i += 1;
            continue;
        }
        let flags = &token.as_str()[1..];
        if flags.len() > 1 && matches!(flags.as_bytes()[0], b'M' | b'm' | b'I') {
            i += 1;
            continue;
        }
        if *token == "-M" || *token == "-m" || *token == "-I" {
            i += 2;
            continue;
        }
        if flags.ends_with('e') || flags.ends_with('E') {
            let before_e = &flags[..flags.len() - 1];
            if before_e.contains('i') {
                return false;
            }
            has_code = true;
            if tokens.get(i + 1).is_some_and(|t| !perl_code_is_safe(t)) {
                return false;
            }
            i += 2;
            continue;
        }
        if flags.contains('i') {
            return false;
        }
        i += 1;
    }
    has_code
}

pub fn command_docs() -> Vec<crate::docs::CommandDoc> {
    use crate::docs::CommandDoc;
    vec![
        CommandDoc::handler("perl",
            "Allowed: -e/-E inline one-liners with safe code, --version, --help, -v, -V. \
             Blocked: script files (no -e/-E), -i (in-place edit), s///e modifier, backticks, \
             and code containing identifiers not in the safe built-in allowlist."),
    ]
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    #[test]
    fn perl_version() {
        assert!(check("perl --version"));
    }

    #[test]
    fn perl_help() {
        assert!(check("perl --help"));
    }

    #[test]
    fn perl_v() {
        assert!(check("perl -v"));
    }

    #[test]
    fn perl_big_v() {
        assert!(check("perl -V"));
    }

    #[test]
    fn perl_print_hello() {
        assert!(check("perl -e 'print \"hello\\n\"'"));
    }

    #[test]
    fn perl_say() {
        assert!(check("perl -E 'say \"hello\"'"));
    }

    #[test]
    fn perl_ne_grep() {
        assert!(check("perl -ne 'print if /pattern/' file.txt"));
    }

    #[test]
    fn perl_pe_substitute() {
        assert!(check("perl -pe 's/foo/bar/g' file.txt"));
    }

    #[test]
    fn perl_lane_field() {
        assert!(check("perl -lane 'print $F[0]' file.txt"));
    }

    #[test]
    fn perl_chomp_split_join() {
        assert!(check("perl -ne 'chomp; print join(\",\", split(/\\t/)), \"\\n\"'"));
    }

    #[test]
    fn perl_tr_transliterate() {
        assert!(check("perl -pe 'tr/a-z/A-Z/' file.txt"));
    }

    #[test]
    fn perl_begin_end_count() {
        assert!(check("perl -ne 'BEGIN{$n=0} $n++; END{print $n}'"));
    }

    #[test]
    fn perl_ne_word_pattern() {
        assert!(check("perl -ne 'print if /\\berror\\b/' log.txt"));
    }

    #[test]
    fn perl_my_variable() {
        assert!(check("perl -e 'my $x = 1; print $x'"));
    }

    #[test]
    fn perl_keys_values() {
        assert!(check("perl -e 'my %h; print keys %h'"));
    }

    #[test]
    fn perl_string_containing_system() {
        assert!(check("perl -e 'print \"system is down\\n\"'"));
    }

    #[test]
    fn perl_substitute_alternate_delim() {
        assert!(check("perl -pe 's{error_count}{warning_count}g' file.txt"));
    }

    #[test]
    fn perl_match_with_if() {
        assert!(check("perl -ne 'print if /TODO/' file.txt"));
    }

    #[test]
    fn perl_match_after_unless() {
        assert!(check("perl -ne 'print unless /^#/' file.txt"));
    }

    #[test]
    fn perl_module_flag() {
        assert!(check("perl -MList::Util -e 'print length \"test\"'"));
    }

    #[test]
    fn perl_include_flag() {
        assert!(check("perl -Ilib -e 'print \"ok\\n\"'"));
    }

    #[test]
    fn perl_script_file_denied() {
        assert!(!check("perl script.pl"));
    }

    #[test]
    fn perl_no_e_flag_denied() {
        assert!(!check("perl -n file.txt"));
    }

    #[test]
    fn perl_inplace_denied() {
        assert!(!check("perl -i -pe 's/foo/bar/' file.txt"));
    }

    #[test]
    fn perl_inplace_backup_denied() {
        assert!(!check("perl -i.bak -pe 's/foo/bar/' file.txt"));
    }

    #[test]
    fn perl_pie_inplace_denied() {
        assert!(!check("perl -pie 's/foo/bar/' file.txt"));
    }

    #[test]
    fn perl_system_denied() {
        assert!(!check("perl -e 'system(\"rm -rf /\")'"));
    }

    #[test]
    fn perl_exec_denied() {
        assert!(!check("perl -e 'exec(\"bad\")'"));
    }

    #[test]
    fn perl_backtick_denied() {
        assert!(!check("perl -e 'print `ls`'"));
    }

    #[test]
    fn perl_qx_denied() {
        assert!(!check("perl -e 'qx(ls)'"));
    }

    #[test]
    fn perl_eval_denied() {
        assert!(!check("perl -e 'eval(\"bad code\")'"));
    }

    #[test]
    fn perl_open_denied() {
        assert!(!check("perl -e 'open(FH, \">file\")'"));
    }

    #[test]
    fn perl_unlink_denied() {
        assert!(!check("perl -e 'unlink(\"file\")'"));
    }

    #[test]
    fn perl_rename_denied() {
        assert!(!check("perl -e 'rename(\"a\", \"b\")'"));
    }

    #[test]
    fn perl_mkdir_denied() {
        assert!(!check("perl -e 'mkdir(\"dir\")'"));
    }

    #[test]
    fn perl_rmdir_denied() {
        assert!(!check("perl -e 'rmdir(\"dir\")'"));
    }

    #[test]
    fn perl_chmod_denied() {
        assert!(!check("perl -e 'chmod(0755, \"file\")'"));
    }

    #[test]
    fn perl_truncate_denied() {
        assert!(!check("perl -e 'truncate(\"file\", 0)'"));
    }

    #[test]
    fn perl_substitution_eval_denied() {
        assert!(!check("perl -pe 's/foo/bar/e' file.txt"));
    }

    #[test]
    fn perl_substitution_eval_global_denied() {
        assert!(!check("perl -pe 's/foo/bar/ge' file.txt"));
    }

    #[test]
    fn perl_use_denied() {
        assert!(!check("perl -e 'use POSIX'"));
    }

    #[test]
    fn perl_require_denied() {
        assert!(!check("perl -e 'require POSIX'"));
    }

    #[test]
    fn perl_fork_denied() {
        assert!(!check("perl -e 'fork()'"));
    }

    #[test]
    fn perl_socket_denied() {
        assert!(!check("perl -e 'socket(S, 2, 1, 0)'"));
    }
}
