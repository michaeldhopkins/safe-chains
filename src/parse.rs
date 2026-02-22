#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLine(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment(String);

#[derive(Debug, Clone, Eq)]
pub struct Token(String);

impl CommandLine {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn segments(&self) -> Vec<Segment> {
        split_outside_quotes(&self.0)
            .into_iter()
            .map(Segment)
            .collect()
    }
}

impl Segment {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn from_words<S: AsRef<str>>(words: &[S]) -> Self {
        Segment(shell_words::join(words))
    }

    pub fn tokenize(&self) -> Option<Vec<Token>> {
        shell_words::split(&self.0)
            .ok()
            .map(|v| v.into_iter().map(Token).collect())
    }

    pub fn has_unsafe_shell_syntax(&self) -> bool {
        check_unsafe_shell_syntax(&self.0)
    }

    pub fn strip_env_prefix(&self) -> Segment {
        Segment(strip_env_prefix_str(self.as_str()).trim().to_string())
    }

    pub fn strip_fd_redirects(&self) -> Segment {
        match self.tokenize() {
            Some(tokens) => {
                let filtered: Vec<_> = tokens
                    .into_iter()
                    .filter(|t| !t.is_fd_redirect())
                    .collect();
                Token::join(&filtered)
            }
            None => Segment(self.0.clone()),
        }
    }
}

impl Token {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn join(tokens: &[Token]) -> Segment {
        Segment::from_words(tokens)
    }

    pub fn as_command_line(&self) -> CommandLine {
        CommandLine(self.0.clone())
    }

    pub fn command_name(&self) -> &str {
        self.rsplit('/').next().unwrap_or(self.as_str())
    }

    pub fn is_fd_redirect(&self) -> bool {
        let s = self.as_str();
        let rest = s.trim_start_matches(|c: char| c.is_ascii_digit());
        if rest.len() < 2 || !rest.starts_with(">&") {
            return false;
        }
        let after = &rest[2..];
        !after.is_empty() && after.bytes().all(|b| b.is_ascii_digit() || b == b'-')
    }

    pub fn is_dev_null_redirect(&self) -> bool {
        let s = self.as_str();
        let rest = s.trim_start_matches(|c: char| c.is_ascii_digit());
        rest.strip_prefix(">>")
            .or_else(|| rest.strip_prefix('>'))
            .or_else(|| rest.strip_prefix('<'))
            .is_some_and(|after| after == "/dev/null")
    }

    pub fn is_redirect_operator(&self) -> bool {
        let s = self.as_str();
        let rest = s.trim_start_matches(|c: char| c.is_ascii_digit());
        matches!(rest, ">" | ">>" | "<")
    }
}

impl std::ops::Deref for Token {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Token {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialEq<str> for Token {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Token {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<Token> for str {
    fn eq(&self, other: &Token) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<Token> for &str {
    fn eq(&self, other: &Token) -> bool {
        *self == other.as_str()
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub fn has_flag(tokens: &[Token], short: &str, long: Option<&str>) -> bool {
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

fn split_outside_quotes(cmd: &str) -> Vec<String> {
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
                segments.push(std::mem::take(&mut current));
                continue;
            }
            if c == '&' && !current.ends_with('>') {
                segments.push(std::mem::take(&mut current));
                if chars.peek() == Some(&'&') {
                    chars.next();
                }
                continue;
            }
            if c == ';' || c == '\n' {
                segments.push(std::mem::take(&mut current));
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

fn check_unsafe_shell_syntax(segment: &str) -> bool {
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

fn find_unquoted_space(s: &str) -> Option<usize> {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    for (i, b) in s.bytes().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if b == b'\\' && !in_single {
            escaped = true;
            continue;
        }
        if b == b'\'' && !in_double {
            in_single = !in_single;
            continue;
        }
        if b == b'"' && !in_single {
            in_double = !in_double;
            continue;
        }
        if b == b' ' && !in_single && !in_double {
            return Some(i);
        }
    }
    None
}

fn strip_env_prefix_str(segment: &str) -> &str {
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
            if let Some(space_pos) = find_unquoted_space(&trimmed[eq_pos..]) {
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

    fn seg(s: &str) -> Segment {
        Segment(s.to_string())
    }

    fn tok(s: &str) -> Token {
        Token(s.to_string())
    }

    fn toks(words: &[&str]) -> Vec<Token> {
        words.iter().map(|s| tok(s)).collect()
    }

    #[test]
    fn split_pipe() {
        let segs = CommandLine::new("grep foo | head -5").segments();
        assert_eq!(segs, vec![seg("grep foo"), seg("head -5")]);
    }

    #[test]
    fn split_and() {
        let segs = CommandLine::new("ls && echo done").segments();
        assert_eq!(segs, vec![seg("ls"), seg("echo done")]);
    }

    #[test]
    fn split_semicolon() {
        let segs = CommandLine::new("ls; echo done").segments();
        assert_eq!(segs, vec![seg("ls"), seg("echo done")]);
    }

    #[test]
    fn split_preserves_quoted_pipes() {
        let segs = CommandLine::new("echo 'a | b' foo").segments();
        assert_eq!(segs, vec![seg("echo 'a | b' foo")]);
    }

    #[test]
    fn split_background_operator() {
        let segs = CommandLine::new("cat file & rm -rf /").segments();
        assert_eq!(segs, vec![seg("cat file"), seg("rm -rf /")]);
    }

    #[test]
    fn split_newline() {
        let segs = CommandLine::new("echo foo\necho bar").segments();
        assert_eq!(segs, vec![seg("echo foo"), seg("echo bar")]);
    }

    #[test]
    fn unsafe_redirect() {
        assert!(seg("echo hello > file.txt").has_unsafe_shell_syntax());
    }

    #[test]
    fn safe_fd_redirect_stderr_to_stdout() {
        assert!(!seg("cargo clippy 2>&1").has_unsafe_shell_syntax());
    }

    #[test]
    fn safe_fd_redirect_close() {
        assert!(!seg("cmd 2>&-").has_unsafe_shell_syntax());
    }

    #[test]
    fn unsafe_redirect_ampersand_no_digit() {
        assert!(seg("echo hello >& file.txt").has_unsafe_shell_syntax());
    }

    #[test]
    fn unsafe_backtick() {
        assert!(seg("echo `rm -rf /`").has_unsafe_shell_syntax());
    }

    #[test]
    fn unsafe_command_substitution() {
        assert!(seg("echo $(rm -rf /)").has_unsafe_shell_syntax());
    }

    #[test]
    fn safe_quoted_dollar_paren() {
        assert!(!seg("echo '$(safe)' arg").has_unsafe_shell_syntax());
    }

    #[test]
    fn safe_quoted_redirect() {
        assert!(!seg("echo 'greater > than' test").has_unsafe_shell_syntax());
    }

    #[test]
    fn safe_no_special_chars() {
        assert!(!seg("grep pattern file").has_unsafe_shell_syntax());
    }

    #[test]
    fn safe_redirect_to_dev_null() {
        assert!(!seg("cmd >/dev/null").has_unsafe_shell_syntax());
    }

    #[test]
    fn safe_redirect_stderr_to_dev_null() {
        assert!(!seg("cmd 2>/dev/null").has_unsafe_shell_syntax());
    }

    #[test]
    fn safe_redirect_append_to_dev_null() {
        assert!(!seg("cmd >>/dev/null").has_unsafe_shell_syntax());
    }

    #[test]
    fn safe_redirect_space_dev_null() {
        assert!(!seg("cmd > /dev/null").has_unsafe_shell_syntax());
    }

    #[test]
    fn safe_redirect_input_dev_null() {
        assert!(!seg("cmd < /dev/null").has_unsafe_shell_syntax());
    }

    #[test]
    fn safe_redirect_both_dev_null() {
        assert!(!seg("cmd 2>/dev/null").has_unsafe_shell_syntax());
    }

    #[test]
    fn unsafe_redirect_dev_null_prefix() {
        assert!(seg("cmd > /dev/nullicious").has_unsafe_shell_syntax());
    }

    #[test]
    fn unsafe_redirect_dev_null_path_traversal() {
        assert!(seg("cmd > /dev/null/../etc/passwd").has_unsafe_shell_syntax());
    }

    #[test]
    fn unsafe_redirect_dev_null_subpath() {
        assert!(seg("cmd > /dev/null/foo").has_unsafe_shell_syntax());
    }

    #[test]
    fn unsafe_redirect_to_file() {
        assert!(seg("cmd > output.txt").has_unsafe_shell_syntax());
    }

    #[test]
    fn has_flag_short() {
        let tokens = toks(&["sed", "-i", "s/foo/bar/"]);
        assert!(has_flag(&tokens, "-i", Some("--in-place")));
    }

    #[test]
    fn has_flag_long_with_eq() {
        let tokens = toks(&["sed", "--in-place=.bak", "s/foo/bar/"]);
        assert!(has_flag(&tokens, "-i", Some("--in-place")));
    }

    #[test]
    fn has_flag_combined_short() {
        let tokens = toks(&["sed", "-ni", "s/foo/bar/p"]);
        assert!(has_flag(&tokens, "-i", Some("--in-place")));
    }

    #[test]
    fn has_flag_stops_at_double_dash() {
        let tokens = toks(&["cmd", "--", "-i"]);
        assert!(!has_flag(&tokens, "-i", Some("--in-place")));
    }

    #[test]
    fn strip_single_env_var() {
        assert_eq!(
            seg("RACK_ENV=test bundle exec rspec").strip_env_prefix(),
            seg("bundle exec rspec")
        );
    }

    #[test]
    fn strip_multiple_env_vars() {
        assert_eq!(
            seg("RACK_ENV=test RAILS_ENV=test bundle exec rspec").strip_env_prefix(),
            seg("bundle exec rspec")
        );
    }

    #[test]
    fn strip_no_env_var() {
        assert_eq!(
            seg("bundle exec rspec").strip_env_prefix(),
            seg("bundle exec rspec")
        );
    }

    #[test]
    fn tokenize_simple() {
        assert_eq!(
            seg("grep foo file.txt").tokenize(),
            Some(vec![tok("grep"), tok("foo"), tok("file.txt")])
        );
    }

    #[test]
    fn tokenize_quoted() {
        assert_eq!(
            seg("echo 'hello world'").tokenize(),
            Some(vec![tok("echo"), tok("hello world")])
        );
    }

    #[test]
    fn strip_env_quoted_single() {
        assert_eq!(
            seg("FOO='bar baz' ls").strip_env_prefix(),
            seg("ls")
        );
    }

    #[test]
    fn strip_env_quoted_double() {
        assert_eq!(
            seg("FOO=\"bar baz\" ls").strip_env_prefix(),
            seg("ls")
        );
    }

    #[test]
    fn strip_env_quoted_with_equals() {
        assert_eq!(
            seg("FOO='a=b' ls").strip_env_prefix(),
            seg("ls")
        );
    }

    #[test]
    fn strip_env_quoted_multiple() {
        assert_eq!(
            seg("FOO='x y' BAR=\"a b\" cmd").strip_env_prefix(),
            seg("cmd")
        );
    }

    #[test]
    fn command_name_simple() {
        assert_eq!(tok("ls").command_name(), "ls");
    }

    #[test]
    fn command_name_with_path() {
        assert_eq!(tok("/usr/bin/ls").command_name(), "ls");
    }

    #[test]
    fn command_name_relative_path() {
        assert_eq!(tok("./scripts/test.sh").command_name(), "test.sh");
    }

    #[test]
    fn fd_redirect_detection() {
        assert!(tok("2>&1").is_fd_redirect());
        assert!(tok(">&2").is_fd_redirect());
        assert!(tok("10>&1").is_fd_redirect());
        assert!(tok("255>&2").is_fd_redirect());
        assert!(tok("2>&-").is_fd_redirect());
        assert!(tok("2>&10").is_fd_redirect());
        assert!(!tok(">").is_fd_redirect());
        assert!(!tok("/dev/null").is_fd_redirect());
        assert!(!tok(">&").is_fd_redirect());
        assert!(!tok("").is_fd_redirect());
        assert!(!tok("42").is_fd_redirect());
        assert!(!tok("123abc").is_fd_redirect());
    }

    #[test]
    fn dev_null_redirect_single_token() {
        assert!(tok(">/dev/null").is_dev_null_redirect());
        assert!(tok(">>/dev/null").is_dev_null_redirect());
        assert!(tok("2>/dev/null").is_dev_null_redirect());
        assert!(tok("2>>/dev/null").is_dev_null_redirect());
        assert!(tok("</dev/null").is_dev_null_redirect());
        assert!(tok("10>/dev/null").is_dev_null_redirect());
        assert!(tok("255>/dev/null").is_dev_null_redirect());
        assert!(!tok(">/tmp/file").is_dev_null_redirect());
        assert!(!tok(">/dev/nullicious").is_dev_null_redirect());
        assert!(!tok("ls").is_dev_null_redirect());
        assert!(!tok("").is_dev_null_redirect());
        assert!(!tok("42").is_dev_null_redirect());
        assert!(!tok("<</dev/null").is_dev_null_redirect());
    }

    #[test]
    fn redirect_operator_detection() {
        assert!(tok(">").is_redirect_operator());
        assert!(tok(">>").is_redirect_operator());
        assert!(tok("<").is_redirect_operator());
        assert!(tok("2>").is_redirect_operator());
        assert!(tok("2>>").is_redirect_operator());
        assert!(tok("10>").is_redirect_operator());
        assert!(tok("255>>").is_redirect_operator());
        assert!(!tok("ls").is_redirect_operator());
        assert!(!tok(">&1").is_redirect_operator());
        assert!(!tok("/dev/null").is_redirect_operator());
        assert!(!tok("").is_redirect_operator());
        assert!(!tok("42").is_redirect_operator());
        assert!(!tok("<<").is_redirect_operator());
    }

    #[test]
    fn reverse_partial_eq() {
        let t = tok("hello");
        assert!("hello" == t);
        assert!("world" != t);
        let s: &str = "hello";
        assert!(s == t);
    }
}
