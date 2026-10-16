use super::budget;
use super::escape::{self, heredoc_line};

/// A heredoc declared on the current line, whose body is read from the lines after it.
#[derive(Debug, Clone)]
pub(super) struct Pending {
    pub(super) delimiter: String,
    pub(super) strip_tabs: bool,
    pub(super) joins: bool,
    pub(super) start: usize,
}

thread_local! {
    static PENDING: std::cell::RefCell<Vec<Pending>> = const { std::cell::RefCell::new(Vec::new()) };
    static DECLARED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub(super) fn push(heredoc: Pending) {
    DECLARED.with(|d| d.set(d.get() + 1));
    PENDING.with(|q| q.borrow_mut().push(heredoc));
}

pub(super) fn pending() -> bool {
    PENDING.with(|q| !q.borrow().is_empty())
}

pub(super) fn drained() -> bool {
    PENDING.with(|q| q.borrow().iter().all(|h| h.start == 0))
}

pub(super) fn reset() {
    PENDING.with(|q| q.borrow_mut().clear());
}

pub(super) fn skip_blank(input: &mut &str) -> winnow::ModalResult<()> {
    loop {
        while let Some(c @ (' ' | '\t' | '\n')) = input.chars().next() {
            *input = &input[1..];
            if c == '\n' && pending() {
                drain(input);
            }
        }
        if !escape::skip_continuation(input) {
            break;
        }
    }
    Ok(())
}

/// Where a heredoc's body begins: after the current line, and after the bodies of heredocs
/// declared earlier on it (`cat <<A <<B`), so the pending queue is replayed.
pub(super) fn expected_body_start(input: &str) -> Option<&str> {
    let mut rest = &input[input.find('\n')? + 1..];
    let priors: Vec<Pending> = PENDING.with(|q| q.borrow().clone());
    for prior in &priors {
        rest = split_body(rest, &prior.delimiter, prior.strip_tabs, prior.joins)?.1;
    }
    Some(rest)
}

/// Consume the bodies pending at a newline. A body that does not start where it was read when its
/// heredoc was declared refuses: the expansions checked would not be the ones the shell runs.
pub(super) fn drain(input: &mut &str) {
    let pending: Vec<Pending> = PENDING.with(|q| std::mem::take(&mut *q.borrow_mut()));
    for h in pending {
        if input.as_ptr() as usize != h.start {
            escape::refuse();
        }
        let split = split_body(input, &h.delimiter, h.strip_tabs, h.joins);
        let _ = budget::charge(0, split.map_or(input.len(), |(_, rest)| input.len() - rest.len()));
        let Some((_, rest)) = split else {
            // With no delimiter line the shell reads the body to the end of the input, so anything
            // left would be body, not commands, and may expand. Only an empty rest is modelled.
            if !input.trim().is_empty() {
                escape::refuse();
            }
            return;
        };
        *input = rest;
    }
}

/// Split at the delimiter line: `(body, rest-after-the-delimiter-line)`, or `None` when the
/// delimiter never appears. `strip_tabs` (`<<-`) strips leading TABS only, matching the shell —
/// spaces do not terminate a `<<-` body.
pub(super) fn split_body<'a>(s: &'a str, delimiter: &str, strip_tabs: bool, joins: bool) -> Option<(&'a str, &'a str)> {
    let bytes = s.as_bytes();
    let mut line_start = 0;
    while line_start <= bytes.len() {
        let (line_end, line) = heredoc_line(s, line_start, joins);
        let line = if strip_tabs { line.trim_start_matches('\t') } else { &line };
        if line == delimiter {
            let advance = line_end + usize::from(line_end < bytes.len());
            return Some((&s[..line_start], &s[advance..]));
        }
        if line_end >= bytes.len() {
            return None;
        }
        line_start = line_end + 1;
    }
    None
}

/// A substitution is read as its own script: a newline inside it does not reach the heredocs
/// pending on the line around it. One of its own still unread at its close gets an empty body, as
/// bash gives it, and the lines after it are commands. bash reads such a body BEFORE the bodies
/// pending around the substitution, so a heredoc declared inside one while another is pending on
/// the line is refused: the bodies would not be where they are read.
pub(super) fn own<O, E>(p: impl FnOnce() -> Result<O, E>) -> Result<O, E> {
    let outer = PENDING.with(|q| std::mem::take(&mut *q.borrow_mut()));
    let declared = DECLARED.with(std::cell::Cell::get);
    let out = p();
    if !drained() || (outer.iter().any(|h| h.start != 0) && DECLARED.with(std::cell::Cell::get) != declared) {
        escape::refuse();
    }
    PENDING.with(|q| *q.borrow_mut() = outer);
    out
}

#[cfg(test)]
#[path = "heredocs_tests.rs"]
mod tests;
