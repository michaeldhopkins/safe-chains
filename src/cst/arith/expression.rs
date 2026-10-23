//! What arithmetic text reads: the names in it, the arguments `$@` stands for, and the flags that
//! change what a name reads.

use super::{UNKNOWN, holds_an_expression, refuse_if};
use crate::pathctx::item_shape::{Binding, binding};
use crate::verdict::Verdict;

/// `text` evaluated as arithmetic could change a variable: it holds an assignment operator, `++` or
/// `--`, or names a variable whose value this command line set to something other than a number.
pub(crate) fn expression_verdict(text: &str) -> Verdict {
    let compared = text
        .replace("<<=", "=")
        .replace(">>=", "=")
        .replace("==", "")
        .replace("!=", "")
        .replace("<=", "")
        .replace(">=", "");
    let assigns = compared.contains('=') || compared.contains("++") || compared.contains("--");
    let every_argument = ["$@", "$*", "${@", "${*"].iter().any(|all| text.contains(all));
    refuse_if(
        assigns
            || text.contains(UNKNOWN)
            || reads_through_a_flag(text)
            || builds_an_operator(text)
            || reaches_the_shell_name(text)
            || names(text).iter().any(|name| holds_an_expression(name))
            || (every_argument && any_argument_holds_an_expression()),
    )
}

/// The arguments are not known (`1` unbound: the shell's own, or a `bash -c` string's), a call
/// passed more than are checked, or one of them holds an expression.
fn any_argument_holds_an_expression() -> bool {
    let bound = |i: usize| !matches!(binding(&i.to_string()), Binding::Unbound);
    !bound(1) || bound(MAX_ARGUMENTS + 1) || (1..=MAX_ARGUMENTS).take_while(|&i| bound(i)).any(|i| holds_an_expression(&i.to_string()))
}

/// A flag or modifier between `$` and the name changes what is read: zsh's `$~x`, `$^x`, `$=x`,
/// `${(P)x}` and `${(j: :)@}`, bash's `${!x}`, and `$-` or `${-}`, whose letters read as a name.
fn reads_through_a_flag(text: &str) -> bool {
    text.match_indices('$').any(|(at, _)| {
        let after = &text[at + 1..];
        after.starts_with(['~', '^', '=', '+', '-'])
            || after.strip_prefix('{').is_some_and(|b| b.starts_with(['!', '~', '^', '=', '(', '+', '-']))
    })
}

/// How many of a function's arguments `$@` is checked through; a call passing more is refused.
const MAX_ARGUMENTS: usize = 64;

/// Variables the shell itself sets from what the line holds: the last argument, what `read` or
/// `select` took, a regex match, an option's value.
pub(super) const SHELL_SET: &[&str] = &[
    "_", "REPLY", "reply", "BASH_REMATCH", "MATCH", "match", "MBEGIN", "MEND", "mbegin", "mend", "OPTARG", "MAPFILE", "BASH_COMMAND",
    "BASH_ARGV", "FUNCNAME", "BASH_SOURCE", "PWD", "OLDPWD", "BUFFER", "LBUFFER", "RBUFFER",
];

/// The identifiers in an expression. A run that starts with a digit is a number (`0x1F`, `16#ff`),
/// unless `$` or `${` names an argument by it: `${12}` is all of them, and `$12` is the first in
/// bash and both in zsh. `${#x}` is a length, which reads no value.
fn names(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(|c: char| c.is_ascii_alphanumeric() || c == '_') {
        let run = &rest[start..];
        let len = run.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '#')).unwrap_or(run.len());
        let digits = run.find(|c: char| !c.is_ascii_digit()).unwrap_or(run.len());
        let before = &rest[..start];
        if before.ends_with("${#") {
            rest = &run[len..];
            continue;
        }
        if !run.starts_with(|c: char| c.is_ascii_digit()) {
            out.push(run[..len].trim_end_matches('#'));
        } else if before.ends_with("${") {
            out.push(&run[..digits]);
        } else if before.ends_with('$') {
            out.extend([&run[..1], &run[..digits]]);
        }
        rest = &run[len..];
    }
    out
}

/// A value written straight after `+` or `-` that may carry a sign of its own: `x=-5; $(( n-$x ))`
/// reads `n--5`, which decrements n. A number this line could not name may be negative too.
fn builds_an_operator(text: &str) -> bool {
    text.match_indices('$').any(|(at, _)| {
        let after = text[at + 1..].trim_start_matches('{');
        let name = &after[..after.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(after.len())];
        // `$#`, `$?`, `$$` and `${#x}` count, so they never carry a sign; `$@` is the arguments.
        let special = name.is_empty() && !after.starts_with(['@', '*']);
        text[..at].ends_with(['+', '-'])
            && !special
            && match binding(name) {
                Binding::Value(value) => value.trim().starts_with(['+', '-']) || value == crate::cst::scope_budget::UNCERTAIN_VALUE,
                Binding::Loop | Binding::Unbound => true,
            }
    })
}

/// `${@:0}` and `${*:0:1}` begin at `$0`, the shell's or the function's name, which no call binds.
fn reaches_the_shell_name(text: &str) -> bool {
    ["${@:", "${*:"].iter().any(|all| {
        text.match_indices(all)
            .any(|(at, _)| !text[at + all.len()..].starts_with(|c: char| ('1'..='9').contains(&c)))
    })
}

/// The parts of zsh's parameter flags that are arithmetic: the arguments of `l`, `r` and `I`
/// (`${(l:n::0:)x}`). The other flags are letters or take strings. A delimiter left open, or text
/// that is no flag, leaves the whole of it to be read.
pub(super) fn flag_arithmetic(flags: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = flags;
    while let Some(c) = rest.chars().next() {
        rest = &rest[c.len_utf8()..];
        if !"jslrZIg_".contains(c) {
            if !(c.is_ascii_alphanumeric() || "@#%*~^=+-!".contains(c)) {
                return vec![flags];
            }
            continue;
        }
        let Some(open) = rest.chars().next() else { return vec![flags] };
        let close = match open {
            '(' => ')',
            '[' => ']',
            '{' => '}',
            '<' => '>',
            other => other,
        };
        // `l` and `r` take up to three arguments, each in its own pair of delimiters.
        while let Some(body) = rest.strip_prefix(open) {
            let Some(end) = body.find(close) else { return vec![flags] };
            if "lrI".contains(c) {
                out.push(&body[..end]);
            }
            rest = &body[end + close.len_utf8()..];
        }
    }
    out
}
