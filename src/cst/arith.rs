//! Variables set from inside a word, where no setter command is named.
//!
//! `HOME=/tmp/x; git log` is judged as an assignment, but the shell also assigns while it expands a
//! word: `${HOME::=/tmp/x}` in zsh, `${HOME:=…}` when empty, and every arithmetic context —
//! `$((HOME=1))`, `$[…]`, a subscript, a `${x:offset}`, `[[ a -eq b ]]`, and in zsh the numeric
//! arguments of `printf %d`, `shift`, `return`, `break` and `continue`. Arithmetic also evaluates a
//! variable's VALUE as an expression, so `x='HOME=1'; echo $((x))` assigns HOME through a name that
//! looks inert. Each such context is refused when it could assign.

mod commands;
mod expression;

use std::cell::RefCell;

pub(crate) use commands::{command_verdict, double_bracket_verdict};
pub(crate) use expression::expression_verdict;
use expression::{SHELL_SET, flag_arithmetic};

use crate::cst::{Cmd, Pipeline, Redir, Script, Stmt, Word, WordPart};
use crate::pathctx::item_shape::{Binding, binding};
use crate::verdict::{SafetyLevel, Verdict};

const INERT: Verdict = Verdict::Allowed(SafetyLevel::Inert);

/// Stands for text no reading settles: the output of a command.
const UNKNOWN: char = '\u{2}';

fn refuse_if(refused: bool) -> Verdict {
    if refused { Verdict::Denied } else { INERT }
}

/// Only a value known to be a number is read as one. A name the line never set holds whatever the
/// shell or its caller gave it, and that text can name a variable the line did set: bash's
/// `HOSTTYPE` holds `aarch64`, so `aarch64='HOME=1'; echo $((HOSTTYPE))` assigns HOME. So can zsh's
/// `argv`, its `functions` table, and a prefix assignment that outlives its command
/// (`a='HOME=1' f`, `a='HOME=1' :`).
fn holds_an_expression(name: &str) -> bool {
    if SHELL_SET.contains(&name) {
        return true;
    }
    match binding(name) {
        Binding::Loop => !counted(name),
        Binding::Value(value) if value == super::scope_budget::UNCERTAIN_VALUE => !counted(name),
        Binding::Value(value) => !is_integer(&value) || !read_as_number(name),
        Binding::Unbound => !ALWAYS_NUMBERS.contains(&name),
    }
}

/// What [`holds_an_expression`] would answer negated, recording nothing.
fn holds_a_number(name: &str) -> bool {
    !SHELL_SET.contains(&name)
        && match binding(name) {
            Binding::Loop => counts(name),
            Binding::Value(value) if value == super::scope_budget::UNCERTAIN_VALUE => counts(name),
            Binding::Value(value) => is_integer(&value) && !given_other_than_a_number(name),
            Binding::Unbound => ALWAYS_NUMBERS.contains(&name),
        }
}

/// A name bound to a number, remembered: a loop's next pass may give it something else written
/// further on, which [`numbers_contradicted`] sees once the line has been walked.
fn read_as_number(name: &str) -> bool {
    NUMBERS.with(|n| n.borrow_mut().push(name.to_string()));
    !given_other_than_a_number(name)
}

fn given_other_than_a_number(name: &str) -> bool {
    FREE.with(|f| f.borrow().iter().any(|x| x == name || x == "*"))
}

/// `counts`, remembering the name: a loop's next pass or a function called again may run an
/// assignment written further on before this read, so [`numbers_contradicted`] asks again once the
/// whole line has been walked.
fn counted(name: &str) -> bool {
    COUNTED.with(|c| c.borrow_mut().push(name.to_string()));
    counts(name)
}

/// A name read as a number somewhere in the line took something else elsewhere in it. Checked by
/// the verdict entry points after the walk, which refuse the line.
pub(super) fn numbers_contradicted() -> bool {
    COUNTED.with(|c| c.borrow().iter().any(|name| !counts(name)))
        || NUMBERS.with(|n| n.borrow().iter().any(|name| given_other_than_a_number(name)))
        || TRUSTED.with(|t| t.borrow().iter().any(|name| crate::cst::functions::ever_defined(name)))
}

/// Every value this line gave `name` was a number, though not one known: `n=$(date +%s)`,
/// `i=$((i + 1))`.
fn counts(name: &str) -> bool {
    !given_other_than_a_number(name) && INTEGER.with(|i| i.borrow().iter().any(|x| x == name))
}

/// One optional sign and at least one digit: `+`, `--5` and the empty string are not numbers, and
/// would build `++` or `--` where they are substituted.
fn is_integer(value: &str) -> bool {
    let value = value.trim();
    let digits = value.strip_prefix(['-', '+']).unwrap_or(value);
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

/// The expansions in one word: `${…}`, `$name[…]`, `$[…]` and `$((…))`. A `$( )` inside is reached
/// by the caller's walk, which hands each nested word here too.
pub(crate) fn word_verdict(word: &Word) -> Verdict {
    let (text, v) = read_text(word);
    v.combine(expansions_verdict(&text))
}

/// The text the shell reads from `word` once its quotes are gone, and the verdict on each `$((…))`
/// inside, which then stands as the number it yields. Quotes do not stop arithmetic from reading
/// `${a["HOME=1"]}`, but quoted text cannot start an expansion or close a bracket, so those
/// characters become `\u{1}`. A substitution's output becomes [`UNKNOWN`].
fn read_text(word: &Word) -> (String, Verdict) {
    let quoted = |c: char| if "$[]{}".contains(c) { '\u{1}' } else { c };
    let mut text = String::new();
    let mut v = INERT;
    for part in &word.0 {
        match part {
            WordPart::Lit(s) => text.push_str(s),
            WordPart::SQuote(s) | WordPart::AnsiC(s) => text.extend(s.chars().map(quoted)),
            WordPart::Escape(c) => text.push(quoted(*c)),
            WordPart::DQuote(inner) => {
                let (inner, inner_v) = read_text(inner);
                text.push_str(&inner);
                v = v.combine(inner_v);
            }
            WordPart::Arith(body) => {
                let (body, body_v) = read_text(body);
                v = v.combine(body_v).combine(expression_verdict(&body));
                text.push('0');
            }
            WordPart::CmdSub(script) => text.push(if numeric_output(script) { '0' } else { UNKNOWN }),
            WordPart::Backtick(body) => {
                let numeric = crate::cst::parse::parse(body).is_some_and(|script| numeric_output(&script));
                text.push(if numeric { '0' } else { UNKNOWN });
            }
            WordPart::ProcSub(_) => text.push(UNKNOWN),
        }
    }
    (text, v)
}

/// A command whose output can only be a number: `date` printing numeric fields (`+%s`), `wc`
/// counting its input, and `seq` counting between integers. Its output stands in arithmetic as the
/// number it is, so the common `$(( $(date +%s) - start ))` keeps working; the command itself is
/// judged where it stands.
fn numeric_output(script: &Script) -> bool {
    // A pipeline prints what its last command prints.
    let [Stmt { pipeline: Pipeline { commands, .. }, op: None }] = script.0.as_slice() else {
        return false;
    };
    let Some(Cmd::Simple(cmd)) = commands.last() else {
        return false;
    };
    let words: Vec<String> = cmd.words.iter().map(Word::eval).collect();
    let reads_only = cmd.env.is_empty() && cmd.redirs.iter().all(|r| matches!(r, Redir::Read { fd: 0, .. }));
    let numeric_format = |w: &String| {
        w.strip_prefix('+').is_some_and(|f| {
            f.split('%').enumerate().all(|(i, part)| {
                let digits = if i == 0 { Some(part) } else { part.strip_prefix(|c: char| "sYmdHMSjuwyNVGC".contains(c)) };
                digits.is_some_and(|d| d.chars().all(|c| c.is_ascii_digit()))
            })
        })
    };
    let numeric = reads_only
        && match words.split_first() {
            Some((name, args)) if name == "date" => args.iter().all(|w| w == "-u" || numeric_format(w)),
            Some((name, args)) if name == "wc" => !args.is_empty() && args.iter().all(|w| matches!(w.as_str(), "-l" | "-c" | "-w" | "-m")),
            Some((name, args)) if name == "seq" => !args.is_empty() && args.iter().all(|w| !w.is_empty() && is_integer(w)),
            _ => false,
        };
    // A function of the same name prints what it likes; one defined later in a loop is found after
    // the walk (`numbers_contradicted`).
    numeric
        && words.first().is_some_and(|name| {
            TRUSTED.with(|t| t.borrow_mut().push(name.clone()));
            !crate::cst::functions::ever_defined(name)
        })
}

/// A word read as arithmetic: `printf %d "$x"`, a side of `[[ a -eq b ]]`.
fn arithmetic_word_verdict(word: &Word) -> Verdict {
    let (text, v) = read_text(word);
    v.combine(expression_verdict(&text))
}

fn expansions_verdict(text: &str) -> Verdict {
    let mut v = INERT;
    for (at, _) in text.match_indices('$') {
        let after = &text[at + 1..];
        if let Some(body) = after.strip_prefix('[') {
            v = v.combine(expression_verdict(bracketed(body, '[', ']')));
        } else if let Some(body) = after.strip_prefix('{') {
            v = v.combine(parameter_verdict(bracketed(body, '{', '}')));
        } else {
            let name = after.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(after.len());
            // A `[` straight after the `$` is `$[…]`, read above, so a subscript here follows a name.
            if let Some(subscript) = after[name..].strip_prefix('[') {
                v = v.combine(expression_verdict(bracketed(subscript, '[', ']')));
            }
        }
    }
    v
}

/// The text up to the bracket closing one already opened, or all of it when none closes.
fn bracketed(text: &str, open: char, close: char) -> &str {
    let mut depth = 1;
    for (i, c) in text.char_indices() {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return &text[..i];
            }
        }
    }
    text
}

/// One `${…}` body. zsh flags (`(l:n:)`), a subscript and a `:offset` are arithmetic; `=`, `:=` and
/// `::=` after the name assign it. A nested `${` is found by the caller's scan of the whole text.
///
/// `${!x}` and zsh's `(P)` read the variable `x` names, and a subscript in that name is arithmetic:
/// `x='a[HOME=1]'; echo ${!x}` assigns HOME. bash's `${x@P}` and zsh's `(e)` flag expand what `x`
/// holds again, substitutions included, so `x='$(touch f)'; echo ${x@P}` runs `touch`.
fn parameter_verdict(body: &str) -> Verdict {
    let mut indirect = body.starts_with('!');
    let mut rest = body.trim_start_matches(['#', '!', '^', '=', '~', '+']);
    let mut v = INERT;
    if let Some(flags) = rest.strip_prefix('(') {
        let flags = bracketed(flags, '(', ')');
        let arithmetic = flag_arithmetic(flags).into_iter().map(expression_verdict);
        v = arithmetic.fold(v, Verdict::combine).combine(refuse_if(flags.contains('e')));
        indirect |= flags.contains('P');
        rest = rest.get(flags.len() + 2..).unwrap_or_default();
    }
    let name = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(rest.len());
    // A special parameter (`${@}`, `${?}`) is one character, whatever it is.
    let name = name.max(rest.chars().next().map_or(0, char::len_utf8));
    let (name, after) = rest.split_at(name);
    rest = after;
    // `${!a[@]}` and `${!prefix*}` list keys and names; they read no variable's value.
    let names_another = indirect && !rest.starts_with(['[', '*', '@']);
    v = v.combine(refuse_if(names_another && !holds_a_name(name)));
    if let Some(subscript) = rest.strip_prefix('[') {
        let subscript = bracketed(subscript, '[', ']');
        v = v.combine(expression_verdict(subscript));
        rest = rest.get(subscript.len() + 2..).unwrap_or_default();
    }
    let assigns = rest.starts_with('=') || rest.starts_with(":=") || rest.starts_with("::=");
    let offset = rest.strip_prefix(':').filter(|r| !r.starts_with(['-', '+', '?', '#', '=', ':']));
    v.combine(refuse_if(assigns || rest.starts_with("@P")))
        .combine(offset.map_or(INERT, expression_verdict))
}

/// The variable holds a plain name, so reading through it evaluates nothing.
fn holds_a_name(name: &str) -> bool {
    match binding(name) {
        Binding::Loop => false,
        Binding::Value(value) => value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        Binding::Unbound => true,
    }
}

thread_local! {
    /// Names this line declared numeric (`declare -i`, zsh's `-E` and `-F`). Every value one takes
    /// is evaluated as arithmetic, so `declare -i y; y=HOME=1` assigns HOME.
    static NUMERIC: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// Names this line gave a value that is not a number, `*` when a command set names not known.
    /// zsh evaluates a variable's value when it is declared numeric, and a loop's next pass runs a
    /// declaration after an assignment written below it, so a declaration checks these.
    static FREE: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// Names this line gave a number it cannot name: a command's count or an arithmetic result.
    static INTEGER: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// Names read as numbers while their value was one this line gave but could not name.
    static COUNTED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// Names read as numbers while bound to a number the line wrote out.
    static NUMBERS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// Commands whose output was read as a number.
    static TRUSTED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Called as an outermost scope walk starts, beside the function registry's reset.
pub(crate) fn forget_numeric_names() {
    NUMERIC.with(|n| n.borrow_mut().clear());
    FREE.with(|f| f.borrow_mut().clear());
    INTEGER.with(|i| i.borrow_mut().clear());
    COUNTED.with(|c| c.borrow_mut().clear());
    NUMBERS.with(|n| n.borrow_mut().clear());
    TRUSTED.with(|t| t.borrow_mut().clear());
}

/// The shells' own integer variables, numeric before any line declares them: bash's and zsh's.
const SHELL_INTEGERS: &[&str] = &[
    "BAUD", "COLUMNS", "DIRSTACKSIZE", "EGID", "ERRNO", "EUID", "FUNCNEST", "GID", "HISTCMD", "HISTSIZE", "KEYTIMEOUT", "LINENO", "LINES",
    "LISTMAX", "MAILCHECK", "OPTIND", "PERIOD", "PPID", "RANDOM", "SAVEHIST", "SECONDS", "SHLVL", "SRANDOM", "TMOUT", "TRY_BLOCK_ERROR",
    "TRY_BLOCK_INTERRUPT", "UID", "ZLE_RPROMPT_INDENT",
];

/// The integers no environment can set to an expression: bash 3.2 and 5 compute these whatever it
/// says, zsh never evaluates what it imports, and dash refuses a value that is not a number. Only
/// an unbound name counts: a string the line assigns is evaluated by bash for `HISTCMD`, `OPTIND`
/// and `SHLVL` (bash 5 also `RANDOM`), and by zsh for `OPTIND`, `RANDOM`, `SECONDS` and `SHLVL`.
/// The rest above are plain strings in some shell (bash takes `UID` and `COLUMNS` as given).
const ALWAYS_NUMBERS: &[&str] = &["HISTCMD", "LINENO", "OPTIND", "PPID", "RANDOM", "SECONDS", "SHLVL"];

fn is_numeric(name: &str) -> bool {
    SHELL_INTEGERS.contains(&name) || NUMERIC.with(|n| n.borrow().iter().any(|x| x == name))
}

/// `name` took a value that is not a number. A name the line never set is not, here: declaring it
/// numeric keeps any later value a number.
fn is_free(name: &str) -> bool {
    given_other_than_a_number(name) || (!matches!(binding(name), Binding::Unbound) && holds_an_expression(name))
}

/// `name` took a value that may not be a number.
fn mark_free(name: &str) {
    FREE.with(|f| f.borrow_mut().push(name.to_string()));
}

/// A value given to `name` that arithmetic would evaluate, if `name` is numeric now or becomes so.
fn value_verdict(name: &str, value: &str) -> Verdict {
    if is_integer(value) {
        return INERT;
    }
    mark_free(name);
    refuse_if(is_numeric(name))
}

/// A value given to `name` by an assignment, read as the shell will produce it.
fn assigned_verdict(name: &str, value: &Word) -> Verdict {
    let text = value.eval();
    let count = matches!(text.as_str(), "$#" | "$?" | "$$" | "$!" | "${#}")
        || text
            .strip_prefix("${#")
            .and_then(|t| t.strip_suffix('}'))
            .is_some_and(|t| !t.contains(['$', '{', '}']));
    if count || is_integer(&read_text(value).0) {
        INTEGER.with(|i| i.borrow_mut().push(name.to_string()));
        return INERT;
    }
    value_verdict(name, &value.eval())
}

/// A loop variable takes each item as its value.
pub(crate) fn loop_verdict(var: &str, items: &[Word]) -> Verdict {
    if items.is_empty() {
        return value_verdict(var, "*");
    }
    let numbers = |item: &Word| item.expand().iter().all(|w| !w.is_empty() && is_integer(w));
    let item_verdict = |item: &Word| {
        if numbers(item) {
            INTEGER.with(|i| i.borrow_mut().push(var.to_string()));
            return INERT;
        }
        assigned_verdict(var, item)
    };
    items.iter().map(item_verdict).fold(INERT, Verdict::combine)
}

#[cfg(test)]
#[path = "arith_tests.rs"]
mod tests;
