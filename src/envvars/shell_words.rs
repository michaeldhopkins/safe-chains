use crate::verdict::{SafetyLevel, Verdict};

pub(super) fn shell_text_verdict(value: &str) -> Verdict {
    if crate::handlers::shell_text::holds_shell_text(value) { Verdict::Denied } else { Verdict::Allowed(SafetyLevel::Inert) }
}

pub(super) const STD_BINS: &[&str] = &["/bin/", "/usr/bin/", "/usr/local/bin/", "/opt/homebrew/bin/"];
pub(super) const SHELLS: &[&str] = &["sh", "bash", "zsh", "dash", "ksh", "tcsh", "csh", "fish"];

/// A shell another program starts: one of the shells by name, in a standard system bin directory.
/// Anything else (a relative, home or temp path, a word with arguments, or another program in a bin
/// directory, such as make reading a Makefile) may run a planted script.
pub(super) fn shell_program_verdict(value: &str) -> Verdict {
    let shell = STD_BINS.iter().find_map(|p| value.strip_prefix(p));
    if shell.is_some_and(|name| SHELLS.contains(&name)) { Verdict::Allowed(SafetyLevel::Inert) } else { Verdict::Denied }
}

#[cfg(test)]
#[path = "shell_words_tests.rs"]
mod tests;
