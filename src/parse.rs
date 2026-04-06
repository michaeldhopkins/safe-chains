use std::ops::Deref;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token(String);

impl Deref for Token {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

#[derive(Copy, Clone)]
pub struct WordSet(&'static [&'static str]);

impl WordSet {
    pub const fn new(words: &'static [&'static str]) -> Self {
        let mut i = 1;
        while i < words.len() {
            assert!(
                const_less(words[i - 1].as_bytes(), words[i].as_bytes()),
                "WordSet: entries must be sorted, no duplicates"
            );
            i += 1;
        }
        Self(words)
    }

    pub const fn flags(words: &'static [&'static str]) -> Self {
        let mut i = 0;
        while i < words.len() {
            let b = words[i].as_bytes();
            assert!(b.len() >= 2, "WordSet::flags: flag too short (need at least 2 chars)");
            assert!(b[0] == b'-', "WordSet::flags: flag must start with '-'");
            if b[1] == b'-' {
                assert!(b.len() >= 3, "WordSet::flags: long flag needs at least 3 chars (e.g. --x)");
            }
            i += 1;
        }
        Self::new(words)
    }

    pub fn contains(&self, s: &str) -> bool {
        self.0.binary_search(&s).is_ok()
    }

    pub fn contains_short(&self, b: u8) -> bool {
        let target = [b'-', b];
        std::str::from_utf8(&target).is_ok_and(|s| self.0.binary_search(&s).is_ok())
    }

    pub fn iter(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.0.iter().copied()
    }
}

const fn const_less(a: &[u8], b: &[u8]) -> bool {
    let min = if a.len() < b.len() { a.len() } else { b.len() };
    let mut i = 0;
    while i < min {
        if a[i] < b[i] {
            return true;
        }
        if a[i] > b[i] {
            return false;
        }
        i += 1;
    }
    a.len() < b.len()
}

impl Token {
    pub(crate) fn from_raw(s: String) -> Self {
        Self(s)
    }

    #[cfg(test)]
    pub(crate) fn from_test(s: &str) -> Self {
        Self(s.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn command_name(&self) -> &str {
        let s = self.as_str();
        if s.starts_with('@') {
            return s;
        }
        s.rsplit('/').next().unwrap_or(s)
    }

    pub fn is_one_of(&self, options: &[&str]) -> bool {
        options.contains(&self.as_str())
    }

    pub fn split_value(&self, sep: &str) -> Option<&str> {
        self.as_str().split_once(sep).map(|(_, v)| v)
    }

    pub fn content_outside_double_quotes(&self) -> String {
        let bytes = self.as_str().as_bytes();
        let mut result = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'"' {
                result.push(b' ');
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'"' {
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

pub fn has_flag(tokens: &[Token], short: Option<&str>, long: Option<&str>) -> bool {
    for token in &tokens[1..] {
        if token == "--" {
            return false;
        }
        if let Some(long_flag) = long
            && (token == long_flag || token.starts_with(&format!("{long_flag}=")))
        {
            return true;
        }
        if let Some(short_flag) = short {
            let short_char = short_flag.trim_start_matches('-');
            if token.starts_with('-')
                && !token.starts_with("--")
                && token[1..].contains(short_char)
            {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(s: &str) -> Token {
        Token(s.to_string())
    }

    fn toks(words: &[&str]) -> Vec<Token> {
        words.iter().map(|s| tok(s)).collect()
    }

    #[test]
    fn has_flag_short() {
        let tokens = toks(&["sed", "-i", "s/foo/bar/"]);
        assert!(has_flag(&tokens, Some("-i"), Some("--in-place")));
    }

    #[test]
    fn has_flag_long_with_eq() {
        let tokens = toks(&["sed", "--in-place=.bak", "s/foo/bar/"]);
        assert!(has_flag(&tokens, Some("-i"), Some("--in-place")));
    }

    #[test]
    fn has_flag_combined_short() {
        let tokens = toks(&["sed", "-ni", "s/foo/bar/p"]);
        assert!(has_flag(&tokens, Some("-i"), Some("--in-place")));
    }

    #[test]
    fn has_flag_stops_at_double_dash() {
        let tokens = toks(&["cmd", "--", "-i"]);
        assert!(!has_flag(&tokens, Some("-i"), Some("--in-place")));
    }

    #[test]
    fn has_flag_long_only() {
        let tokens = toks(&["sort", "--compress-program", "gzip", "file.txt"]);
        assert!(has_flag(&tokens, None, Some("--compress-program")));
    }

    #[test]
    fn has_flag_long_only_eq() {
        let tokens = toks(&["sort", "--compress-program=gzip", "file.txt"]);
        assert!(has_flag(&tokens, None, Some("--compress-program")));
    }

    #[test]
    fn has_flag_long_only_absent() {
        let tokens = toks(&["sort", "-r", "file.txt"]);
        assert!(!has_flag(&tokens, None, Some("--compress-program")));
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
    fn command_name_scoped_package() {
        assert_eq!(tok("@herb-tools/linter").command_name(), "@herb-tools/linter");
    }

    #[test]
    fn reverse_partial_eq() {
        let t = tok("hello");
        assert!("hello" == t);
        assert!("world" != t);
    }

    #[test]
    fn token_deref() {
        let t = tok("--flag");
        assert!(t.starts_with("--"));
        assert!(t.contains("fl"));
        assert_eq!(t.len(), 6);
    }

    #[test]
    fn token_is_one_of() {
        assert!(tok("-v").is_one_of(&["-v", "--verbose"]));
        assert!(!tok("-q").is_one_of(&["-v", "--verbose"]));
    }

    #[test]
    fn token_split_value() {
        assert_eq!(tok("--method=GET").split_value("="), Some("GET"));
        assert_eq!(tok("--flag").split_value("="), None);
    }

    #[test]
    fn word_set_contains() {
        let set = WordSet::new(&["list", "show", "view"]);
        assert!(set.contains(&tok("list")));
        assert!(set.contains(&tok("view")));
        assert!(!set.contains(&tok("delete")));
    }

    #[test]
    fn word_set_iter() {
        let set = WordSet::new(&["a", "b", "c"]);
        let items: Vec<&str> = set.iter().collect();
        assert_eq!(items, vec!["a", "b", "c"]);
    }

    #[test]
    fn content_outside_double_quotes_strips_string() {
        assert_eq!(tok(r#""system""#).content_outside_double_quotes(), " ");
    }

    #[test]
    fn content_outside_double_quotes_preserves_code() {
        let result = tok(r#"{print "hello"} END{print NR}"#).content_outside_double_quotes();
        assert_eq!(result, r#"{print  } END{print NR}"#);
    }

    #[test]
    fn content_outside_double_quotes_escaped() {
        let result = tok(r#"{print "he said \"hi\""}"#).content_outside_double_quotes();
        assert_eq!(result, "{print  }");
    }

    #[test]
    fn content_outside_double_quotes_no_quotes() {
        assert_eq!(tok("{print $1}").content_outside_double_quotes(), "{print $1}");
    }

    #[test]
    fn content_outside_double_quotes_empty() {
        assert_eq!(tok("").content_outside_double_quotes(), "");
    }
}
