pub fn split_outside_quotes(cmd: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut chars = cmd.chars().peekable();

    while let Some(c) = chars.next() {
        if escaped {
            current.push(c);
            escaped = false;
            continue;
        }
        if c == '\\' && !in_single {
            escaped = true;
            current.push(c);
            continue;
        }
        if c == '\'' && !in_double {
            in_single = !in_single;
            current.push(c);
            continue;
        }
        if c == '"' && !in_single {
            in_double = !in_double;
            current.push(c);
            continue;
        }
        if !in_single && !in_double {
            if c == '|' {
                segments.push(current.clone());
                current.clear();
                continue;
            }
            if c == '&' && !current.ends_with('>') {
                segments.push(current.clone());
                current.clear();
                if chars.peek() == Some(&'&') {
                    chars.next();
                }
                continue;
            }
            if c == ';' || c == '\n' {
                segments.push(current.clone());
                current.clear();
                continue;
            }
        }
        current.push(c);
    }
    segments.push(current);
    segments
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn tokenize(segment: &str) -> Option<Vec<String>> {
    shell_words::split(segment).ok()
}

pub fn has_unsafe_shell_syntax(segment: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let chars: Vec<char> = segment.chars().collect();

    for (i, &c) in chars.iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' && !in_single {
            escaped = true;
            continue;
        }
        if c == '\'' && !in_double {
            in_single = !in_single;
            continue;
        }
        if c == '"' && !in_single {
            in_double = !in_double;
            continue;
        }
        if !in_single && !in_double {
            if c == '>' || c == '<' {
                let next = chars.get(i + 1);
                if next == Some(&'&')
                    && chars
                        .get(i + 2)
                        .is_some_and(|ch| ch.is_ascii_digit() || *ch == '-')
                {
                    continue;
                }
                if is_dev_null_target(&chars, i + 1, c) {
                    continue;
                }
                return true;
            }
            if c == '`' {
                return true;
            }
            if c == '$' && chars.get(i + 1) == Some(&'(') {
                return true;
            }
        }
    }
    false
}

const DEV_NULL: [char; 9] = ['/', 'd', 'e', 'v', '/', 'n', 'u', 'l', 'l'];

fn is_dev_null_target(chars: &[char], start: usize, redirect_char: char) -> bool {
    let mut j = start;
    if redirect_char == '>' && j < chars.len() && chars[j] == '>' {
        j += 1;
    }
    while j < chars.len() && chars[j] == ' ' {
        j += 1;
    }
    if j + DEV_NULL.len() > chars.len() {
        return false;
    }
    if chars[j..j + DEV_NULL.len()] != DEV_NULL {
        return false;
    }
    let end = j + DEV_NULL.len();
    end >= chars.len() || chars[end].is_whitespace() || ";|&)".contains(chars[end])
}

pub fn has_flag(tokens: &[String], short: &str, long: Option<&str>) -> bool {
    let short_char = short.trim_start_matches('-');
    for token in &tokens[1..] {
        if token == "--" {
            return false;
        }
        if let Some(long_flag) = long
            && (token == long_flag || token.starts_with(&format!("{long_flag}=")))
        {
            return true;
        }
        if token.starts_with('-') && !token.starts_with("--") && token[1..].contains(short_char) {
            return true;
        }
    }
    false
}

pub fn is_fd_redirect(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes.len() < 3 {
        return false;
    }
    let start = usize::from(bytes[0].is_ascii_digit());
    bytes.get(start) == Some(&b'>')
        && bytes.get(start + 1) == Some(&b'&')
        && bytes[start + 2..].iter().all(|b| b.is_ascii_digit() || *b == b'-')
}

pub fn strip_fd_redirects(s: &str) -> String {
    match tokenize(s) {
        Some(tokens) => {
            let filtered: Vec<_> = tokens
                .into_iter()
                .filter(|t| !is_fd_redirect(t))
                .collect();
            shell_words::join(&filtered)
        }
        None => s.to_string(),
    }
}

pub fn strip_env_prefix(segment: &str) -> &str {
    let mut rest = segment;
    loop {
        let trimmed = rest.trim_start();
        if trimmed.is_empty() {
            return trimmed;
        }
        let bytes = trimmed.as_bytes();
        if !bytes[0].is_ascii_uppercase() && bytes[0] != b'_' {
            return trimmed;
        }
        if let Some(eq_pos) = trimmed.find('=') {
            let key = &trimmed[..eq_pos];
            let valid_key = key
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
            if !valid_key {
                return trimmed;
            }
            if let Some(space_pos) = trimmed[eq_pos..].find(' ') {
                rest = &trimmed[eq_pos + space_pos..];
                continue;
            }
            return trimmed;
        }
        return trimmed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_pipe() {
        assert_eq!(
            split_outside_quotes("grep foo | head -5"),
            vec!["grep foo", "head -5"]
        );
    }

    #[test]
    fn split_and() {
        assert_eq!(
            split_outside_quotes("ls && echo done"),
            vec!["ls", "echo done"]
        );
    }

    #[test]
    fn split_semicolon() {
        assert_eq!(
            split_outside_quotes("ls; echo done"),
            vec!["ls", "echo done"]
        );
    }

    #[test]
    fn split_preserves_quoted_pipes() {
        assert_eq!(
            split_outside_quotes("echo 'a | b' foo"),
            vec!["echo 'a | b' foo"]
        );
    }

    #[test]
    fn split_background_operator() {
        assert_eq!(
            split_outside_quotes("cat file & rm -rf /"),
            vec!["cat file", "rm -rf /"]
        );
    }

    #[test]
    fn split_newline() {
        assert_eq!(
            split_outside_quotes("echo foo\necho bar"),
            vec!["echo foo", "echo bar"]
        );
    }

    #[test]
    fn unsafe_redirect() {
        assert!(has_unsafe_shell_syntax("echo hello > file.txt"));
    }

    #[test]
    fn safe_fd_redirect_stderr_to_stdout() {
        assert!(!has_unsafe_shell_syntax("cargo clippy 2>&1"));
    }

    #[test]
    fn safe_fd_redirect_close() {
        assert!(!has_unsafe_shell_syntax("cmd 2>&-"));
    }

    #[test]
    fn unsafe_redirect_ampersand_no_digit() {
        assert!(has_unsafe_shell_syntax("echo hello >& file.txt"));
    }

    #[test]
    fn unsafe_backtick() {
        assert!(has_unsafe_shell_syntax("echo `rm -rf /`"));
    }

    #[test]
    fn unsafe_command_substitution() {
        assert!(has_unsafe_shell_syntax("echo $(rm -rf /)"));
    }

    #[test]
    fn safe_quoted_dollar_paren() {
        assert!(!has_unsafe_shell_syntax("echo '$(safe)' arg"));
    }

    #[test]
    fn safe_quoted_redirect() {
        assert!(!has_unsafe_shell_syntax("echo 'greater > than' test"));
    }

    #[test]
    fn safe_no_special_chars() {
        assert!(!has_unsafe_shell_syntax("grep pattern file"));
    }

    #[test]
    fn safe_redirect_to_dev_null() {
        assert!(!has_unsafe_shell_syntax("cmd >/dev/null"));
    }

    #[test]
    fn safe_redirect_stderr_to_dev_null() {
        assert!(!has_unsafe_shell_syntax("cmd 2>/dev/null"));
    }

    #[test]
    fn safe_redirect_append_to_dev_null() {
        assert!(!has_unsafe_shell_syntax("cmd >>/dev/null"));
    }

    #[test]
    fn safe_redirect_space_dev_null() {
        assert!(!has_unsafe_shell_syntax("cmd > /dev/null"));
    }

    #[test]
    fn safe_redirect_input_dev_null() {
        assert!(!has_unsafe_shell_syntax("cmd < /dev/null"));
    }

    #[test]
    fn safe_redirect_both_dev_null() {
        assert!(!has_unsafe_shell_syntax("cmd 2>/dev/null"));
    }

    #[test]
    fn unsafe_redirect_dev_null_prefix() {
        assert!(has_unsafe_shell_syntax("cmd > /dev/nullicious"));
    }

    #[test]
    fn unsafe_redirect_dev_null_path_traversal() {
        assert!(has_unsafe_shell_syntax("cmd > /dev/null/../etc/passwd"));
    }

    #[test]
    fn unsafe_redirect_dev_null_subpath() {
        assert!(has_unsafe_shell_syntax("cmd > /dev/null/foo"));
    }

    #[test]
    fn unsafe_redirect_to_file() {
        assert!(has_unsafe_shell_syntax("cmd > output.txt"));
    }

    #[test]
    fn has_flag_short() {
        let tokens: Vec<String> = vec!["sed", "-i", "s/foo/bar/"]
            .into_iter()
            .map(String::from)
            .collect();
        assert!(has_flag(&tokens, "-i", Some("--in-place")));
    }

    #[test]
    fn has_flag_long_with_eq() {
        let tokens: Vec<String> = vec!["sed", "--in-place=.bak", "s/foo/bar/"]
            .into_iter()
            .map(String::from)
            .collect();
        assert!(has_flag(&tokens, "-i", Some("--in-place")));
    }

    #[test]
    fn has_flag_combined_short() {
        let tokens: Vec<String> = vec!["sed", "-ni", "s/foo/bar/p"]
            .into_iter()
            .map(String::from)
            .collect();
        assert!(has_flag(&tokens, "-i", Some("--in-place")));
    }

    #[test]
    fn has_flag_stops_at_double_dash() {
        let tokens: Vec<String> = vec!["cmd", "--", "-i"]
            .into_iter()
            .map(String::from)
            .collect();
        assert!(!has_flag(&tokens, "-i", Some("--in-place")));
    }

    #[test]
    fn strip_single_env_var() {
        assert_eq!(strip_env_prefix("RACK_ENV=test bundle exec rspec"), "bundle exec rspec");
    }

    #[test]
    fn strip_multiple_env_vars() {
        assert_eq!(
            strip_env_prefix("RACK_ENV=test RAILS_ENV=test bundle exec rspec"),
            "bundle exec rspec"
        );
    }

    #[test]
    fn strip_no_env_var() {
        assert_eq!(strip_env_prefix("bundle exec rspec"), "bundle exec rspec");
    }

    #[test]
    fn tokenize_simple() {
        assert_eq!(
            tokenize("grep foo file.txt"),
            Some(vec!["grep".to_string(), "foo".to_string(), "file.txt".to_string()])
        );
    }

    #[test]
    fn tokenize_quoted() {
        assert_eq!(
            tokenize("echo 'hello world'"),
            Some(vec!["echo".to_string(), "hello world".to_string()])
        );
    }
}
