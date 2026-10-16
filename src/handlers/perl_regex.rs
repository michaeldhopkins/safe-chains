pub(super) fn closing_delimiter(open: u8) -> u8 {
    match open {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        b'<' => b'>',
        _ => open,
    }
}

pub(super) fn is_paired_delimiter(b: u8) -> bool {
    matches!(b, b'(' | b'[' | b'{' | b'<')
}

pub(super) fn skip_delimited(bytes: &[u8], start: usize) -> Option<usize> {
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

fn skip_one_section(bytes: &[u8], pos: usize) -> Option<usize> {
    let delim = *bytes.get(pos)?;
    if delim.is_ascii_alphanumeric() || delim.is_ascii_whitespace() || delim == b'_' {
        return None;
    }
    if is_paired_delimiter(delim) {
        skip_delimited(bytes, pos)
    } else {
        let mut i = pos + 1;
        while i < bytes.len() {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == delim {
                return Some(i + 1);
            }
            i += 1;
        }
        None
    }
}

fn skip_regex_body(bytes: &[u8], start: usize, sections: usize) -> Option<(usize, &[u8])> {
    let mut pos = start;
    while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    let delim = *bytes.get(pos)?;
    if delim.is_ascii_alphanumeric() || delim == b'_' || (pos > start && delim == b'#') {
        return None;
    }

    if is_paired_delimiter(delim) {
        for s in 0..sections {
            if s > 0 {
                let before_ws = pos;
                while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
                    pos += 1;
                }
                if pos > before_ws && bytes.get(pos) == Some(&b'#') {
                    return None;
                }
            }
            pos = skip_one_section(bytes, pos)?;
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

fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn at_word_boundary(bytes: &[u8], i: usize) -> bool {
    i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_')
}

/// The code with its inert text dropped, read in one pass from left to right so a quote inside a
/// regex and a slash inside a string are each read as perl reads them: string literals, comments,
/// regex bodies and substitution replacements. Interpolated expressions in a double-quoted string
/// are kept for the identifier walk. `None` when the text cannot be read or can run code: an
/// unterminated string, regex or quote-like, a `(?{ })` or `(??{ })` block or an interpolated
/// expression in a pattern or replacement, a substitution with the `e` flag, and a heredoc or a
/// POD block, whose extent this pass does not follow.
pub(super) fn strip_inert_text(code: &str) -> Option<String> {
    let bytes = code.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'$' if bytes.get(i + 1).is_some_and(|b| {
                (b.is_ascii_punctuation() && !matches!(b, b'{' | b'_' | b'$' | b'#'))
                    || (*b == b'$' && !bytes.get(i + 2).is_some_and(|c| is_name_byte(*c) || matches!(c, b'{' | b'$')))
            }) =>
            {
                result.extend_from_slice(b" 1 ");
                i += 2;
                continue;
            }
            b'\'' if i > 0 && is_name_byte(bytes[i - 1]) && bytes.get(i + 1).is_some_and(|b| is_name_byte(*b)) => return None,
            b'"' => {
                i = strip_double_quoted(bytes, i, &mut result)?;
                continue;
            }
            b'\'' => {
                let (end, letters) = skip_regex_body(bytes, i, 1)?;
                result.extend_from_slice(b" 1 ");
                i = end - letters.len();
                continue;
            }
            b'#' if i == 0 || bytes[i - 1] != b'$' => {
                i = bytes[i..].iter().position(|b| *b == b'\n').map_or(bytes.len(), |n| i + n);
                continue;
            }
            b'<' if bytes.get(i + 1) == Some(&b'<')
                && bytes.get(i + 2).is_some_and(|b| matches!(b, b'"' | b'\'' | b'~' | b'_') || b.is_ascii_alphabetic()) =>
            {
                return None;
            }
            b'=' if (i == 0 || bytes[i - 1] == b'\n') && bytes.get(i + 1).is_some_and(u8::is_ascii_alphabetic) => return None,
            _ => {}
        }

        if let Some((keyword, sections)) = quote_like_at(bytes, i) {
            let open = i + keyword.len();
            let (end, flags) = skip_regex_body(bytes, open, sections)?;
            let inert = matches!(keyword, b"q" | b"qw");
            if !inert && (runs_code(&bytes[open..end]) || (keyword == b"s" && flags.contains(&b'e'))) {
                return None;
            }
            result.extend_from_slice(keyword);
            result.extend_from_slice(b" 1 ");
            i = end;
            continue;
        }

        if bytes[i] == b'/' && is_regex_context(&result) {
            let (end, _) = skip_regex_body(bytes, i, 1)?;
            if runs_code(&bytes[i..end]) {
                return None;
            }
            result.extend_from_slice(b" 1 ");
            i = end;
            continue;
        }

        result.push(bytes[i]);
        i += 1;
    }

    String::from_utf8(result).ok()
}

/// A quote-like operator starting at `i`, with how many delimited sections it takes. A sigil,
/// `-` or `->` before the letters makes them a name, a file test or a method, and `{s}` makes
/// them an autoquoted hash key, never an operator.
fn quote_like_at(bytes: &[u8], i: usize) -> Option<(&'static [u8], usize)> {
    if !at_word_boundary(bytes, i) || (i > 0 && matches!(bytes[i - 1], b'$' | b'@' | b'%' | b'&' | b'*' | b'-' | b'>')) {
        return None;
    }
    let rest = &bytes[i..];
    let (keyword, sections): (&'static [u8], usize) = [
        (&b"tr"[..], 2),
        (&b"qr"[..], 1),
        (&b"qq"[..], 1),
        (&b"qw"[..], 1),
        (&b"s"[..], 2),
        (&b"y"[..], 2),
        (&b"m"[..], 1),
        (&b"q"[..], 1),
    ]
    .into_iter()
    .find(|(keyword, _)| rest.starts_with(keyword))?;
    if rest.get(keyword.len()).is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_') {
        return None;
    }
    let mut j = keyword.len();
    while rest.get(j).is_some_and(u8::is_ascii_whitespace) {
        j += 1;
    }
    let next = *rest.get(j)?;
    let hash_key = next == b'}' && bytes[..i].iter().rfind(|b| !b.is_ascii_whitespace()) == Some(&b'{');
    let delimiter = !next.is_ascii_alphanumeric() && next != b'_' && !hash_key;
    delimiter.then_some((keyword, sections))
}

fn simple_interpolated(slice: &[u8]) -> bool {
    !slice.is_empty()
        && slice
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b':' | b'$' | b'@' | b' ' | b'\t'))
}

/// Copies the double-quoted string starting at `start` into `out` as blanks, keeping the
/// expressions it interpolates (`@{[ EXPR ]}`, `${\ EXPR }`, subscripts) for the identifier walk,
/// and returns where it ends. `None` when it never closes.
fn strip_double_quoted(bytes: &[u8], start: usize, out: &mut Vec<u8>) -> Option<usize> {
    out.push(b' ');
    let mut i = start + 1;
    loop {
        match *bytes.get(i)? {
            b'\\' => {
                out.push(b' ');
                i += 2;
            }
            b'"' => {
                out.extend_from_slice(b" 1 ");
                return Some(i + 1);
            }
            b'$' | b'@' => {
                out.push(b' ');
                i += 1;
                while matches!(bytes.get(i), Some(b'$' | b'#')) {
                    i += 1;
                }
                if bytes.get(i) == Some(&b'{') {
                    let end = skip_delimited(bytes, i)?;
                    if !simple_interpolated(&bytes[i + 1..end - 1]) {
                        return None;
                    }
                    out.extend_from_slice(&bytes[i + 1..end - 1]);
                    out.push(b' ');
                    i = end;
                    continue;
                }
                while i < bytes.len() && (bytes[i] == b'_' || bytes[i] == b':' || bytes[i].is_ascii_alphanumeric()) {
                    i += 1;
                }
                while matches!(bytes.get(i), Some(b'[' | b'{'))
                    || (bytes.get(i..i + 2) == Some(b"->") && matches!(bytes.get(i + 2), Some(b'[' | b'{')))
                {
                    if bytes.get(i) == Some(&b'-') {
                        i += 2;
                    }
                    let end = skip_delimited(bytes, i)?;
                    if !simple_interpolated(&bytes[i + 1..end - 1]) {
                        return None;
                    }
                    out.extend_from_slice(&bytes[i + 1..end - 1]);
                    out.push(b' ');
                    i = end;
                }
            }
            _ => {
                out.push(b' ');
                i += 1;
            }
        }
    }
}

/// Whether a regex or replacement section evaluates code when perl compiles or applies it.
pub(super) fn runs_code(section: &[u8]) -> bool {
    let mut i = 0;
    while i < section.len() {
        match section[i] {
            b'\\' => {
                i += 2;
                continue;
            }
            b'(' => {
                let mut j = i + 1;
                while matches!(section.get(j), Some(b'?' | b'*' | b' ' | b'\t' | b'\n')) {
                    j += 1;
                }
                if j > i + 1 && section.get(j) == Some(&b'{') {
                    return true;
                }
            }
            b'$' | b'@' => {
                let mut j = i + 1;
                while matches!(section.get(j), Some(b'$' | b'#')) {
                    j += 1;
                }
                let braced = section.get(j) == Some(&b'{');
                j += usize::from(braced);
                let name = j;
                while section.get(j).is_some_and(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b':')) {
                    j += 1;
                }
                if braced {
                    if j == name || section.get(j) != Some(&b'}') {
                        return true;
                    }
                    j += 1;
                }
                if (braced || j > name) && (matches!(section.get(j), Some(b'[' | b'{')) || section[j..].starts_with(b"->")) {
                    return true;
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
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
        return start == 0 || !matches!(preceding[start - 1], b'$' | b'@' | b'%');
    }
    false
}

pub(super) fn has_substitution_eval(code: &str) -> bool {
    let bytes = code.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b's'
            && at_word_boundary(bytes, i)
            && !bytes.get(i + 1).is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
            && let Some((_, flags)) = skip_regex_body(bytes, i + 1, 2)
            && flags.contains(&b'e')
        {
            return true;
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::is_safe_command;
    use proptest::prelude::*;

    fn check(cmd: &str) -> bool {
        is_safe_command(cmd)
    }

    safe! {
        plain_match: "perl -ne 'print if /foo$/' a.txt",
        capture_in_replacement: "perl -pe 's/(\\d+)/$1x/' a.txt",
        braced_capture_in_replacement: "perl -pe 's/(\\d+)/${1}x/' a.txt",
        named_variable_in_pattern: "perl -ne 'my $w = 1; print if /$w/' a.txt",
        at_sign_in_pattern: "perl -ne 'print if /a@b\\.com/' a.txt",
        quantifier_braces: "perl -ne 'print if /a{2,3}/' a.txt",
        paired_delimiters: "perl -pe 's{a}{b}g' a.txt",
        non_capturing_group: "perl -ne 'print if /(?:ab)+/' a.txt",
        slash_in_double_quoted_string: "perl -e 'print \"a/b\"; print 1'",
        slash_in_single_quoted_string: "perl -e \"print 'a/b'; print 1\"",
        comment_after_code: "perl -e 'print 1; # a/b'",
        file_test_size: "perl -e 'print -s \"a.txt\"'",
        hash_key_named_like_an_operator: "perl -e 'my %h; print $h{s}; print $h{y}'",
        quoted_word_list: "perl -e 'print for qw(a/b c)'",
        left_shift: "perl -e 'print 1 << 2'",
        record_separator_variable: "perl -e 'print $/; print $\"'",
    }

    denied! {
        code_block_in_pattern: "perl -ne 'print if /(?{ 1 })/' a.txt",
        postponed_block_in_pattern: "perl -ne 'print if /(??{ 1 })/' a.txt",
        code_block_after_match_operator: "perl -ne 'print if m#(?{ 1 })#' a.txt",
        code_block_in_bound_match: "perl -e '\"a\" =~ /(?{ 1 })/'",
        interpolated_block_in_pattern: "perl -ne 'print if /@{[ 1 ]}/' a.txt",
        interpolated_block_in_replacement: "perl -pe 's/a/@{[ 1 ]}/' a.txt",
        scalar_block_in_replacement: "perl -pe 's/a/${\\ 1}/' a.txt",
        subscript_in_replacement: "perl -pe 's/a/$h{1}/' a.txt",
        unterminated_pattern: "perl -ne 'print if /a' a.txt",
        placeholder_in_pattern_from_find: "find . -exec perl -ne 'print if /{}/' a.txt \\;",
        placeholder_in_pattern_from_xargs: "ls | xargs -I{} perl -ne 'print if /{}/' a.txt",
        placeholder_in_replacement_from_xargs: "ls | xargs -I{} perl -pe 's/x/{}/' a.txt",
        placeholder_in_pattern_from_fd: "fd -x perl -ne 'print if /{}/' a.txt",
        quote_inside_pattern_hides_nothing: "perl -ne 'print if /\"(?{ 1 })\"/' a.txt",
        slash_in_single_quoted_string_hides_nothing: "perl -e \"print 'x;/'; unlink 1; '/'\"",
        slash_in_comment_hides_nothing: "perl -e 'print 1; #;/\nunlink 1;\n/'",
        slash_in_quoted_words_hides_nothing: "perl -e 'q{/}; unlink 1; q{/}'",
        hash_key_hides_nothing: "perl -e 'my %h; $h{s}; unlink 1; $h{s}'",
        unterminated_string: "perl -e 'print \"a'",
        interpolating_quote_like: "perl -e 'print qq{@{[ 1 ]}}'",
        heredoc: "perl -e 'print <<\"E\";\n;/\nE\nunlink 1; /'",
        pod_block: "perl -e 'print 1;\n=pod\n;/\n=cut\nunlink 1; /'",
        postmatch_variable_hides_nothing: "perl -e \"print $'; unlink 1; print $'\"",
        list_separator_variable_hides_nothing: "perl -e 'print $\"; unlink 1; print $\"'",
        old_package_separator: "perl -e 'print $main'\\''x'",
        quote_in_interpolated_arrayref_hides_nothing: "perl -e 'print \"@{[ q(}); unlink 1 ]}\"'",
        quote_in_interpolated_subscript_hides_nothing: "perl -e 'print \"$h{q(}); unlink 1}\"'",
        arrow_subscript_in_double_quotes: "perl -e 'print \"$r->{unlink 1}\"'",
        last_index_block_in_double_quotes: "perl -e 'print \"$#{[unlink 1]}\"'",
        regex_code_in_interpolated_arrayref: "perl -e 'print \"@{[ /(?{ 1 })/ ]}\"'",
        match_operator_with_space_hides_nothing: "perl -e \"m /'/; unlink 1; m /'/\"",
        substitution_eval_with_space: "perl -pe 's /a/\"unlink 1\"/ee' a.txt",
        substitution_eval_after_paired_first_section: "perl -pe 's{a}  \"unlink 1\"ee' a.txt",
        comma_delimited_quote_like_hides_nothing: "perl -e \"q,'/,, unlink 1, q,'/,\"",
        equals_delimited_match_runs_code: "perl -e 'm=(?{ 1 })='",
        bare_regex_after_print_hides_nothing: "perl -e \"print /'/; unlink 1; print /'/\"",
        bare_regex_after_split_runs_code: "perl -ne 'split /(?{ 1 })/' a.txt",
        division_after_regex_hides_nothing: "perl -ne 'print /a/ / 2; unlink 1; print /a/' a.txt",
        division_after_string_hides_nothing: "perl -e 'print \"8\" / 2; unlink 1; print 1 / 2'",
        division_after_punct_var_hides_nothing: "perl -e 'print $; / 2; unlink 1; print 1 / 2'",
        division_after_pid_var_hides_nothing: "perl -e 'print $$ / 2; unlink 1; print 1 / 2'",
    }

    proptest! {
        /// Perl evaluates a code block or an interpolated expression wherever a pattern or a
        /// replacement holds one, so a program carrying one is never approved, whatever surrounds
        /// it and whichever quote-like form holds it.
        #[test]
        fn a_regex_that_evaluates_code_is_never_approved(
            form in prop::sample::select(vec!["(?{ 1 })", "(??{ 1 })", "(*{ 1 })", "@{[ 1 ]}", "${\\ 1}", "$h{1}", "$a[1]", "$r->[1]", "$#{[1]}"]),
            before in "[a-z0-9 .^]{0,4}",
            after in "[a-z0-9 .$]{0,4}",
            wrapper in prop::sample::select(vec!["print if /{}/", "print if m!{}!", "print if m{{}}", "s/x/{}/", "s/{}/x/", "s{x}{{}}", "$_ =~ /{}/"]),
        ) {
            let body = format!("{before}{form}{after}");
            let program = wrapper.replacen("{}", &body, 1);
            let line = format!("perl -ne '{program}' a.txt");
            prop_assert!(!check(&line), "{line} must not approve");
        }

        /// Text perl reads as inert (a string, a comment, a pattern, a quoted word list) never
        /// hides the code between two such pieces, whatever quotes and slashes they hold.
        #[test]
        fn code_between_inert_pieces_is_always_read(
            first in prop::sample::select(INERT_PIECES.to_vec()),
            second in prop::sample::select(INERT_PIECES.to_vec()),
        ) {
            let program = format!("{first}; unlink 1; {second}");
            let line = format!("perl -e {}", shell_words::quote(&program));
            prop_assert!(!check(&line), "{line} must not approve");
        }
    }

    const INERT_PIECES: &[&str] = &[
        "\"a/b\"", "\"x;/\"", "'x;/'", "'a\"b'", "q{/}", "qw(/ ;)", "qq{/}", "# x;/\n", "print if /x\"/", "print if m{x'}", "my %h; $h{s}",
        "$h{y}", "s/'/\"/", "tr/\"/'/", "print 1 / 2", "$x =~ s{/}{;}", "print $'", "print $\"", "print $/", "print $#", "print $;",
        "q,'/,", "print /x'/", "print /a/ / 2", "print \"8\" / 2", "print $; / 2", "print $$ / 2",
    ];
}
