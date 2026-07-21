use super::*;
use winnow::ModalResult;
use winnow::combinator::{alt, delimited, not, opt, preceded, repeat, separated, terminated};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::{any, take_while};

pub fn parse(input: &str) -> Option<Script> {
    reset_heredoc_queue();
    PARSE_DEPTH.with(|d| d.set(0));
    PARSE_WORK.with(|w| w.set(0));
    PARSE_WORK_LIMIT
        .with(|l| l.set(MAX_PARSE_WORK_BASE + MAX_PARSE_WORK_PER_BYTE * input.len() as u64));
    let result = script.parse(input).ok();
    reset_heredoc_queue();
    result
}

fn backtrack<T>() -> ModalResult<T> {
    Err(ErrMode::Backtrack(ContextError::new()))
}

thread_local! {
    static PARSE_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    /// MONOTONIC work counter for one `parse()` call — every `script()` entry bumps it and it is
    /// never decremented (unlike `PARSE_DEPTH`), so it counts total recursive-descent work, not
    /// concurrent depth. Reset per parse; compared against `PARSE_WORK_LIMIT`.
    static PARSE_WORK: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static PARSE_WORK_LIMIT: std::cell::Cell<u64> = const { std::cell::Cell::new(u64::MAX) };
}

/// Nesting depth beyond which the parser bails instead of recursing further. EVERY recursion source
/// — subshells `( )`, brace groups `{ }`, command/process substitutions `$( )`/`<( )`, and
/// double-quote-nested subs — funnels through `script()`, so bounding it there caps stack depth. A
/// deeply-nested adversarial input (`"$("` × 100 000) would otherwise overflow the stack and ABORT
/// the process — a fail-open CRASH of the hook that `catch_unwind` cannot recover (a stack overflow
/// is not an unwindable panic). 200 is far beyond any real command; past it the parse fails and the
/// command is denied (fail closed). Found by `classifier_terminates_on_adversarial_input`. Kept
/// LOW (winnow's combinator frames are fat — ~200 levels alone overflowed a 2 MB stack), yet still
/// far beyond any real command, which nests a handful of levels at most.
const MAX_PARSE_DEPTH: u32 = 48;

/// Cumulative `script()` entries allowed per parse, as `BASE + PER_BYTE * input.len()`. A correct
/// recursive-descent parse is linear in input length, so this bound is loose for every real command
/// yet trips fast on combinator BACKTRACKING blow-up — inputs where nested constructs make winnow
/// re-parse overlapping tails super-linearly (the `a$(a<(a` × N interleaved-substitution class the
/// depth cap misses because its nesting stays shallow). The balanced-scan in `cmd_sub`/`proc_sub`
/// removes the known source; this is the belt-and-suspenders backstop that fails ANY future
/// exponential closed rather than hanging the hook. Found by `classifier_terminates_on_adversarial_input`.
const MAX_PARSE_WORK_BASE: u64 = 16_384;
const MAX_PARSE_WORK_PER_BYTE: u64 = 512;

/// RAII depth counter for the recursive descent — increments on `enter`, decrements on drop (winnow
/// returns errors rather than panicking, so drops balance even on the bail path). `enter` also bumps
/// the monotonic work counter and bails (→ fail closed) once it exceeds the per-parse work budget.
struct DepthGuard;

impl DepthGuard {
    fn enter() -> Option<Self> {
        let over_budget = PARSE_WORK.with(|w| {
            let n = w.get().saturating_add(1);
            w.set(n);
            n > PARSE_WORK_LIMIT.with(|l| l.get())
        });
        if over_budget {
            return None;
        }
        PARSE_DEPTH.with(|d| {
            if d.get() >= MAX_PARSE_DEPTH {
                None
            } else {
                d.set(d.get() + 1);
                Some(DepthGuard)
            }
        })
    }
}

impl Drop for DepthGuard {
    fn drop(&mut self) {
        PARSE_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    }
}

fn comment(input: &mut &str) -> ModalResult<()> {
    if input.starts_with('#') {
        if let Some(pos) = input.find('\n') {
            *input = &input[pos + 1..];
        } else {
            *input = "";
        }
    }
    Ok(())
}

fn ws(input: &mut &str) -> ModalResult<()> {
    loop {
        take_while(0.., [' ', '\t']).void().parse_next(input)?;
        if input.starts_with('#') {
            comment(input)?;
        } else {
            break;
        }
    }
    Ok(())
}

fn sep(input: &mut &str) -> ModalResult<()> {
    loop {
        take_while(0.., [' ', '\t', ';', '\n']).void().parse_next(input)?;
        if input.starts_with('#') {
            comment(input)?;
        } else {
            break;
        }
    }
    Ok(())
}

fn eat_keyword(input: &mut &str, kw: &str) -> ModalResult<()> {
    if !input.starts_with(kw) {
        return backtrack();
    }
    if input
        .as_bytes()
        .get(kw.len())
        .is_some_and(|&b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return backtrack();
    }
    *input = &input[kw.len()..];
    Ok(())
}

const SCRIPT_STOPS: &[&str] = &["do", "done", "elif", "else", "fi", "then"];

fn at_script_stop(input: &str) -> bool {
    input.starts_with(')')
        || input.starts_with('}')
        || SCRIPT_STOPS.iter().any(|kw| {
            input.starts_with(kw)
                && !input
                    .as_bytes()
                    .get(kw.len())
                    .is_some_and(|&b| b.is_ascii_alphanumeric() || b == b'_')
        })
}

fn is_word_boundary(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | ';' | '|' | '&' | ')' | '>' | '<')
}

fn is_word_literal(c: char) -> bool {
    !is_word_boundary(c) && !matches!(c, '\'' | '"' | '`' | '\\' | '(' | '$')
}

fn is_dq_literal(c: char) -> bool {
    !matches!(c, '"' | '\\' | '`' | '$')
}

// === Script ===

fn script(input: &mut &str) -> ModalResult<Script> {
    // Bound recursion depth: every nested `(`/`{`/`$(`/`<(`/`` ` `` funnels back through `script`,
    // so this one guard caps stack depth against deeply-nested adversarial input (see MAX_PARSE_DEPTH).
    let Some(_depth) = DepthGuard::enter() else {
        return backtrack();
    };
    sep.parse_next(input)?;
    let mut stmts = Vec::new();
    while let Some(pl) = opt(pipeline).parse_next(input)? {
        ws.parse_next(input)?;
        let op = opt(list_op).parse_next(input)?;
        stmts.push(Stmt { pipeline: pl, op });
        // Drain any heredoc bodies pending from this statement before
        // the next pipeline starts; otherwise the body would be parsed
        // as the next statement (which would either misvalidate or
        // misalign the line counter).
        drain_pending_heredocs(input);
        if op.is_none() {
            break;
        }
        sep.parse_next(input)?;
    }
    Ok(Script(stmts))
}

fn list_op(input: &mut &str) -> ModalResult<ListOp> {
    ws.parse_next(input)?;
    alt((
        "&&".value(ListOp::And),
        "||".value(ListOp::Or),
        '\n'.value(ListOp::Semi),
        ';'.value(ListOp::Semi),
        ('&', not('>')).value(ListOp::Amp),
    ))
    .parse_next(input)
}

fn pipe_sep(input: &mut &str) -> ModalResult<()> {
    (ws, '|', not('|'), ws).void().parse_next(input)
}

// === Pipeline ===

fn pipeline(input: &mut &str) -> ModalResult<Pipeline> {
    ws.parse_next(input)?;
    if at_script_stop(input) {
        return backtrack();
    }
    let bang = opt(terminated('!', ws)).parse_next(input)?.is_some();
    let commands: Vec<Cmd> = separated(1.., command, pipe_sep).parse_next(input)?;
    Ok(Pipeline { bang, commands })
}

// === Command ===

fn command(input: &mut &str) -> ModalResult<Cmd> {
    ws.parse_next(input)?;
    if at_script_stop(input) {
        return backtrack();
    }
    alt((
        subshell,
        brace_group,
        for_cmd,
        while_cmd,
        until_cmd,
        if_cmd,
        double_bracket_cmd,
        simple_cmd.map(Cmd::Simple),
    ))
    .parse_next(input)
}

fn trailing_redirs(input: &mut &str) -> ModalResult<Vec<Redir>> {
    let mut redirs = Vec::new();
    loop {
        ws.parse_next(input)?;
        if let Some(r) = opt(redirect).parse_next(input)? {
            redirs.push(r);
        } else {
            break;
        }
    }
    Ok(redirs)
}

fn subshell(input: &mut &str) -> ModalResult<Cmd> {
    let body = delimited(('(', ws), script, (ws, ')')).parse_next(input)?;
    let redirs = trailing_redirs(input)?;
    Ok(Cmd::Subshell { body, redirs })
}

fn brace_group(input: &mut &str) -> ModalResult<Cmd> {
    if !input.starts_with('{') {
        return backtrack();
    }
    if !input
        .as_bytes()
        .get(1)
        .is_some_and(|b| matches!(b, b' ' | b'\t' | b'\n'))
    {
        return backtrack();
    }
    *input = &input[1..];
    sep.parse_next(input)?;
    let body = script.parse_next(input)?;
    if body.0.is_empty() {
        return backtrack();
    }
    sep.parse_next(input)?;
    if !input.starts_with('}') {
        return backtrack();
    }
    let last_op = body.0.last().and_then(|s| s.op);
    if last_op.is_none() {
        return backtrack();
    }
    *input = &input[1..];
    let redirs = trailing_redirs(input)?;
    Ok(Cmd::BraceGroup { body, redirs })
}

// === Simple Command ===

fn simple_cmd(input: &mut &str) -> ModalResult<SimpleCmd> {
    let env: Vec<(String, Word)> =
        repeat(0.., terminated(assignment, ws)).parse_next(input)?;
    let mut words = Vec::new();
    let mut redirs = Vec::new();

    loop {
        ws.parse_next(input)?;
        if at_cmd_end(input) {
            break;
        }
        if let Some(r) = opt(redirect).parse_next(input)? {
            redirs.push(r);
        } else if let Some(w) = opt(word).parse_next(input)? {
            words.push(w);
        } else {
            break;
        }
    }

    if env.is_empty() && words.is_empty() && redirs.is_empty() {
        return backtrack();
    }
    Ok(SimpleCmd { env, words, redirs })
}

fn at_cmd_end(input: &str) -> bool {
    input.is_empty()
        || matches!(
            input.as_bytes().first(),
            Some(b'\n' | b';' | b'|' | b'&' | b')')
        )
}

fn assignment(input: &mut &str) -> ModalResult<(String, Word)> {
    let n: &str = take_while(1.., |c: char| c.is_ascii_alphanumeric() || c == '_')
        .parse_next(input)?;
    '='.parse_next(input)?;
    let value = opt(word)
        .parse_next(input)?
        .unwrap_or(Word(vec![WordPart::Lit(String::new())]));
    Ok((n.to_string(), value))
}

// === Redirect ===

fn redirect(input: &mut &str) -> ModalResult<Redir> {
    let fd = opt(fd_prefix).parse_next(input)?;
    alt((
        preceded("<<<", (ws, word)).map(|(_, target)| Redir::HereStr(target)),
        heredoc,
        preceded(">>", (ws, word)).map(move |(_, target)| Redir::Write {
            fd: fd.unwrap_or(1),
            target,
            append: true,
        }),
        preceded(">&", fd_target).map(move |dst| Redir::DupFd {
            src: fd.unwrap_or(1),
            dst,
        }),
        preceded('>', (ws, word)).map(move |(_, target)| Redir::Write {
            fd: fd.unwrap_or(1),
            target,
            append: false,
        }),
        preceded('<', (ws, word)).map(move |(_, target)| Redir::Read {
            fd: fd.unwrap_or(0),
            target,
        }),
    ))
    .parse_next(input)
}

fn heredoc(input: &mut &str) -> ModalResult<Redir> {
    "<<".parse_next(input)?;
    let strip_tabs = opt('-').parse_next(input)?.is_some();
    ws.parse_next(input)?;
    let delimiter = heredoc_delimiter.parse_next(input)?;
    // Bash semantics: the heredoc body lives on lines AFTER the
    // command line is finished, not immediately after `<<DELIM`. The
    // command line can continue with more redirects, a pipe, etc.
    // Push the delimiter onto a thread-local queue; the body is
    // drained at the next `\n`/`;` separator by drain_pending_heredocs.
    PENDING_HEREDOCS.with(|q| {
        q.borrow_mut().push(PendingHeredoc {
            delimiter: delimiter.clone(),
            strip_tabs,
        });
    });
    Ok(Redir::HereDoc { delimiter, strip_tabs })
}

#[derive(Debug, Clone)]
struct PendingHeredoc {
    delimiter: String,
    strip_tabs: bool,
}

thread_local! {
    static PENDING_HEREDOCS: std::cell::RefCell<Vec<PendingHeredoc>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

fn drain_pending_heredocs(input: &mut &str) {
    let pending: Vec<PendingHeredoc> =
        PENDING_HEREDOCS.with(|q| std::mem::take(&mut *q.borrow_mut()));
    for h in pending {
        if !skip_heredoc_body(input, &h.delimiter, h.strip_tabs) {
            // Couldn't find the matching delimiter line. Leave input
            // as-is; the parser will likely fail on the leftover body
            // text, which is the safe outcome (we deny on parse fail).
            return;
        }
    }
}

fn skip_heredoc_body(input: &mut &str, delimiter: &str, strip_tabs: bool) -> bool {
    let s = *input;
    let bytes = s.as_bytes();
    let mut line_start = 0;
    while line_start <= bytes.len() {
        let line_end = match s[line_start..].find('\n') {
            Some(rel) => line_start + rel,
            None => bytes.len(),
        };
        let line_bytes = &bytes[line_start..line_end];
        let line = if strip_tabs {
            std::str::from_utf8(line_bytes)
                .unwrap_or("")
                .trim_start_matches('\t')
        } else {
            std::str::from_utf8(line_bytes).unwrap_or("")
        };
        if line == delimiter {
            // Advance past the delimiter line + its newline.
            let advance = line_end + usize::from(line_end < bytes.len());
            *input = &s[advance..];
            return true;
        }
        if line_end >= bytes.len() {
            return false;
        }
        line_start = line_end + 1;
    }
    false
}

fn reset_heredoc_queue() {
    PENDING_HEREDOCS.with(|q| q.borrow_mut().clear());
}

fn heredoc_delimiter(input: &mut &str) -> ModalResult<String> {
    alt((
        delimited('\'', take_while(0.., |c| c != '\''), '\'').map(|s: &str| s.to_string()),
        delimited('"', take_while(0.., |c| c != '"'), '"').map(|s: &str| s.to_string()),
        take_while(1.., |c: char| c.is_ascii_alphanumeric() || c == '_').map(|s: &str| s.to_string()),
    ))
    .parse_next(input)
}

fn fd_prefix(input: &mut &str) -> ModalResult<u32> {
    let b = input.as_bytes();
    if b.len() >= 2 && b[0].is_ascii_digit() && matches!(b[1], b'>' | b'<') {
        let d = (b[0] - b'0') as u32;
        *input = &input[1..];
        Ok(d)
    } else {
        backtrack()
    }
}

fn fd_target(input: &mut &str) -> ModalResult<String> {
    alt((
        '-'.value("-".to_string()),
        take_while(1.., |c: char| c.is_ascii_digit()).map(|s: &str| s.to_string()),
    ))
    .parse_next(input)
}

// === Word ===

fn word(input: &mut &str) -> ModalResult<Word> {
    repeat(1.., word_part)
        .map(Word)
        .parse_next(input)
}

fn word_part(input: &mut &str) -> ModalResult<WordPart> {
    if input.is_empty() {
        return backtrack();
    }
    if input.starts_with("<(") || input.starts_with(">(") {
        return proc_sub(input);
    }
    if is_word_boundary(input.as_bytes()[0] as char) {
        return backtrack();
    }
    alt((single_quoted, double_quoted, arith_sub, cmd_sub, backtick_part, escaped, dollar_lit(is_word_literal), lit(is_word_literal)))
        .parse_next(input)
}

fn single_quoted(input: &mut &str) -> ModalResult<WordPart> {
    delimited('\'', take_while(0.., |c| c != '\''), '\'')
        .map(|s: &str| WordPart::SQuote(s.to_string()))
        .parse_next(input)
}

fn double_quoted(input: &mut &str) -> ModalResult<WordPart> {
    delimited('"', repeat(0.., dq_part).map(Word), '"')
        .map(WordPart::DQuote)
        .parse_next(input)
}

/// Byte offset of the `)` that closes a substitution body starting at `body[0]` — the first `)` at
/// paren-depth zero — or `None` if it is never closed. Quote (`'…'`, `"…"`), backtick, and backslash
/// spans are skipped so a `)` inside them does not count, mirroring how the grammar's own
/// `single_quoted`/`double_quoted`/`backtick`/`escaped` parsers treat those regions. This is what
/// keeps `cmd_sub`/`proc_sub` linear: the interior is parsed only once, over a bounded slice, instead
/// of the old `delimited(script, ')')` shape that recursed into the tail BEFORE knowing a close even
/// existed — the source of the `a$(a<(a` × N exponential.
fn find_sub_close(body: &str) -> Option<usize> {
    let b = body.as_bytes();
    let mut i = 0;
    let mut depth: usize = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1, // escape: skip the next byte too (the trailing `+= 1` handles it)
            b'\'' => {
                i += 1;
                while i < b.len() && b[i] != b'\'' {
                    i += 1;
                }
                if i >= b.len() {
                    return None;
                }
            }
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                if i >= b.len() {
                    return None;
                }
            }
            b'`' => {
                i += 1;
                while i < b.len() && b[i] != b'`' {
                    // `bt_escape` treats `\<any>` inside backticks as a literal, so an escaped
                    // backtick does NOT close the span — skip the escaped byte too.
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                if i >= b.len() {
                    return None;
                }
            }
            b'(' => depth += 1,
            b')' => {
                if depth == 0 {
                    return Some(i);
                }
                depth -= 1;
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Parse a substitution body (`$( … )`, `<( … )`, `>( … )`) as a full script.
///
/// FAST PATH: `find_sub_close` locates the matching `)` and we parse only the bounded interior, so
/// nested substitutions stay linear instead of the old `delimited(script, ')')` shape that recursed
/// into the tail before knowing a close existed (the `a$(a<(a` × N exponential).
///
/// FALLBACK: a few grammar constructs move the real close PAST that first balanced `)` — chiefly a
/// heredoc body, whose text (including any `)`) is consumed out-of-band by `drain_pending_heredocs`
/// and which `find_sub_close` does not model. When the bounded interior does not parse cleanly we
/// re-run the EXACT old grammar over the full body, preserving classification for those inputs. The
/// fallback is the recursive shape, but the per-parse work budget (`MAX_PARSE_WORK_*`) bounds it, so
/// it cannot reintroduce the hang. A `None` from `find_sub_close` means no unquoted `)` exists at all
/// — the grammar could not close the sub either — so we fail fast without the fallback.
fn sub_body(input: &mut &str, open_len: usize) -> ModalResult<Script> {
    let body = &input[open_len..];
    let Some(rel) = find_sub_close(body) else {
        return backtrack();
    };
    // A heredoc body is drained out-of-band (`drain_pending_heredocs`) and can run PAST `rel`, so the
    // bounded interior would be truncated mid-heredoc and still parse "clean" — the fast path is
    // unreliable whenever the interior holds a heredoc operator. Skip straight to the grammar fallback
    // there. `<<` covers `<<`, `<<-`, and `<<<`; the latter (herestring) is inline and would be fine,
    // but taking the fallback for it is merely slower, never wrong.
    let interior = &body[..rel];
    if !interior.contains("<<") {
        let mut fast: &str = interior;
        if let Ok(parsed) = script.parse_next(&mut fast) {
            ws.parse_next(&mut fast)?;
            if fast.is_empty() {
                *input = &body[rel + 1..];
                return Ok(parsed);
            }
        }
    }
    let mut rest: &str = body;
    ws.parse_next(&mut rest)?;
    let parsed = script.parse_next(&mut rest)?;
    ws.parse_next(&mut rest)?;
    if !rest.starts_with(')') {
        return backtrack();
    }
    *input = &rest[1..];
    Ok(parsed)
}

fn cmd_sub(input: &mut &str) -> ModalResult<WordPart> {
    if !input.starts_with("$(") {
        return backtrack();
    }
    sub_body(input, 2).map(WordPart::CmdSub)
}

fn proc_sub(input: &mut &str) -> ModalResult<WordPart> {
    if !(input.starts_with("<(") || input.starts_with(">(")) {
        return backtrack();
    }
    sub_body(input, 2).map(WordPart::ProcSub)
}

fn arith_sub(input: &mut &str) -> ModalResult<WordPart> {
    if !input.starts_with("$((") {
        return backtrack();
    }
    let body_start = 3;
    let bytes = input.as_bytes();
    let mut depth: i32 = 1;
    let mut i = body_start;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                if depth == 1 && i + 1 < bytes.len() && bytes[i + 1] == b')' {
                    let body = input[body_start..i].to_string();
                    if body.contains("$(") || body.contains('`') {
                        return backtrack();
                    }
                    *input = &input[i + 2..];
                    return Ok(WordPart::Arith(body));
                }
                depth -= 1;
                if depth < 0 {
                    return backtrack();
                }
            }
            _ => {}
        }
        i += 1;
    }
    backtrack()
}

fn backtick_part(input: &mut &str) -> ModalResult<WordPart> {
    delimited('`', backtick_inner, '`')
        .map(WordPart::Backtick)
        .parse_next(input)
}

fn escaped(input: &mut &str) -> ModalResult<WordPart> {
    preceded('\\', any).map(WordPart::Escape).parse_next(input)
}

fn lit(pred: fn(char) -> bool) -> impl FnMut(&mut &str) -> ModalResult<WordPart> {
    move |input: &mut &str| {
        take_while(1.., pred)
            .map(|s: &str| WordPart::Lit(s.to_string()))
            .parse_next(input)
    }
}

fn dollar_lit(pred: fn(char) -> bool) -> impl FnMut(&mut &str) -> ModalResult<WordPart> {
    move |input: &mut &str| {
        ('$', not('(')).void().parse_next(input)?;
        let rest: &str = take_while(0.., pred).parse_next(input)?;
        Ok(WordPart::Lit(format!("${rest}")))
    }
}

// === Double-quoted parts ===

fn dq_part(input: &mut &str) -> ModalResult<WordPart> {
    if input.is_empty() || input.starts_with('"') {
        return backtrack();
    }
    alt((dq_escape, arith_sub, cmd_sub, backtick_part, dollar_lit(is_dq_literal), lit(is_dq_literal)))
        .parse_next(input)
}

fn dq_escape(input: &mut &str) -> ModalResult<WordPart> {
    preceded('\\', any)
        .map(|c: char| match c {
            '"' | '\\' | '$' | '`' => WordPart::Escape(c),
            _ => WordPart::Lit(format!("\\{c}")),
        })
        .parse_next(input)
}

// === Backtick inner content ===

fn backtick_inner(input: &mut &str) -> ModalResult<String> {
    repeat(0.., alt((bt_escape, bt_literal)))
        .fold(String::new, |mut acc, chunk: &str| {
            acc.push_str(chunk);
            acc
        })
        .parse_next(input)
}

fn bt_escape<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    ('\\', any).take().parse_next(input)
}

fn bt_literal<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    take_while(1.., |c: char| c != '`' && c != '\\').parse_next(input)
}

// === Compound Commands ===

fn for_cmd(input: &mut &str) -> ModalResult<Cmd> {
    eat_keyword(input, "for")?;
    ws.parse_next(input)?;
    let var = name.parse_next(input)?;
    ws.parse_next(input)?;

    let items = if eat_keyword(input, "in").is_ok() {
        ws.parse_next(input)?;
        repeat(0.., terminated(word, ws)).parse_next(input)?
    } else {
        vec![]
    };

    let body = do_done_body.parse_next(input)?;
    let redirs = trailing_redirs(input)?;
    Ok(Cmd::For { var, items, body, redirs })
}

fn while_cmd(input: &mut &str) -> ModalResult<Cmd> {
    eat_keyword(input, "while")?;
    ws.parse_next(input)?;
    let cond = script.parse_next(input)?;
    let body = do_done_body.parse_next(input)?;
    let redirs = trailing_redirs(input)?;
    Ok(Cmd::While { cond, body, redirs })
}

fn until_cmd(input: &mut &str) -> ModalResult<Cmd> {
    eat_keyword(input, "until")?;
    ws.parse_next(input)?;
    let cond = script.parse_next(input)?;
    let body = do_done_body.parse_next(input)?;
    let redirs = trailing_redirs(input)?;
    Ok(Cmd::Until { cond, body, redirs })
}

fn do_done_body(input: &mut &str) -> ModalResult<Script> {
    sep.parse_next(input)?;
    eat_keyword(input, "do")?;
    sep.parse_next(input)?;
    let body = script.parse_next(input)?;
    sep.parse_next(input)?;
    eat_keyword(input, "done")?;
    Ok(body)
}

fn if_cmd(input: &mut &str) -> ModalResult<Cmd> {
    eat_keyword(input, "if")?;
    ws.parse_next(input)?;
    let mut branches = vec![cond_then_body.parse_next(input)?];
    let mut else_body = None;

    loop {
        sep.parse_next(input)?;
        if eat_keyword(input, "elif").is_ok() {
            ws.parse_next(input)?;
            branches.push(cond_then_body.parse_next(input)?);
        } else if eat_keyword(input, "else").is_ok() {
            sep.parse_next(input)?;
            else_body = Some(script.parse_next(input)?);
            break;
        } else {
            break;
        }
    }

    sep.parse_next(input)?;
    eat_keyword(input, "fi")?;
    let redirs = trailing_redirs(input)?;
    Ok(Cmd::If { branches, else_body, redirs })
}

fn cond_then_body(input: &mut &str) -> ModalResult<Branch> {
    let cond = script.parse_next(input)?;
    sep.parse_next(input)?;
    eat_keyword(input, "then")?;
    sep.parse_next(input)?;
    let body = script.parse_next(input)?;
    Ok(Branch { cond, body })
}

fn double_bracket_cmd(input: &mut &str) -> ModalResult<Cmd> {
    if !input.starts_with("[[") {
        return backtrack();
    }
    let bytes = input.as_bytes();
    if bytes.len() < 3 || !matches!(bytes[2], b' ' | b'\t' | b'\n') {
        return backtrack();
    }
    *input = &input[2..];

    let mut words: Vec<Word> = Vec::new();
    loop {
        ws.parse_next(input)?;
        if at_double_bracket_end(input) {
            *input = &input[2..];
            let redirs = trailing_redirs(input)?;
            return Ok(Cmd::DoubleBracket { words, redirs });
        }
        if input.is_empty() {
            return backtrack();
        }
        let w = bracket_word.parse_next(input)?;
        words.push(w);
    }
}

fn at_double_bracket_end(input: &str) -> bool {
    if !input.starts_with("]]") {
        return false;
    }
    let after = &input[2..];
    after.is_empty()
        || after.starts_with(|c: char| {
            matches!(c, ' ' | '\t' | '\n' | ';' | '&' | '|' | ')' | '>' | '<')
        })
}

fn bracket_word(input: &mut &str) -> ModalResult<Word> {
    repeat(1.., bracket_word_part).map(Word).parse_next(input)
}

fn bracket_word_part(input: &mut &str) -> ModalResult<WordPart> {
    if input.is_empty() {
        return backtrack();
    }
    if matches!(input.as_bytes()[0], b' ' | b'\t' | b'\n') {
        return backtrack();
    }
    if at_double_bracket_end(input) {
        return backtrack();
    }
    alt((
        single_quoted,
        double_quoted,
        arith_sub,
        cmd_sub,
        backtick_part,
        escaped,
        dollar_lit(is_bracket_literal),
        bracket_lit,
    ))
    .parse_next(input)
}

fn is_bracket_literal(c: char) -> bool {
    !matches!(c, '\'' | '"' | '`' | '\\' | '$' | ' ' | '\t' | '\n')
}

fn bracket_lit(input: &mut &str) -> ModalResult<WordPart> {
    // Byte-by-byte scan relies on every stop char being single-byte ASCII —
    // multibyte UTF-8 continuation bytes always pass `is_bracket_literal` and
    // get consumed as part of the same `Lit`, so `end` only lands on a char
    // boundary.
    let bytes = input.as_bytes();
    let mut end = 0;
    while end < bytes.len() {
        let c = bytes[end] as char;
        if !is_bracket_literal(c) {
            break;
        }
        if c == ']' && at_double_bracket_end(&input[end..]) {
            break;
        }
        end += 1;
    }
    if end == 0 {
        return backtrack();
    }
    let lit = input[..end].to_string();
    *input = &input[end..];
    Ok(WordPart::Lit(lit))
}

fn name(input: &mut &str) -> ModalResult<String> {
    take_while(1.., |c: char| c.is_ascii_alphanumeric() || c == '_')
        .map(|s: &str| s.to_string())
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(input: &str) -> Script {
        parse(input).unwrap_or_else(|| panic!("failed to parse: {input}"))
    }

    fn words(script: &Script) -> Vec<String> {
        match &script.0[0].pipeline.commands[0] {
            Cmd::Simple(s) => s.words.iter().map(|w| w.eval()).collect(),
            _ => panic!("expected simple command"),
        }
    }

    fn simple(script: &Script) -> &SimpleCmd {
        match &script.0[0].pipeline.commands[0] {
            Cmd::Simple(s) => s,
            _ => panic!("expected simple command"),
        }
    }

    #[test]
    fn simple_command() { assert_eq!(words(&p("echo hello")), ["echo", "hello"]); }
    #[test]
    fn flags() { assert_eq!(words(&p("ls -la")), ["ls", "-la"]); }
    #[test]
    fn single_quoted() { assert_eq!(words(&p("echo 'hello world'")), ["echo", "hello world"]); }
    #[test]
    fn double_quoted() { assert_eq!(words(&p("echo \"hello world\"")), ["echo", "hello world"]); }
    #[test]
    fn mixed_quotes() { assert_eq!(words(&p("jq '.key' file.json")), ["jq", ".key", "file.json"]); }

    #[test]
    fn pipeline_test() { assert_eq!(p("grep foo | head -5").0[0].pipeline.commands.len(), 2); }
    #[test]
    fn sequence_and() { assert_eq!(p("ls && echo done").0[0].op, Some(ListOp::And)); }
    #[test]
    fn sequence_semi() { assert_eq!(p("ls; echo done").0.len(), 2); }
    #[test]
    fn newline_separator() { assert_eq!(p("echo foo\necho bar").0.len(), 2); }
    #[test]
    fn blank_line_between_statements() { assert_eq!(p("echo foo\n\necho bar").0.len(), 2); }
    #[test]
    fn multiple_blank_lines() { assert_eq!(p("echo foo\n\n\n\necho bar").0.len(), 2); }
    #[test]
    fn blank_line_with_whitespace() { assert_eq!(p("echo foo\n   \necho bar").0.len(), 2); }
    #[test]
    fn comment_between_statements() { assert_eq!(p("echo foo\n# comment\necho bar").0.len(), 2); }
    #[test]
    fn semi_then_blank() { assert_eq!(p("echo foo;\n\necho bar").0.len(), 2); }
    #[test]
    fn and_then_blank() { assert_eq!(p("echo foo &&\n\necho bar").0.len(), 2); }

    #[test]
    fn brace_group_simple() {
        assert!(matches!(
            &p("{ echo hello; }").0[0].pipeline.commands[0],
            Cmd::BraceGroup { body, redirs } if body.0.len() == 1 && redirs.is_empty()
        ));
    }
    #[test]
    fn brace_group_multiple_stmts() {
        if let Cmd::BraceGroup { body, .. } = &p("{ echo a; echo b; echo c; }").0[0].pipeline.commands[0] {
            assert_eq!(body.0.len(), 3);
        } else { panic!("expected BraceGroup"); }
    }
    #[test]
    fn brace_group_with_redirect() {
        if let Cmd::BraceGroup { redirs, .. } = &p("{ echo a; echo b; } > /tmp/out.txt").0[0].pipeline.commands[0] {
            assert_eq!(redirs.len(), 1);
            assert!(matches!(redirs[0], Redir::Write { .. }));
        } else { panic!("expected BraceGroup"); }
    }
    #[test]
    fn brace_group_with_append_redirect() {
        if let Cmd::BraceGroup { redirs, .. } = &p("{ echo a; } >> log.txt").0[0].pipeline.commands[0] {
            assert!(matches!(redirs[0], Redir::Write { append: true, .. }));
        } else { panic!("expected BraceGroup"); }
    }
    #[test]
    fn brace_group_with_stderr_redirect() {
        if let Cmd::BraceGroup { redirs, .. } = &p("{ echo a; } 2>&1").0[0].pipeline.commands[0] {
            assert!(matches!(redirs[0], Redir::DupFd { src: 2, .. }));
        } else { panic!("expected BraceGroup"); }
    }
    #[test]
    fn brace_group_newline_separated() {
        if let Cmd::BraceGroup { body, .. } = &p("{\n  echo a\n  echo b\n}").0[0].pipeline.commands[0] {
            assert_eq!(body.0.len(), 2);
        } else { panic!("expected BraceGroup"); }
    }
    #[test]
    fn brace_group_in_pipeline() {
        let pl = &p("{ echo a; echo b; } | grep a").0[0].pipeline;
        assert_eq!(pl.commands.len(), 2);
        assert!(matches!(&pl.commands[0], Cmd::BraceGroup { .. }));
    }
    #[test]
    fn brace_group_followed_by_other() {
        let stmts = &p("{ echo a; }; echo b").0;
        assert_eq!(stmts.len(), 2);
        assert!(matches!(&stmts[0].pipeline.commands[0], Cmd::BraceGroup { .. }));
    }
    #[test]
    fn brace_group_nested() {
        if let Cmd::BraceGroup { body, .. } = &p("{ { echo inner; }; echo outer; }").0[0].pipeline.commands[0] {
            assert_eq!(body.0.len(), 2);
            assert!(matches!(&body.0[0].pipeline.commands[0], Cmd::BraceGroup { .. }));
        } else { panic!("expected outer BraceGroup"); }
    }
    #[test]
    fn brace_group_with_subshell_inside() {
        if let Cmd::BraceGroup { body, .. } = &p("{ (echo sub); echo grp; }").0[0].pipeline.commands[0] {
            assert_eq!(body.0.len(), 2);
            assert!(matches!(&body.0[0].pipeline.commands[0], Cmd::Subshell { .. }));
        } else { panic!("expected BraceGroup"); }
    }
    #[test]
    fn brace_open_requires_whitespace() {
        // {echo (no space) is NOT a brace group; it's a literal word
        // that becomes part of a simple command. Parser should not
        // treat it as a brace group.
        let cmds = &p("{echo a}").0;
        // Either parsed as a simple_cmd with a literal `{echo` token,
        // or fails. Either way, it should NOT be a BraceGroup.
        if !cmds.is_empty() {
            assert!(!matches!(&cmds[0].pipeline.commands[0], Cmd::BraceGroup { .. }));
        }
    }
    #[test]
    fn subshell_with_redirect() {
        if let Cmd::Subshell { redirs, .. } = &p("(echo hello) > /tmp/out.txt").0[0].pipeline.commands[0] {
            assert_eq!(redirs.len(), 1);
        } else { panic!("expected Subshell with redir"); }
    }
    #[test]
    fn for_loop_with_redirect() {
        if let Cmd::For { redirs, .. } = &p("for f in a b; do echo $f; done 2>/dev/null").0[0].pipeline.commands[0] {
            assert_eq!(redirs.len(), 1);
        } else { panic!("expected For with redir"); }
    }
    #[test]
    fn for_loop_redirect_then_pipe() {
        // `done 2>&1 | head` — redirect on the loop, then a pipe.
        let pl = &p("for f in a b; do echo $f; done 2>&1 | head -5").0[0].pipeline;
        assert_eq!(pl.commands.len(), 2);
        assert!(matches!(&pl.commands[0], Cmd::For { redirs, .. } if redirs.len() == 1));
    }
    #[test]
    fn while_and_if_with_redirect() {
        assert!(matches!(
            &p("while true; do echo x; done 2>/dev/null").0[0].pipeline.commands[0],
            Cmd::While { redirs, .. } if redirs.len() == 1
        ));
        assert!(matches!(
            &p("if true; then echo x; fi 2>&1").0[0].pipeline.commands[0],
            Cmd::If { redirs, .. } if redirs.len() == 1
        ));
    }
    #[test]
    fn background() { assert_eq!(p("ls & echo done").0[0].op, Some(ListOp::Amp)); }

    #[test]
    fn redirect_dev_null() {
        let s = p("echo hello > /dev/null");
        let cmd = simple(&s);
        assert_eq!(cmd.words.len(), 2);
        assert!(matches!(&cmd.redirs[0], Redir::Write { fd: 1, append: false, .. }));
    }
    #[test]
    fn redirect_stderr() {
        assert!(matches!(&simple(&p("echo hello 2>&1")).redirs[0], Redir::DupFd { src: 2, dst } if dst == "1"));
    }
    #[test]
    fn here_string() {
        assert!(matches!(&simple(&p("grep -c , <<< 'hello,world,test'")).redirs[0], Redir::HereStr(_)));
    }
    #[test]
    fn heredoc_bare() {
        assert!(matches!(&simple(&p("cat <<EOF")).redirs[0], Redir::HereDoc { delimiter, strip_tabs: false } if delimiter == "EOF"));
    }
    #[test]
    fn heredoc_with_content() {
        let s = p("cat <<EOF\nhello world\nEOF");
        assert!(matches!(&simple(&s).redirs[0], Redir::HereDoc { delimiter, .. } if delimiter == "EOF"));
    }
    #[test]
    fn heredoc_quoted_delimiter() {
        assert!(matches!(&simple(&p("cat <<'EOF'")).redirs[0], Redir::HereDoc { delimiter, .. } if delimiter == "EOF"));
    }
    #[test]
    fn heredoc_strip_tabs() {
        assert!(matches!(&simple(&p("cat <<-EOF")).redirs[0], Redir::HereDoc { strip_tabs: true, .. }));
    }
    #[test]
    fn heredoc_pipe_on_command_line() {
        // Correct bash: pipe is on the command line BEFORE the body,
        // body terminator is on its own line.
        let s = p("cat <<EOF | grep hello\nhello\nEOF");
        assert_eq!(s.0[0].pipeline.commands.len(), 2);
    }
    #[test]
    fn heredoc_body_does_not_swallow_pipe() {
        // Regression for the `cat <<EOF | bash\n...\nEOF` bypass: the
        // heredoc parser must NOT consume the pipe + downstream
        // commands as part of the body.
        let s = p("cat <<EOF | bash\nrm\nEOF");
        assert_eq!(
            s.0[0].pipeline.commands.len(),
            2,
            "pipeline must keep `bash` as a second command"
        );
    }
    #[test]
    fn heredoc_followed_by_next_statement() {
        // After the heredoc body terminator, the script can continue
        // with another statement.
        let s = p("cat <<EOF\nhello\nEOF\nls");
        assert_eq!(s.0.len(), 2);
    }

    #[test]
    fn env_prefix() {
        let s = p("FOO='bar baz' ls -la");
        let cmd = simple(&s);
        assert_eq!(cmd.env[0].0, "FOO");
        assert_eq!(cmd.env[0].1.eval(), "bar baz");
    }
    #[test]
    fn cmd_substitution() { assert!(matches!(&simple(&p("echo $(ls)")).words[1].0[0], WordPart::CmdSub(_))); }
    #[test]
    fn backtick_substitution() { assert_eq!(simple(&p("ls `pwd`")).words[1].eval(), "__SAFE_CHAINS_CMDSUB__"); }
    #[test]
    fn nested_substitution() {
        if let WordPart::CmdSub(inner) = &simple(&p("echo $(echo $(ls))")).words[1].0[0] {
            assert!(matches!(&simple(inner).words[1].0[0], WordPart::CmdSub(_)));
        } else { panic!("expected CmdSub"); }
    }

    #[test]
    fn subshell_test() { assert!(matches!(&p("(echo hello)").0[0].pipeline.commands[0], Cmd::Subshell { .. })); }
    #[test]
    fn negation() { assert!(p("! echo hello").0[0].pipeline.bang); }

    #[test]
    fn for_loop() { assert!(matches!(&p("for x in 1 2 3; do echo $x; done").0[0].pipeline.commands[0], Cmd::For { var, .. } if var == "x")); }
    #[test]
    fn while_loop() { assert!(matches!(&p("while test -f /tmp/foo; do sleep 1; done").0[0].pipeline.commands[0], Cmd::While { .. })); }
    #[test]
    fn if_then_fi() {
        if let Cmd::If { branches, else_body, .. } = &p("if test -f foo; then echo exists; fi").0[0].pipeline.commands[0] {
            assert_eq!(branches.len(), 1);
            assert!(else_body.is_none());
        } else { panic!("expected If"); }
    }
    #[test]
    fn if_elif_else() {
        if let Cmd::If { branches, else_body, .. } = &p("if test -f a; then echo a; elif test -f b; then echo b; else echo c; fi").0[0].pipeline.commands[0] {
            assert_eq!(branches.len(), 2);
            assert!(else_body.is_some());
        } else { panic!("expected If"); }
    }

    #[test]
    fn escaped_outside_quotes() { assert_eq!(words(&p("echo hello\\ world")), ["echo", "hello world"]); }
    #[test]
    fn double_quoted_escape() { assert_eq!(words(&p("echo \"hello\\\"world\"")), ["echo", "hello\"world"]); }
    #[test]
    fn assign_subst() { assert_eq!(simple(&p("out=$(ls)")).env[0].0, "out"); }

    #[test]
    fn unmatched_single_quote_fails() { assert!(parse("echo 'hello").is_none()); }
    #[test]
    fn unmatched_double_quote_fails() { assert!(parse("echo \"hello").is_none()); }
    #[test]
    fn unclosed_subshell_fails() { assert!(parse("(echo hello").is_none()); }
    #[test]
    fn unclosed_cmd_sub_fails() { assert!(parse("echo $(ls").is_none()); }
    #[test]
    fn for_missing_do_fails() { assert!(parse("for x in 1 2 3; echo $x; done").is_none()); }
    #[test]
    fn if_missing_fi_fails() { assert!(parse("if true; then echo hello").is_none()); }

    #[test]
    fn subshell_for() {
        if let Cmd::Subshell { body, .. } = &p("(for x in 1 2; do echo $x; done)").0[0].pipeline.commands[0] {
            assert!(matches!(&body.0[0].pipeline.commands[0], Cmd::For { .. }));
        } else { panic!("expected Subshell"); }
    }
    #[test]
    fn proc_sub_input() {
        let s = p("diff <(sort a.txt) <(sort b.txt)");
        let cmd = simple(&s);
        assert_eq!(cmd.words.len(), 3);
        assert!(matches!(&cmd.words[1].0[0], WordPart::ProcSub(_)));
        assert!(matches!(&cmd.words[2].0[0], WordPart::ProcSub(_)));
    }
    #[test]
    fn proc_sub_output() {
        let s = p("tee >(grep error > /dev/null)");
        let cmd = simple(&s);
        assert_eq!(cmd.words.len(), 2);
        assert!(matches!(&cmd.words[1].0[0], WordPart::ProcSub(_)));
    }
    #[test]
    fn comment_only() {
        let s = p("# just a comment");
        assert!(s.0.is_empty());
    }
    #[test]
    fn comment_before_command() {
        let s = p("# comment\necho hello");
        assert_eq!(words(&s), ["echo", "hello"]);
    }
    #[test]
    fn inline_comment() {
        let s = p("echo hello # this is a comment");
        assert_eq!(words(&s), ["echo", "hello"]);
    }
    #[test]
    fn comment_between_commands() {
        let s = p("echo hello\n# middle comment\necho world");
        assert_eq!(s.0.len(), 2);
    }
    #[test]
    fn comment_after_semicolon() {
        let s = p("echo hello; # comment\necho world");
        assert_eq!(s.0.len(), 2);
    }
    #[test]
    fn comment_in_for_loop() {
        assert!(parse("for x in 1 2; do\n# loop body\necho $x\ndone").is_some());
    }
    #[test]
    fn quoted_redirect_in_echo() {
        let s = p("echo 'greater > than' test");
        let cmd = simple(&s);
        assert_eq!(cmd.words.len(), 3);
        assert_eq!(cmd.redirs.len(), 0);
    }

    #[test]
    fn parses_all_safe_commands() {
        let cmds = [
            "grep foo file.txt", "cat /etc/hosts", "jq '.key' file.json", "base64 -d",
            "ls -la", "wc -l file.txt", "ps aux", "echo hello", "cat file.txt",
            "echo $(ls)", "ls `pwd`", "echo $(echo $(ls))", "echo \"$(ls)\"",
            "out=$(ls)", "out=$(git status)", "a=$(ls) b=$(pwd)",
            "(echo hello)", "(ls)", "(ls && echo done)", "(echo hello; echo world)",
            "(ls | grep foo)", "(echo hello) | grep hello", "(ls) && echo done",
            "((echo hello))", "(for x in 1 2; do echo $x; done)",
            "echo 'greater > than' test", "echo '$(safe)' arg",
            "FOO='bar baz' ls -la", "FOO=\"bar baz\" ls -la",
            "RACK_ENV=test bundle exec rspec spec/foo_spec.rb",
            "grep foo file.txt | head -5", "cat file | sort | uniq",
            "ls && echo done", "ls; echo done", "ls & echo done",
            "grep -c , <<< 'hello,world,test'",
            "cat <<EOF\nhello world\nEOF",
            "cat <<'MARKER'\nsome text\nMARKER",
            "cat <<-EOF\n\thello\nEOF",
            "echo foo\necho bar", "ls\ncat file.txt",
            "git log --oneline -20 | head -5",
            "echo hello > /dev/null", "echo hello 2> /dev/null",
            "echo hello >> /dev/null", "git log > /dev/null 2>&1",
            "ls 2>&1", "cargo clippy 2>&1", "git log < /dev/null",
            "for x in 1 2 3; do echo $x; done",
            "for f in *.txt; do cat $f | grep pattern; done",
            "for x in 1 2 3; do; done",
            "for x in 1 2; do echo $x; done; for y in a b; do echo $y; done",
            "for x in 1 2; do for y in a b; do echo $x $y; done; done",
            "for x in 1 2; do echo $x; done && echo finished",
            "for x in $(seq 1 5); do echo $x; done",
            "while test -f /tmp/foo; do sleep 1; done",
            "while ! test -f /tmp/done; do sleep 1; done",
            "until test -f /tmp/ready; do sleep 1; done",
            "if test -f foo; then echo exists; fi",
            "if test -f foo; then echo yes; else echo no; fi",
            "if test -f a; then echo a; elif test -f b; then echo b; else echo c; fi",
            "for x in 1 2; do if test $x = 1; then echo one; fi; done",
            "if true; then for x in 1 2; do echo $x; done; fi",
            "diff <(sort a.txt) <(sort b.txt)",
            "comm -23 file.txt <(sort other.txt)",
            "cat <(echo hello)",
            "# comment only",
            "# comment\necho hello",
            "echo hello # inline comment",
            "echo one\n# between\necho two",
            "! echo hello", "! test -f foo",
            "echo for; echo done; echo if; echo fi",
        ];
        let mut failures = Vec::new();
        for cmd in &cmds {
            if parse(cmd).is_none() { failures.push(*cmd); }
        }
        assert!(failures.is_empty(), "failed on {} commands:\n{}", failures.len(), failures.join("\n"));
    }

    // === Balanced-scan divergence guards ===
    // `cmd_sub`/`proc_sub` find their closing `)` with `find_sub_close`, then parse the bounded
    // interior. The `roundtrip` proptest exercises this, but its `arb_shell_word` is paren-free, so
    // these lock the case it can't reach: a `)` inside a quote / escape / backtick / nested sub must
    // NOT be mistaken for the substitution's own close.
    fn inner_sub(s: &Script) -> &Script {
        match &simple(s).words[1].0[0] {
            WordPart::CmdSub(inner) | WordPart::ProcSub(inner) => inner,
            other => panic!("expected a substitution, got {other:?}"),
        }
    }

    #[test]
    fn cmd_sub_squote_paren_is_not_close() {
        assert_eq!(words(inner_sub(&p("echo $(echo ')')"))), ["echo", ")"]);
    }
    #[test]
    fn cmd_sub_dquote_paren_is_not_close() {
        assert_eq!(words(inner_sub(&p("echo $(echo \")\")"))), ["echo", ")"]);
    }
    #[test]
    fn cmd_sub_escaped_paren_is_not_close() {
        assert_eq!(words(inner_sub(&p("echo $(echo \\))"))), ["echo", ")"]);
    }
    #[test]
    fn cmd_sub_backtick_paren_is_not_close() {
        // the ) lives inside a backtick span in the sub body; the real close is the final ).
        let s = p("echo $(x `)` y)");
        let inner = simple(inner_sub(&s));
        assert_eq!(inner.words.len(), 3);
        assert!(matches!(&inner.words[1].0[0], WordPart::Backtick(_)));
    }
    #[test]
    fn cmd_sub_escaped_backtick_does_not_end_span() {
        // The `\` escapes the next backtick, so the span — and the ) inside it — belong to the sub
        // body; the real close is the final ). Until find_sub_close honored backtick escapes it
        // mis-placed the span boundary and rejected this (a fail-closed divergence from the grammar).
        let s = p("echo $(`\\`)`)");
        assert!(matches!(&simple(&s).words[1].0[0], WordPart::CmdSub(_)));
    }
    #[test]
    fn proc_sub_squote_paren_is_not_close() {
        assert_eq!(words(inner_sub(&p("cat <(grep ')' f)"))), ["grep", ")", "f"]);
    }
    #[test]
    fn proc_sub_out_squote_paren_is_not_close() {
        assert_eq!(words(inner_sub(&p("tee >(grep ')' f)"))), ["grep", ")", "f"]);
    }
    #[test]
    fn cmd_sub_nested_picks_outer_close() {
        let s = p("echo $(a $(b) c)");
        let inner = simple(inner_sub(&s));
        assert_eq!(inner.words.len(), 3);
        assert!(matches!(&inner.words[1].0[0], WordPart::CmdSub(_)));
    }
    #[test]
    fn cmd_sub_literal_after_close_stays_in_outer_word() {
        let s = p("echo $(ls)tail");
        let w = &simple(&s).words[1];
        assert_eq!(w.0.len(), 2);
        assert!(matches!(&w.0[0], WordPart::CmdSub(_)));
        assert!(matches!(&w.0[1], WordPart::Lit(s) if s == "tail"));
    }
    #[test]
    fn cmd_sub_heredoc_body_paren_does_not_close() {
        // The ) sits in the heredoc body (drained out-of-band), so it must not close the sub — the
        // real close is the final ). find_sub_close can't see heredocs, so sub_body falls back to the
        // full grammar here. This parsed before the balanced-scan rewrite and must keep parsing.
        let s = p("x=$(cat <<EOF\na)b\nEOF\n)");
        assert!(matches!(&simple(&s).env[0].1.0[0], WordPart::CmdSub(_)));
    }
    #[test]
    fn cmd_sub_with_only_a_quoted_paren_is_unclosed() {
        // the sole ) is single-quoted, so the sub never closes → whole parse fails (fail closed).
        assert!(parse("echo $(echo ')").is_none());
    }
    #[test]
    fn proc_sub_with_only_a_quoted_paren_is_unclosed() {
        assert!(parse("cat <(grep ')").is_none());
    }
}
