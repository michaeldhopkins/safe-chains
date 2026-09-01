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
    // ONE work budget for the whole explanation, taken the same way `command_verdict` takes it.
    //
    // Without this, explaining had no budget of its own: brace-expansion fan-out charged the shared
    // counter while the per-segment classifications inside reset it whenever one bottomed out at
    // depth 0. The result depended on how much the CALLER had already spent and on where the resets
    // fell, so `explain` was neither order-independent (it disagreed with the verdict enforced just
    // before it) nor deterministic (two consecutive calls on one dense input rendered different
    // answers). Entering here resets once, at the top, and keeps every nested classification at
    // depth >= 1, which is what makes explaining and enforcing spend from the same pool.
    let Some(_guard) = super::check::ClassifyGuard::enter() else {
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
    // A culprit is suppressed when it would only repeat the segment's own name: for a lone SIMPLE
    // command the segment text already IS `cat ~/.ssh/id_rsa`, so labelling it `cat` says nothing.
    //
    // That used to be spelled `commands.len() <= 1`, which caught compounds as well — and there the
    // label is the only actionable information there is. The segment text of a denied `for` loop is
    // the whole loop; what the caller has to change is the command inside it, and suppressing that
    // is how a third of the author's decision-log denials came to read "no reason recorded".
    let redundant_with_segment_text = matches!(stmt.pipeline.commands.as_slice(), [Cmd::Simple(_)]);
    let culprit = if verdict.is_allowed() || redundant_with_segment_text {
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
        // `SafeWrite`, the TOP of the auto-approve band — not `Inert`.
        //
        // A `permissions.allow` rule says the user accepts this command. It does NOT say the command
        // is inert, and claiming so was a lie with teeth: `Inert` is the bottom of the ordering, so it
        // cleared every threshold and a `Bash(rm:*)` rule out-ranked even `--level paranoid`. A
        // ceiling a per-command rule can lift is not a ceiling.
        //
        // Granting at the band's top keeps the rule honoured wherever the band is (the default
        // threshold IS `SafeWrite`, so ordinary use is unchanged) while letting a stricter level
        // clamp it: `paranoid` and `reader` now refuse a covered command, which is what someone
        // asking for a read-only plan meant. The grant widens what is allowed; it no longer escapes
        // the ceiling the user stated.
        return Verdict::Allowed(SafetyLevel::SafeWrite);
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

/// The name to report as the culprit for a denied command.
///
/// A compound is not itself a command anyone can act on: the thing the caller has to change lives
/// INSIDE it. So this descends into the body and names the first inner command that is denied on
/// its own — `(cat ~/.ssh/id_rsa)` reports `cat`, not nothing.
///
/// It used to return `None` for everything but `Simple`, which is why a denied `for`/`while`/`case`
/// left `culprit: null` and `facets: null` in the decision log — roughly a third of the denials in
/// the author's own log read "no reason recorded". `--explain` had the same hole, and it is the
/// worse place for it: the hook renders that text back to the agent, so a refusal with no reason is
/// one the agent cannot act on except by guessing.
///
/// Every body is descended, not just the one that will run. Which `if` branch or `case` arm
/// executes is a runtime value, so the classifier already treats such a command as only as safe as
/// its worst body; reporting has to look in the same places or it would name nothing for exactly
/// the constructs that were denied because of what is buried in them.
fn command_label(cmd: &Cmd) -> Option<String> {
    match cmd {
        Cmd::Simple(s) => simple_cmd_name(s),
        // A function DEFINITION is inert — its body only matters when called, and naming the body's
        // commands here would report a culprit for a command that did nothing.
        Cmd::FunctionDef { .. } => None,
        Cmd::Subshell { body, .. } | Cmd::BraceGroup { body, .. } => denied_label_in(body),
        Cmd::For { body, .. } => denied_label_in(body),
        Cmd::While { cond, body, .. } | Cmd::Until { cond, body, .. } => {
            denied_label_in(cond).or_else(|| denied_label_in(body))
        }
        Cmd::If { branches, else_body, .. } => branches
            .iter()
            .find_map(|b| denied_label_in(&b.cond).or_else(|| denied_label_in(&b.body)))
            .or_else(|| else_body.as_ref().and_then(denied_label_in)),
        Cmd::Case { arms, .. } => arms.iter().find_map(|arm| denied_label_in(&arm.body)),
        // `[[ … ]]` is a test expression, not a command that could be the culprit.
        Cmd::DoubleBracket { .. } => None,
    }
}

/// The WORDS of the first denied command inside a compound, for the facet breakdown.
///
/// `--explain`'s profile section tokenises the raw string flatly, which cannot see into a compound:
/// `(cat ~/.ssh/id_rsa)` splits to `["(cat", "~/.ssh/id_rsa)"]`, no resolver recognises `(cat`, and
/// the refusal renders with no reason at all. Roughly a third of the denials in the author's
/// decision log read "no reason recorded" for this shape.
///
/// Returns `None` for a plain simple command, so the caller keeps its existing path and this is
/// only consulted where that path has nothing to say.
pub(crate) fn denied_inner_words(input: &str) -> Option<Vec<String>> {
    let _guard = super::check::ClassifyGuard::enter()?;
    let script = parse(input)?;
    let [stmt] = &script.0[..] else { return None };
    let [cmd] = &stmt.pipeline.commands[..] else { return None };
    // A simple command is already handled by the flat path, and going through the CST for it would
    // change what that path reports on inputs it handles correctly today.
    if matches!(cmd, Cmd::Simple(_)) {
        return None;
    }
    first_denied_simple(cmd)
}

/// The first simple command at or below `cmd` that is denied on its own.
fn first_denied_simple(cmd: &Cmd) -> Option<Vec<String>> {
    match cmd {
        Cmd::Simple(s) => Some(s.words.iter().map(Word::eval).collect()),
        Cmd::FunctionDef { .. } | Cmd::DoubleBracket { .. } => None,
        Cmd::Subshell { body, .. } | Cmd::BraceGroup { body, .. } | Cmd::For { body, .. } => {
            first_denied_simple_in(body)
        }
        Cmd::While { cond, body, .. } | Cmd::Until { cond, body, .. } => {
            first_denied_simple_in(cond).or_else(|| first_denied_simple_in(body))
        }
        Cmd::If { branches, else_body, .. } => branches
            .iter()
            .find_map(|b| {
                first_denied_simple_in(&b.cond).or_else(|| first_denied_simple_in(&b.body))
            })
            .or_else(|| else_body.as_ref().and_then(first_denied_simple_in)),
        Cmd::Case { arms, .. } => arms.iter().find_map(|arm| first_denied_simple_in(&arm.body)),
    }
}

fn first_denied_simple_in(script: &Script) -> Option<Vec<String>> {
    script.0.iter().find_map(|stmt| {
        stmt.pipeline
            .commands
            .iter()
            .find(|c| !cmd_verdict(c).is_allowed())
            .and_then(first_denied_simple)
    })
}

/// The first command inside `script` that is denied on its own, by name.
///
/// Recurses through `command_label`, so a culprit nested several constructs deep is still found.
/// Termination rests on the CST being finite and acyclic — a body is always a strictly smaller
/// subtree than the command containing it — which is the same property the classifier's own walk
/// relies on.
fn denied_label_in(script: &Script) -> Option<String> {
    script.0.iter().find_map(|stmt| {
        stmt.pipeline
            .commands
            .iter()
            .find(|c| !cmd_verdict(c).is_allowed())
            .and_then(command_label)
    })
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
                "This is not a block. It just needs manual approval. Next time send a command that needs approval on its own, not in the same call as commands that auto-approve.",
            );
        }
        if denied == total {
            return Some(
                "This is not a block. These all need manual approval. None of them auto-approve on their own.",
            );
        }
        if self.stateful {
            return Some(
                "This is not a block. The command has likely already run, so this is feedback and not a request to re-run it. These segments share shell state, such as a cd, a variable, or a source, so they belong in one call. Bundling them was correct. Nothing to change.",
            );
        }
        Some(
            "This is not a block. The command has likely already run, so this is feedback and not a request to re-run it. Next time send independent commands as separate tool calls instead of chaining them. The ✓ segments auto-approve on their own, so only a ✗ segment needs approval.",
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
    format!("safe-chains: not auto-approved. {denied} of {total} segments are not on the allowlist:\n")
}

/// One `✓`/`✗` line. The echoed text is command-derived, so it is neutralized first: a raw newline
/// in it let a command forge an entire extra line carrying our own `✓` marker (see
/// [`crate::sanitize_display`]).
fn render_line(s: &SegmentReport) -> String {
    let mark = if s.verdict.is_allowed() { '✓' } else { '✗' };
    let text = crate::sanitize_display(&s.text);
    match &s.culprit {
        Some(culprit) if !s.verdict.is_allowed() => {
            format!("  {mark}  {text}   ({})\n", crate::sanitize_display(culprit))
        }
        _ => format!("  {mark}  {text}\n"),
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

    /// A denied COMPOUND names the command inside it, rather than nothing.
    ///
    /// `command_label` returned `None` for every non-simple command, so a denied `for`/`while`/
    /// `case`/subshell left `culprit: null` in the decision log and no reason in `--explain`.
    /// Roughly a third of the denials in the author's own log read "no reason recorded".
    ///
    /// The construct itself is never the actionable answer — the caller cannot change "a subshell",
    /// only the command in it — so every body is descended, including the branches and arms that
    /// may not run. The classifier already treats such a command as only as safe as its worst body;
    /// reporting looks in the same places, or it names nothing for precisely the constructs whose
    /// denial came from something buried in them.
    #[test]
    fn a_denied_compound_names_the_command_inside_it() {
        for src in [
            "(cat ~/.ssh/id_rsa)",
            "{ cat ~/.ssh/id_rsa; }",
            "if true; then cat ~/.ssh/id_rsa; fi",
            "for f in a b; do cat ~/.ssh/id_rsa; done",
            "while true; do cat ~/.ssh/id_rsa; done",
            "case $x in a) cat ~/.ssh/id_rsa ;; esac",
        ] {
            let ex = explain(src);
            assert_eq!(ex.segments.len(), 1, "{src}: one segment");
            assert!(!ex.is_allowed(), "{src}: denied");
            assert_eq!(
                ex.segments[0].culprit.as_deref(),
                Some("cat"),
                "{src}: must name the command inside the construct"
            );
        }

        // And the words reach the facet breakdown, which is what puts a REASON on the refusal.
        assert_eq!(
            denied_inner_words("(cat ~/.ssh/id_rsa)"),
            Some(vec!["cat".to_string(), "~/.ssh/id_rsa".to_string()]),
        );
        // A plain simple command keeps the existing path — this is only for what it cannot see.
        assert_eq!(denied_inner_words("cat ~/.ssh/id_rsa"), None);
        // A construct whose body is fine has no culprit to name.
        assert_eq!(denied_inner_words("(ls)"), None);
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
