use super::*;
use winnow::ModalResult;
use winnow::combinator::{alt, delimited, not, opt, preceded, repeat, separated, terminated};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::{any, take_while};

pub fn parse(input: &str) -> Option<Script> {
    script.parse(input).ok()
}

fn backtrack<T>() -> ModalResult<T> {
    Err(ErrMode::Backtrack(ContextError::new()))
}

fn ws(input: &mut &str) -> ModalResult<()> {
    take_while(0.., [' ', '\t']).void().parse_next(input)
}

fn sep(input: &mut &str) -> ModalResult<()> {
    take_while(0.., [' ', '\t', ';', '\n'])
        .void()
        .parse_next(input)
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
    let mut stmts = Vec::new();
    while let Some(pl) = opt(pipeline).parse_next(input)? {
        ws.parse_next(input)?;
        let op = opt(list_op).parse_next(input)?;
        stmts.push(Stmt { pipeline: pl, op });
        if op.is_none() {
            break;
        }
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
        for_cmd,
        while_cmd,
        until_cmd,
        if_cmd,
        simple_cmd.map(Cmd::Simple),
    ))
    .parse_next(input)
}

fn subshell(input: &mut &str) -> ModalResult<Cmd> {
    delimited(('(', ws), script, (ws, ')'))
        .map(Cmd::Subshell)
        .parse_next(input)
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
    let needle = format!("\n{delimiter}");
    if let Some(pos) = input.find(&needle) {
        let after = pos + needle.len();
        *input = input[after..].trim_start_matches([' ', '\t', '\n']);
    }
    Ok(Redir::HereDoc { delimiter, strip_tabs })
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
    if input.is_empty() || is_word_boundary(input.as_bytes()[0] as char) {
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

fn cmd_sub(input: &mut &str) -> ModalResult<WordPart> {
    delimited(("$(", ws), script, (ws, ')'))
        .map(WordPart::CmdSub)
        .parse_next(input)
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
    Ok(Cmd::For { var, items, body })
}

fn while_cmd(input: &mut &str) -> ModalResult<Cmd> {
    eat_keyword(input, "while")?;
    ws.parse_next(input)?;
    let cond = script.parse_next(input)?;
    let body = do_done_body.parse_next(input)?;
    Ok(Cmd::While { cond, body })
}

fn until_cmd(input: &mut &str) -> ModalResult<Cmd> {
    eat_keyword(input, "until")?;
    ws.parse_next(input)?;
    let cond = script.parse_next(input)?;
    let body = do_done_body.parse_next(input)?;
    Ok(Cmd::Until { cond, body })
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
    Ok(Cmd::If { branches, else_body })
}

fn cond_then_body(input: &mut &str) -> ModalResult<Branch> {
    let cond = script.parse_next(input)?;
    sep.parse_next(input)?;
    eat_keyword(input, "then")?;
    sep.parse_next(input)?;
    let body = script.parse_next(input)?;
    Ok(Branch { cond, body })
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
    fn heredoc_then_pipe() {
        let s = p("cat <<EOF\nhello\nEOF | grep hello");
        assert_eq!(s.0[0].pipeline.commands.len(), 2);
    }
    #[test]
    fn heredoc_then_pipe_next_line() {
        let s = p("cat <<EOF\nhello\nEOF\n| grep hello");
        assert_eq!(s.0[0].pipeline.commands.len(), 2);
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
    fn backtick_substitution() { assert_eq!(simple(&p("ls `pwd`")).words[1].eval(), "__SAFE_CHAINS_SUB__"); }
    #[test]
    fn nested_substitution() {
        if let WordPart::CmdSub(inner) = &simple(&p("echo $(echo $(ls))")).words[1].0[0] {
            assert!(matches!(&simple(inner).words[1].0[0], WordPart::CmdSub(_)));
        } else { panic!("expected CmdSub"); }
    }

    #[test]
    fn subshell_test() { assert!(matches!(&p("(echo hello)").0[0].pipeline.commands[0], Cmd::Subshell(_))); }
    #[test]
    fn negation() { assert!(p("! echo hello").0[0].pipeline.bang); }

    #[test]
    fn for_loop() { assert!(matches!(&p("for x in 1 2 3; do echo $x; done").0[0].pipeline.commands[0], Cmd::For { var, .. } if var == "x")); }
    #[test]
    fn while_loop() { assert!(matches!(&p("while test -f /tmp/foo; do sleep 1; done").0[0].pipeline.commands[0], Cmd::While { .. })); }
    #[test]
    fn if_then_fi() {
        if let Cmd::If { branches, else_body } = &p("if test -f foo; then echo exists; fi").0[0].pipeline.commands[0] {
            assert_eq!(branches.len(), 1);
            assert!(else_body.is_none());
        } else { panic!("expected If"); }
    }
    #[test]
    fn if_elif_else() {
        if let Cmd::If { branches, else_body } = &p("if test -f a; then echo a; elif test -f b; then echo b; else echo c; fi").0[0].pipeline.commands[0] {
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
        if let Cmd::Subshell(inner) = &p("(for x in 1 2; do echo $x; done)").0[0].pipeline.commands[0] {
            assert!(matches!(&inner.0[0].pipeline.commands[0], Cmd::For { .. }));
        } else { panic!("expected Subshell"); }
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
            "! echo hello", "! test -f foo",
            "echo for; echo done; echo if; echo fi",
        ];
        let mut failures = Vec::new();
        for cmd in &cmds {
            if parse(cmd).is_none() { failures.push(*cmd); }
        }
        assert!(failures.is_empty(), "failed on {} commands:\n{}", failures.len(), failures.join("\n"));
    }
}
