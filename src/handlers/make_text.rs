//! make text that comes from the command line instead of the project's makefile.
//!
//! GNU make evaluates a command-line variable assignment (`A:=$(shell id)`, `MAKEFLAGS=p`) before
//! it reads a rule, under `-n` too, and a makefile read from standard input (`-f -`,
//! `-f /dev/stdin`) is whatever the line pipes in. Neither is a file the project's commits show.

use crate::parse::Token;

/// Whether a make line hands make text to evaluate: an assignment operand, or a makefile read from
/// standard input or an inherited descriptor.
pub(crate) fn evaluates_text_from_the_line(tokens: &[Token]) -> bool {
    tokens.iter().skip(1).map(Token::as_str).any(|word| {
        if reads_stdin(word) {
            return true;
        }
        if let Some(long) = word.strip_prefix("--") {
            return long.split_once('=').is_some_and(|(_, value)| reads_stdin(value));
        }
        if let Some(cluster) = word.strip_prefix('-') {
            return cluster.find('f').is_some_and(|at| reads_stdin(&cluster[at + 1..]));
        }
        word.contains('=')
    })
}

fn reads_stdin(name: &str) -> bool {
    name == "-" || name == "/dev/stdin" || name.starts_with("/dev/fd/") || name.starts_with("/proc/self/fd/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn refused(line: &str) -> bool {
        let tokens: Vec<Token> = line.split_whitespace().map(Token::from_test).collect();
        evaluates_text_from_the_line(&tokens)
    }

    #[test]
    fn a_plain_dry_run_hands_make_no_text() {
        for line in
            ["make -n", "make -n all", "make -nq", "make -n -C src", "make -n -f Makefile", "make --file=build.mk -n", "make -j4 -n"]
        {
            assert!(!refused(line), "{line}");
        }
    }

    proptest! {
        /// Every assignment operand, in any of make's assignment operators and wherever it sits, is
        /// text make evaluates.
        #[test]
        fn an_assignment_operand_is_refused(
            name in "[A-Za-z_][A-Za-z0-9_]{0,7}",
            op in prop::sample::select(vec!["=", ":=", "::=", ":::=", "+=", "?=", "!="]),
            value in "[a-z$()]{0,8}",
            before in prop::sample::select(vec!["-n", "-n all", "-nq", "-n -f Makefile"]),
        ) {
            let line = format!("make {before} {name}{op}{value}");
            prop_assert!(refused(&line), "{}", line);
        }

        /// A makefile named as standard input or a descriptor, in every spelling of -f, is refused.
        #[test]
        fn a_makefile_from_stdin_is_refused(
            name in prop::sample::select(vec!["-", "/dev/stdin", "/dev/fd/0", "/dev/fd/3", "/proc/self/fd/0"]),
            form in prop::sample::select(vec!["-f {}", "-f{}", "-nf {}", "-nf{}", "--file={}", "--file {}", "--makefile={}", "--makefile {}"]),
        ) {
            let line = format!("make -n {}", form.replace("{}", name));
            prop_assert!(refused(&line), "{}", line);
        }

        /// A target name is never mistaken for text to evaluate.
        #[test]
        fn a_target_is_not_refused(targets in prop::collection::vec("[a-z][a-z0-9_.-]{0,8}", 0..4)) {
            let line = format!("make -n {}", targets.join(" "));
            prop_assert!(!refused(&line), "{}", line);
        }
    }
}
