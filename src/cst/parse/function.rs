//! Function definitions: `name() { … }`, `name() ( … )` and `function name [()] { … }`.

use super::super::*;
use super::{backtrack, blank, brace_group, eat_keyword, run, subshell, ws};
use winnow::ModalResult;
use winnow::combinator::opt;
use winnow::prelude::*;

/// `name() { body }` / `name() ( body )` (POSIX form) or `function name [()] { body }` (bash form).
/// Tried before `simple_cmd`; a bare `name` with no `()` and no `function` keyword backtracks so an
/// ordinary command is not misread as a definition.
pub(super) fn function_def(input: &mut &str) -> ModalResult<Cmd> {
    let had_keyword = opt_function_keyword(input);
    let name = function_name(input)?;
    ws.parse_next(input)?;
    let has_parens = opt_paren_pair(input);
    if !had_keyword && !has_parens {
        return backtrack();
    }
    // The body may sit on the next line (`foo()\n{ … }`); consume ws/newlines but not `;`.
    blank(input)?;
    let body = function_body(input)?;
    Ok(Cmd::FunctionDef { name, body })
}

/// Consume a leading `function` keyword (must be followed by whitespace, else it's a command named
/// `function`). Returns whether it was present; only commits `*input` when it was.
fn opt_function_keyword(input: &mut &str) -> bool {
    let mut probe = *input;
    if eat_keyword(&mut probe, "function").is_ok() && probe.starts_with([' ', '\t', '\n']) && ws.parse_next(&mut probe).is_ok() {
        *input = probe;
        return true;
    }
    false
}

/// Consume a `(` ws `)` function-def paren pair. Commits `*input` only on a full match.
fn opt_paren_pair(input: &mut &str) -> bool {
    let mut probe = *input;
    if let Some(rest) = probe.strip_prefix('(') {
        probe = rest;
        if ws.parse_next(&mut probe).is_ok()
            && let Some(rest) = probe.strip_prefix(')')
        {
            *input = rest;
            return true;
        }
    }
    false
}

fn function_name(input: &mut &str) -> ModalResult<String> {
    run(input, |c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':' | '+')).map(String::from)
}

/// The compound body of a function — `{ …; }` or `( … )`. A redirect written after it applies on
/// every call (`f() { echo hi; } > ~/.bashrc; f` truncates the file), so a body carrying one keeps
/// the compound whole, redirect included, as a body of one statement.
fn function_body(input: &mut &str) -> ModalResult<Script> {
    let compound = match opt(brace_group).parse_next(input)? {
        Some(cmd) => cmd,
        None => match opt(subshell).parse_next(input)? {
            Some(cmd) => cmd,
            None => return backtrack(),
        },
    };
    match compound {
        Cmd::BraceGroup { body, redirs } | Cmd::Subshell { body, redirs } if redirs.is_empty() => Ok(body),
        cmd => Ok(Script(vec![Stmt { pipeline: Pipeline { bang: false, commands: vec![cmd] }, op: None }])),
    }
}
