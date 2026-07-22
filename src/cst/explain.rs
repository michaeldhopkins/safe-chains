use super::check::{cmd_verdict, pipeline_verdict};
use super::*;
use crate::allowlist::{Matcher, is_cmd_covered};
use crate::parse::Token;
use crate::verdict::{SafetyLevel, Verdict};

/// A per-segment breakdown of why a command would or would not auto-approve.
///
/// "Segment" means a top-level list element — the pieces a user separates with
/// `&&`, `||`, `;`, or `&`. This is the granularity that matters for the common
/// failure mode: one un-allowlisted command torpedoing an otherwise-safe chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explanation {
    pub overall: Verdict,
    pub segments: Vec<SegmentReport>,
    /// False when the input could not be parsed at all.
    pub parsed: bool,
    /// True when segments share shell state (a `cd`, `export`, assignment, or
    /// `source`) so that splitting them into separate calls would break them.
    pub stateful: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentReport {
    /// The segment rendered back to source (whitespace/operators normalized).
    pub text: String,
    pub verdict: Verdict,
    /// For a denied *pipeline* segment (`a | b | c`), the name of the first
    /// stage that is not auto-approved — disambiguating which stage to drop.
    /// `None` for a single-command segment (its text already names it) or when
    /// the culprit isn't a plain command (e.g. a subshell or redirect target).
    pub culprit: Option<String>,
}

/// Explain against the built-in classification only.
pub fn explain(input: &str) -> Explanation {
    explain_inner(input, |_| false)
}

/// Explain with the user's allowlist patterns overlaid, so a command the user
/// has allowed isn't reported as not-auto-approved. This mirrors the hook's own
/// coverage check (`main.rs`): a segment counts as allowed when it is built-in
/// safe *or* every command in it is covered by the user's patterns.
pub fn explain_with_coverage(input: &str, patterns: &Matcher) -> Explanation {
    explain_inner(input, |cmd| is_cmd_covered(cmd, patterns))
}

fn explain_inner(input: &str, covered: impl Fn(&Cmd) -> bool) -> Explanation {
    let Some(script) = parse(input) else {
        return Explanation {
            overall: Verdict::Denied,
            segments: vec![SegmentReport {
                text: input.trim().to_string(),
                verdict: Verdict::Denied,
                culprit: None,
            }],
            parsed: false,
            stateful: false,
        };
    };

    // Walk with the SAME accumulated scope as `script_verdict` (cwd + `VAR=` bindings + function
    // definitions), so each segment is judged in the context of the ones before it. Without this the
    // per-segment view — and the hook's coverage fallback built on it — would re-allow a call whose
    // definition shadows a builtin (`ls(){ rm; }; ls`) that the whole-command verdict denies.
    let segments: Vec<SegmentReport> =
        super::check::walk_with_scope(&script, |stmt| segment_report(stmt, &covered));
    let overall = segments
        .iter()
        .map(|s| s.verdict)
        .fold(Verdict::Allowed(SafetyLevel::Inert), Verdict::combine);
    let stateful = segments.len() >= 2 && script.0.iter().any(establishes_shell_state);

    Explanation {
        overall,
        segments,
        parsed: true,
        stateful,
    }
}

fn segment_report(stmt: &Stmt, covered: &impl Fn(&Cmd) -> bool) -> SegmentReport {
    let verdict = effective_verdict(&stmt.pipeline, covered);
    let culprit = if verdict.is_allowed() || stmt.pipeline.commands.len() <= 1 {
        None
    } else {
        first_denied_label(&stmt.pipeline, covered)
    };
    SegmentReport {
        text: stmt.pipeline.to_string(),
        verdict,
        culprit,
    }
}

fn effective_verdict(pipeline: &Pipeline, covered: &impl Fn(&Cmd) -> bool) -> Verdict {
    let base = pipeline_verdict(pipeline);
    if base.is_allowed() {
        return base;
    }
    if !pipeline.commands.is_empty() && pipeline.commands.iter().all(covered) {
        return Verdict::Allowed(SafetyLevel::Inert);
    }
    base
}

fn first_denied_label(pipeline: &Pipeline, covered: &impl Fn(&Cmd) -> bool) -> Option<String> {
    pipeline
        .commands
        .iter()
        .find(|c| !cmd_verdict(c).is_allowed() && !covered(c))
        .and_then(command_label)
}

fn command_label(cmd: &Cmd) -> Option<String> {
    match cmd {
        Cmd::Simple(s) => simple_cmd_name(s),
        _ => None,
    }
}

fn simple_cmd_name(s: &SimpleCmd) -> Option<String> {
    s.words
        .first()
        .map(|w| Token::from_raw(w.eval()).command_name().to_string())
        .filter(|name| !name.is_empty())
}

/// Whether a segment establishes shell state that later segments would rely on:
/// a directory change, an environment change, or a sourced script. Splitting
/// such a chain into separate calls would silently lose that state.
fn establishes_shell_state(stmt: &Stmt) -> bool {
    stmt.pipeline.commands.iter().any(|cmd| match cmd {
        Cmd::Simple(s) => {
            if s.words.is_empty() && !s.env.is_empty() {
                return true;
            }
            matches!(
                simple_cmd_name(s).as_deref(),
                Some("cd" | "pushd" | "popd" | "export" | "source" | "." | "set" | "alias" | "umask")
            )
        }
        _ => false,
    })
}

impl Explanation {
    pub fn is_allowed(&self) -> bool {
        self.overall.is_allowed()
    }

    fn counts(&self) -> (usize, usize) {
        let total = self.segments.len();
        let denied = self
            .segments
            .iter()
            .filter(|s| !s.verdict.is_allowed())
            .count();
        (total, denied)
    }

    /// Whether this explanation is worth injecting into an agent's context
    /// automatically. The teachable case is a *mix*: an otherwise-auto-approving
    /// chain dragged into a manual prompt by one un-allowlisted segment. A single
    /// denied command, or an all-denied chain, carries no chaining lesson — so we
    /// stay quiet and let the normal approval flow handle it.
    pub fn should_surface(&self) -> bool {
        if !self.parsed || self.segments.len() < 2 {
            return false;
        }
        let (total, denied) = self.counts();
        denied > 0 && denied < total
    }

    /// A model- and human-readable breakdown: which segments auto-approve, which
    /// don't, and what to actually do about it.
    pub fn render(&self) -> String {
        if !self.parsed {
            return "safe-chains: could not parse this command, so it will not be auto-approved.\n"
                .to_string();
        }
        if self.segments.is_empty() {
            return "safe-chains: no command to check.\n".to_string();
        }

        let (total, denied) = self.counts();
        let mut out = String::new();
        out.push_str(&header(total, denied));
        for s in &self.segments {
            out.push_str(&render_line(s));
        }
        if let Some(tip) = self.guidance(total, denied) {
            out.push_str(tip);
            out.push('\n');
        }
        out
    }

    fn guidance(&self, total: usize, denied: usize) -> Option<&'static str> {
        if denied == 0 {
            return None;
        }
        // The auto-injected case is always the mixed chain (see should_surface).
        // By the time an agent reads this, the command has gone through the
        // normal approval flow and most likely already run — so the guidance is
        // feedback for next time, never an instruction to re-run.
        if total == 1 {
            return Some(
                "This is not a block — it just needs manual approval. Don't put a command that needs approval in the same call as auto-approving ones.",
            );
        }
        if denied == total {
            return Some(
                "This is not a block — these just need manual approval; none auto-approve on their own.",
            );
        }
        if self.stateful {
            return Some(
                "This is not a block — the command has likely already run, so this is feedback, not a request to re-run. These segments share shell state (a cd, variable, or source), so they belong in one call; bundling was correct here — nothing to change.",
            );
        }
        Some(
            "This is not a block — the command has likely already run, so this is feedback, not a request to re-run. Next time send independent commands as separate tool calls instead of chaining them: the ✓ segments auto-approve on their own, so only a ✗ segment needs approval.",
        )
    }
}

fn header(total: usize, denied: usize) -> String {
    if denied == 0 {
        if total == 1 {
            return "safe-chains: auto-approves.\n".to_string();
        }
        return format!("safe-chains: all {total} segments auto-approve.\n");
    }
    if total == 1 {
        return "safe-chains: this command is not on the allowlist, so it is not auto-approved:\n"
            .to_string();
    }
    format!("safe-chains: not auto-approved — {denied} of {total} segments are not on the allowlist:\n")
}

fn render_line(s: &SegmentReport) -> String {
    let mark = if s.verdict.is_allowed() { '✓' } else { '✗' };
    match &s.culprit {
        Some(culprit) if !s.verdict.is_allowed() => format!("  {mark}  {}   ({culprit})\n", s.text),
        _ => format!("  {mark}  {}\n", s.text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marks(input: &str) -> Vec<bool> {
        explain(input)
            .segments
            .iter()
            .map(|s| s.verdict.is_allowed())
            .collect()
    }

    #[test]
    fn single_safe_command_one_allowed_segment() {
        let e = explain("ls -la");
        assert!(e.is_allowed());
        assert_eq!(e.segments.len(), 1);
        assert!(e.segments[0].verdict.is_allowed());
        assert_eq!(e.segments[0].culprit, None);
    }

    #[test]
    fn single_unsafe_command_is_denied_without_redundant_culprit() {
        let e = explain("rm -rf /");
        assert!(!e.is_allowed());
        assert_eq!(e.segments.len(), 1);
        assert_eq!(e.segments[0].culprit, None);
    }

    #[test]
    fn one_torpedo_marks_only_that_segment() {
        let e = explain("git status && rm -rf / && echo done");
        assert!(!e.is_allowed());
        assert_eq!(marks("git status && rm -rf / && echo done"), vec![true, false, true]);
        assert!(e.segments.iter().all(|s| s.culprit.is_none()));
    }

    #[test]
    fn all_safe_chain_is_allowed() {
        let e = explain("git status && ls && echo hi");
        assert!(e.is_allowed());
        assert_eq!(marks("git status && ls && echo hi"), vec![true, true, true]);
    }

    #[test]
    fn semicolons_and_or_split_into_segments() {
        assert_eq!(explain("ls; pwd; whoami").segments.len(), 3);
        assert_eq!(explain("ls || rm -rf /").segments.len(), 2);
    }

    #[test]
    fn culprit_is_first_denied_in_a_pipeline() {
        let e = explain("grep foo file | rm -rf /");
        assert!(!e.is_allowed());
        assert_eq!(e.segments.len(), 1);
        assert_eq!(e.segments[0].culprit.as_deref(), Some("rm"));
    }

    #[test]
    fn segment_text_round_trips() {
        let e = explain("git status && echo done");
        assert_eq!(e.segments[0].text, "git status");
        assert_eq!(e.segments[1].text, "echo done");
    }

    #[test]
    fn unparseable_input_is_a_single_unparsed_segment() {
        let e = explain("echo 'unterminated");
        assert!(!e.parsed);
        assert!(!e.is_allowed());
    }

    // ---- stateful detection ----

    #[test]
    fn cd_chain_is_marked_stateful() {
        assert!(explain("cd build && rm -rf x").stateful);
        assert!(explain("export FOO=bar && rm -rf x").stateful);
        assert!(explain("FOO=bar && rm -rf x").stateful);
        assert!(explain("source ./env && rm -rf x").stateful);
    }

    #[test]
    fn independent_chain_is_not_stateful() {
        assert!(!explain("git status && rm -rf x && echo done").stateful);
        assert!(!explain("ls && pwd").stateful);
    }

    #[test]
    fn single_segment_is_never_stateful() {
        assert!(!explain("cd build").stateful);
    }

    // ---- should_surface (auto-injection gate) ----

    #[test]
    fn surfaces_only_the_mixed_bundling_case() {
        assert!(explain("git status && rm -rf / && echo done").should_surface());
        assert!(!explain("ls && pwd").should_surface(), "all-safe: nothing to teach");
        assert!(!explain("rm -rf / && rm -rf /etc").should_surface(), "all-denied: no rescue");
        assert!(!explain("rm -rf /").should_surface(), "single denied: no chaining lesson");
        assert!(!explain("echo 'unterminated").should_surface(), "unparseable");
    }

    // ---- coverage overlay ----

    #[test]
    fn coverage_overlay_flips_a_user_allowed_segment() {
        let patterns = Matcher::from_allow_patterns(&["rm *"]);
        let e = explain_with_coverage("git status && rm -rf / && echo done", &patterns);
        assert!(e.is_allowed(), "user allowlisted rm, so the chain auto-approves");
        assert!(e.segments.iter().all(|s| s.verdict.is_allowed()));
        assert!(!e.should_surface());
    }

    #[test]
    fn coverage_overlay_leaves_uncovered_segments_denied() {
        let patterns = Matcher::from_allow_patterns(&["rm *"]);
        let e = explain_with_coverage("rm -rf / && cargo publish", &patterns);
        assert!(!e.is_allowed());
        assert_eq!(marks_cov("rm -rf / && cargo publish", &patterns), vec![true, false]);
    }

    fn marks_cov(input: &str, patterns: &Matcher) -> Vec<bool> {
        explain_with_coverage(input, patterns)
            .segments
            .iter()
            .map(|s| s.verdict.is_allowed())
            .collect()
    }

    // ---- rendering ----

    #[test]
    fn render_mixed_chain_lists_marks_and_split_tip() {
        let out = explain("git status && rm -rf / && echo done").render();
        assert!(out.contains("✓  git status"));
        assert!(out.contains("✗  rm -rf /"));
        assert!(out.contains("✓  echo done"));
        assert!(out.contains("1 of 3 segments"));
        assert!(out.contains("not a block"), "must clarify it is not a block: {out}");
        assert!(out.contains("not a request to re-run"), "must not invite a re-run: {out}");
        assert!(out.contains("separate tool calls"));
    }

    #[test]
    fn render_stateful_chain_says_belongs_in_one_call() {
        let out = explain("cd build && rm -rf / && echo done").render();
        assert!(out.contains("belong in one call"), "stateful chain must not advise splitting: {out}");
        assert!(out.contains("not a request to re-run"));
        assert!(!out.contains("separate tool calls"));
    }

    #[test]
    fn render_pipeline_culprit_disambiguates_failing_stage() {
        let out = explain("grep foo file | rm -rf /").render();
        assert!(out.contains("(rm)"), "pipeline should name the failing stage: {out}");
    }

    #[test]
    fn render_all_safe_has_no_tip() {
        let out = explain("ls && pwd").render();
        assert!(out.contains("all 2 segments auto-approve"));
        assert!(!out.contains('✗'));
        assert!(!out.contains("approval"));
    }

    #[test]
    fn render_single_denied_keeps_it_alone() {
        let out = explain("cargo publish").render();
        assert!(out.contains("not auto-approved"));
        assert!(out.contains("not a block"));
        assert!(out.contains("needs manual approval"));
    }

    #[test]
    fn render_unparseable_is_explicit() {
        let out = explain("echo 'unterminated").render();
        assert!(out.contains("could not parse"));
    }

    #[test]
    fn empty_input_renders_no_command() {
        for input in ["", "   "] {
            let e = explain(input);
            assert!(e.segments.is_empty(), "{input:?} should have no segments");
            assert!(e.render().contains("no command to check"));
        }
    }
}
