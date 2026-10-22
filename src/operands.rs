use crate::{cst, pathctx};

/// A word a command runs with, as the shell will see it at that point in the line: `text` is the
/// spelling with any variable an earlier statement bound put in, and `resolved` the place it names
/// from the directory the shell is in by then.
pub(crate) struct Operand {
    pub(crate) text: String,
    pub(crate) resolved: String,
}

/// The words a command actually RUNS with, for explaining a denial.
///
/// This must agree with the parse the verdict came from, so it walks the CST. Splitting the raw
/// string instead (`shell_words::split`) tokenizes text the shell never treats as an argument —
/// above all a heredoc BODY, which is data. `git commit -m "$(cat <<'EOF' … EOF)"` whose message
/// merely MENTIONS `/etc/hosts` was reported as "reaches /etc/hosts", naming a false reason for the
/// denial and advising the reader to grant that path — a config widening the command never needed.
///
/// Falls back to the raw split only when the command does not parse, where a best-effort nudge on
/// approximate tokens still beats none.
///
/// Each statement's words are read in the scope the statements before it leave, as the verdict
/// reads them. Read alone, `M=/elsewhere; git -C $M status` reaches `$M`, which names nothing, so
/// the refusal named no place and offered no grant, though the verdict had judged `/elsewhere`.
pub(crate) fn operand_words(command: &str) -> Option<Vec<Operand>> {
    let Some(script) = cst::parse(command) else {
        return Some(shell_words::split(command).ok()?.into_iter().map(operand).collect());
    };
    let mut out = Vec::new();
    collect_script_words(&script, &mut out);
    Some(out)
}

/// A variable or a `cd` the walk could not pin stands for a stand-in, not a place, so it is left as
/// written: `M=$X; touch $M/x` names no more than `touch $X/x` does.
fn operand(word: String) -> Operand {
    let opaque = cst::check::is_opaque_value;
    let expanded = pathctx::expand_vars(&word, false).into_owned();
    let text = if opaque(&expanded) && !opaque(&word) { word } else { expanded };
    let resolved = pathctx::resolve(&text).into_owned();
    let resolved = if opaque(&resolved) && !opaque(&text) { text.clone() } else { resolved };
    Operand { text, resolved }
}

/// A word contributes its own expansions AND the words of any command substitution inside it: the
/// inner command runs, so `notacommand $(cat /etc/shadow)` really does read the file, even though
/// `expand()` renders the substitution as an opaque stand-in and hides the path.
fn collect_word(word: &cst::Word, out: &mut Vec<Operand>) {
    out.extend(word.expand().into_iter().map(operand));
    for part in &word.0 {
        collect_part_subs(part, out);
    }
}

/// The words of any command SUBSTITUTION inside a word part, and nothing else — the part's own
/// literal text is the caller's business, because whether it counts as an operand depends on where
/// the word came from (a heredoc body's literal text never does).
fn collect_part_subs(part: &cst::WordPart, out: &mut Vec<Operand>) {
    use cst::WordPart;
    match part {
        WordPart::CmdSub(script) | WordPart::ProcSub(script) => collect_script_words(script, out),
        WordPart::DQuote(inner) => collect_word(inner, out),
        // Arithmetic contributes no operand of its own — its value is a number — but a `$( )`
        // inside it runs, and that command's words are operands the verdict layer classifies.
        WordPart::Arith(inner) => collect_word(inner, out),
        WordPart::Lit(_) | WordPart::Escape(_) | WordPart::SQuote(_) | WordPart::AnsiC(_) | WordPart::Backtick(_) => {}
    }
}

fn collect_script_words(script: &cst::Script, out: &mut Vec<Operand>) {
    cst::check::walk_with_scope(script, |stmt| {
        for cmd in &stmt.pipeline.commands {
            collect_cmd_words(cmd, out);
        }
    });
}

/// A redirect TARGET is a path the command opens, so it is a reach and must be reported —
/// `notacommand > /etc/passwd` names `/etc/passwd`. A heredoc DELIMITER is not a path at all, and
/// its body never appears in the CST, which is the whole point.
fn collect_redir_words(redirs: &[cst::Redir], out: &mut Vec<Operand>) {
    use cst::Redir;
    for redir in redirs {
        match redir {
            Redir::Write { target, .. } | Redir::Read { target, .. } | Redir::ReadWrite { target, .. } | Redir::HereStr(target) => {
                collect_word(target, out)
            }
            // Only the body's SUBSTITUTIONS, never its literal text. Behind a bare delimiter a
            // `$(cat /etc/shadow)` in the body really runs, so it is a reach worth naming; the
            // prose around it is data and naming it would state a false reason for the denial.
            Redir::HereDoc { body, .. } => {
                for part in &body.0 {
                    collect_part_subs(part, out);
                }
            }
            Redir::DupFd { .. } => {}
        }
    }
}

fn collect_cmd_words(cmd: &cst::Cmd, out: &mut Vec<Operand>) {
    use cst::Cmd;
    let words = |ws: &[cst::Word], out: &mut Vec<Operand>| {
        for w in ws {
            collect_word(w, out);
        }
    };
    match cmd {
        Cmd::Simple(s) => {
            words(&s.words, out);
            collect_redir_words(&s.redirs, out);
        }
        Cmd::Subshell { body, redirs } | Cmd::BraceGroup { body, redirs } => {
            collect_script_words(body, out);
            collect_redir_words(redirs, out);
        }
        Cmd::For { items, body, redirs, .. } => {
            words(items, out);
            collect_script_words(body, out);
            collect_redir_words(redirs, out);
        }
        Cmd::While { cond, body, redirs } | Cmd::Until { cond, body, redirs } => {
            collect_script_words(cond, out);
            collect_script_words(body, out);
            collect_redir_words(redirs, out);
        }
        Cmd::If { branches, else_body, redirs } => {
            collect_redir_words(redirs, out);
            for branch in branches {
                collect_script_words(&branch.cond, out);
                collect_script_words(&branch.body, out);
            }
            if let Some(body) = else_body {
                collect_script_words(body, out);
            }
        }
        Cmd::DoubleBracket { words: ws, redirs } => {
            words(ws, out);
            collect_redir_words(redirs, out);
        }
        Cmd::Case { subject, arms, redirs } => {
            collect_word(subject, out);
            for arm in arms {
                collect_script_words(&arm.body, out);
            }
            collect_redir_words(redirs, out);
        }
        Cmd::FunctionDef { body, .. } => collect_script_words(body, out),
    }
}
