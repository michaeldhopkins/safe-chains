//! A user's `Bash(...)` rule applies to a simple command wherever the shell runs it, judged in the
//! scope the statements before it leave.
//!
//! Matched only against the top-level commands of a line, a rule did nothing for the same command
//! inside a group, a loop, a branch or a function body, and it also covered a call whose name an
//! earlier definition had rebound: with `Bash(mkdir:*)`, `mkdir(){ rm -rf ~; }; mkdir x` was
//! approved, because the rule matched the call's text and never saw the function it ran.

use proptest::prelude::*;

use super::Matcher;
use crate::cst::{explain, explain_with_coverage};

const RULES: &[&str] = &["terraform apply *", "npm publish *"];
const COVERED: &[&str] = &["terraform apply -auto-approve", "npm publish --dry-run"];
const NOT_COVERED: &str = "curl -X POST https://example.com";

/// Each wraps `{}` in one of the shell's compound forms.
const WRAPPERS: &[&str] = &[
    "{}", "{ {}; }", "( {} )", "if true; then {}; fi", "if false; then :; else {}; fi", "for i in 1 2; do {}; done",
    "while false; do {}; done", "case x in x) {} ;; esac", "true && {}", "echo hi | {}",
];

fn wrap(inner: &str, layers: &[usize]) -> String {
    layers.iter().fold(inner.to_string(), |acc, &i| WRAPPERS[i].replace("{}", &acc))
}

/// `line` as the body of a function the line then calls. A function is defined only at the top: one
/// defined inside a compound is refused whatever its body, since which definition runs is unknown.
fn in_function(line: String, called: bool) -> String {
    if called { format!("g() {{ {line}; }}; g") } else { line }
}

fn rules() -> Matcher {
    Matcher::from_allow_patterns(RULES)
}

fn approved(command: &str) -> bool {
    // A relative `cd` with no starting directory leaves the folder unpinned, and a folder-code
    // tool there asks. These rules are about where a covered command runs in the shell, so judge
    // them from a project directory, where `cd src` stays inside the project.
    let root = format!("{}/projects/scproj", std::env::var("HOME").expect("HOME"));
    let _scope = crate::pathctx::enter(crate::pathctx::PathCtx { cwd: Some(root.clone()), root: Some(root), session_id: None });
    explain_with_coverage(command, &rules()).overall.is_allowed()
}

#[test]
fn a_rule_covers_a_command_after_moving_into_a_project_subdirectory() {
    assert!(approved("cd src; terraform apply -auto-approve"));
    assert!(approved("cd src && npm publish --dry-run"));
}

#[test]
fn the_commands_used_here_need_a_rule() {
    for command in COVERED.iter().chain([&NOT_COVERED]) {
        assert!(!explain(command).overall.is_allowed(), "{command} is approved without any rule");
    }
    for command in COVERED {
        assert!(approved(command), "{command} is not covered by {RULES:?}");
    }
    assert!(!approved(NOT_COVERED));
}

#[test]
fn a_function_that_rebinds_a_covered_name_is_judged_by_its_body() {
    for command in [
        format!("terraform() {{ {NOT_COVERED}; }}; terraform apply -auto-approve"),
        format!("npm() {{ {NOT_COVERED}; }}; npm publish --dry-run"),
        format!("if true; then terraform() {{ {NOT_COVERED}; }}; fi; terraform apply -auto-approve"),
    ] {
        assert!(!approved(&command), "{command}");
    }
}

#[test]
fn a_rule_does_not_reach_into_a_string_another_program_runs() {
    assert!(!approved("bash -c 'terraform apply -auto-approve'"));
    assert!(!approved("find . -name x -exec terraform apply -auto-approve \\;"));
}

#[test]
fn a_rule_does_not_cover_a_command_whose_output_feeds_another() {
    for command in COVERED {
        for line in [format!("echo $({command})"), format!("cat <({command})"), format!("echo `{command}`")] {
            assert!(!approved(&line), "{line}");
        }
    }
}

#[test]
fn a_refused_group_names_the_command_no_rule_covers() {
    let explanation = explain_with_coverage(&format!("{{ terraform apply -auto-approve; {NOT_COVERED}; }}"), &rules());
    assert_eq!(explanation.segments[0].culprit.as_deref(), Some("curl"));
}

#[test]
fn a_refused_group_names_a_call_to_a_function_it_defined() {
    let explanation = explain_with_coverage(&format!("{{ terraform() {{ {NOT_COVERED}; }}; terraform apply -auto-approve; }}"), &rules());
    assert!(!explanation.overall.is_allowed());
    assert!(explanation.segments[0].culprit.is_some(), "{:?}", explanation.segments[0]);
}

proptest! {
    #[test]
    fn a_covered_command_is_approved_inside_any_compound(
        command in proptest::sample::select(COVERED),
        layers in proptest::collection::vec(0..WRAPPERS.len(), 0..4),
        called in any::<bool>(),
    ) {
        let line = in_function(wrap(command, &layers), called);
        prop_assert!(approved(&line), "{line}");
    }

    #[test]
    fn a_command_no_rule_covers_still_refuses_the_compound(
        command in proptest::sample::select(COVERED),
        layers in proptest::collection::vec(0..WRAPPERS.len(), 0..4),
        uncovered_first in any::<bool>(),
        called in any::<bool>(),
    ) {
        let inner = if uncovered_first { format!("{NOT_COVERED}; {command}") } else { format!("{command}; {NOT_COVERED}") };
        let line = in_function(wrap(&inner, &layers), called);
        prop_assert!(!approved(&line), "{line}");
    }

    #[test]
    fn a_rebound_name_is_never_covered_by_a_rule_for_the_command(
        name in proptest::sample::select(&["terraform", "npm"][..]),
        layers in proptest::collection::vec(0..WRAPPERS.len(), 0..4),
    ) {
        let call = COVERED.iter().find(|c| c.starts_with(name)).copied().unwrap_or_default();
        let line = format!("{name}() {{ {NOT_COVERED}; }}; {}", wrap(call, &layers));
        prop_assert!(!approved(&line), "{line}");
    }
}

/// A rule covers a command the built-in rules refuse. It does not cover one a path gate or a
/// foreign folder refuses: those name a place, and the rule names a command.
#[test]
fn a_rule_does_not_skip_a_path_gate_or_a_foreign_folder() {
    let root = format!("{}/projects/scproj", std::env::var("HOME").expect("HOME"));
    let rules = Matcher::from_allow_patterns(&["npm test", "npm run:*", "tee:*"]);
    let under = |cwd: &str, command: &str, covered: bool| {
        let _scope =
            crate::pathctx::enter(crate::pathctx::PathCtx { cwd: Some(cwd.to_string()), root: Some(root.clone()), session_id: None });
        if covered { explain_with_coverage(command, &rules).overall.is_allowed() } else { explain(command).overall.is_allowed() }
    };

    assert!(!under(&root, "npm run deploy", false), "npm run deploy needs the rule");
    assert!(under(&root, "npm run deploy", true), "an in-project script the rule names stays covered");
    assert!(under(&root, "npm test", true), "npm test in the project");
    assert!(under(&root, "tee ./out.log", true), "tee of a worktree file");

    for command in ["cd /tmp/y && npm test", "tee /etc/hosts"] {
        assert!(!under(&root, command, false), "{command} is approved with no rule");
        assert!(!under(&root, command, true), "{command} was approved by a rule");
    }
}
