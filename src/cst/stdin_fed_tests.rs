use proptest::prelude::*;

use crate::pathctx::{PathCtx, enter};

const ROOT: &str = "/work/u/proj";
const PLUS_COMMANDS: &[&str] = &[
    "less", "more", "vim", "vi", "view", "rvim", "vimdiff", "rview", "ex", "nvim", "zless", "zmore", "xzless", "xzmore", "lzless",
    "lzmore", "bzmore", "bzless", "zstdless",
];
const STDIN_COMMANDS: &[&str] = &["vim", "vi", "view", "rvim", "vimdiff", "rview", "ex", "nvim", "nano", "pico", "rnano"];

fn allowed(cmd: &str) -> bool {
    let _g = enter(PathCtx { cwd: Some(ROOT.into()), root: Some(ROOT.into()), session_id: None });
    crate::command_verdict(cmd).is_allowed()
}

fn fed(command: &str) -> Vec<String> {
    vec![
        format!("printf ':!id\\n' | {command}"),
        format!("echo x | {command} ./notes.md"),
        format!("cat ./keys | {command}"),
        format!("{command} < ./keys"),
        format!("{command} 0< ./keys"),
        format!("{command} <> ./keys"),
        format!("{command} <<< ':!id'"),
        format!("{command} <<'EOF'\n:!id\nEOF"),
        format!("{{ {command}; }} < ./keys"),
        format!("( {command} ./notes.md ) < ./keys"),
        format!("while read -r line; do {command}; done < ./keys"),
        format!("echo x | {{ {command}; }}"),
        format!("echo x | sh -c '{command}'"),
        format!("echo x | timeout 5 {command}"),
        format!("f() {{ {command}; }}; echo x | f"),
    ]
}

fn unfed(command: &str) -> Vec<String> {
    vec![
        command.to_string(),
        format!("{command} ./notes.md"),
        format!("{command} < /dev/null"),
        format!("{command} 3< ./keys"),
        format!("{command} ./notes.md; echo x | cat"),
        format!("{command} ./notes.md | cat"),
    ]
}

#[test]
fn every_listed_command_is_in_the_registry() {
    let names = crate::registry::toml_command_names();
    for command in PLUS_COMMANDS.iter().chain(STDIN_COMMANDS) {
        assert!(names.contains(&crate::registry::canonical_name(command)), "{command} is not registered");
        assert!(allowed(&format!("{command} ./notes.md")), "{command} must open a file: the guards below would pass vacuously");
    }
}

#[test]
fn an_editor_fed_on_standard_input_is_refused_and_one_left_alone_is_not() {
    for command in STDIN_COMMANDS {
        for line in fed(command) {
            assert!(!allowed(&line), "should be refused: {line}");
        }
        for line in unfed(command) {
            assert!(allowed(&line), "should be allowed: {line}");
        }
    }
}

#[test]
fn a_pager_reading_its_input_as_text_still_takes_a_pipe() {
    for command in ["less", "more", "zless", "bzmore", "cat"] {
        assert!(allowed(&format!("echo x | {command}")), "{command}");
    }
}

proptest! {
    /// A `+` operand runs as a command in these pagers and editors, so only a line number or a bare
    /// `+` is approved, wherever it sits among the operands.
    #[test]
    fn only_a_line_number_passes_as_a_plus_operand(
        command in prop::sample::select(PLUS_COMMANDS),
        word in "\\+[!-~]{1,8}",
        line in 0u32..100_000,
        before in any::<bool>(),
    ) {
        let place = |plus: &str| if before { format!("{command} {plus} ./notes.md") } else { format!("{command} ./notes.md {plus}") };
        let clean = word.replace('\'', "");
        let quoted = format!("'{clean}'");
        if clean.len() > 1 && !clean[1..].bytes().all(|b| b.is_ascii_digit()) {
            prop_assert!(!allowed(&place(&quoted)), "should be refused: {}", place(&quoted));
        }
        prop_assert!(allowed(&place(&format!("+{line}"))), "should be allowed: {}", place(&format!("+{line}")));
        prop_assert!(allowed(&place("+")), "should be allowed: {}", place("+"));
    }
}
