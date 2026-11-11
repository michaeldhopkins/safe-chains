//! Whether a command's standard input is fed: by the stage before it in a pipeline, or by an input
//! redirect, here-string or heredoc on it or on a compound command around it.
//!
//! vi reads its keystrokes from a standard input that is not a terminal, so `printf ':!id\n' | vim`
//! runs `id` while the line shows only an editor. Whatever feeds the input reaches every command
//! inside the stage, through `sh -c`, `timeout` and the rest, so the mark lives in a thread-local
//! that nested classifications inherit, as `netargs` does for run-time values.

use std::cell::Cell;

use super::{Cmd, Pipeline, Redir};

thread_local! {
    static FED: Cell<bool> = const { Cell::new(false) };
}

struct FedGuard(bool);

impl Drop for FedGuard {
    fn drop(&mut self) {
        FED.with(|c| c.set(self.0));
    }
}

pub(crate) fn fed() -> bool {
    FED.with(Cell::get)
}

/// Classify one pipeline stage, marked when its standard input is fed. Never clears a mark an
/// enclosing stage set.
pub(super) fn with_stage<T>(pipeline: &Pipeline, position: usize, run: impl FnOnce() -> T) -> T {
    let marked = position > 0 || redirs(&pipeline.commands[position]).iter().any(feeds_stdin);
    let prev = fed();
    FED.with(|c| c.set(prev || marked));
    let _guard = FedGuard(prev);
    run()
}

fn redirs(cmd: &Cmd) -> &[Redir] {
    match cmd {
        Cmd::Simple(s) => &s.redirs,
        Cmd::Subshell { redirs, .. }
        | Cmd::BraceGroup { redirs, .. }
        | Cmd::For { redirs, .. }
        | Cmd::While { redirs, .. }
        | Cmd::Until { redirs, .. }
        | Cmd::If { redirs, .. }
        | Cmd::Case { redirs, .. }
        | Cmd::DoubleBracket { redirs, .. } => redirs,
        Cmd::FunctionDef { .. } => &[],
    }
}

/// `/dev/null` and a closed descriptor give end of input at once, so they feed nothing.
fn feeds_stdin(redir: &Redir) -> bool {
    match redir {
        Redir::Read { fd: 0, target } | Redir::ReadWrite { fd: 0, target } => target.eval() != "/dev/null",
        Redir::HereStr(_) | Redir::HereDoc { .. } => true,
        Redir::DupFd { src: 0, dst } => dst != "-",
        Redir::Read { .. } | Redir::ReadWrite { .. } | Redir::Write { .. } | Redir::DupFd { .. } => false,
    }
}

#[cfg(test)]
#[path = "stdin_fed_tests.rs"]
mod tests;
