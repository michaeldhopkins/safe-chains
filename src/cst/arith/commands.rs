//! The commands that read an argument as arithmetic, or that set or test a variable by a name that
//! may carry a subscript.

use super::{
    INERT, INTEGER, NUMERIC, arithmetic_word_verdict, assigned_verdict, bracketed, expression_verdict, holds_an_expression, is_free,
    is_integer, is_numeric, read_text, refuse_if, value_verdict,
};
use crate::cst::{SimpleCmd, Word};
use crate::parse::Token;
use crate::verdict::Verdict;

fn numeric_verdict(cmd: &SimpleCmd, name: &str, args: &[String]) -> Verdict {
    let declares = matches!(name, "declare" | "typeset" | "local" | "export" | "readonly")
        && args.iter().any(|w| w.starts_with(['-', '+']) && w.contains(['i', 'E', 'F']));
    let env = cmd.env.iter().map(|(n, value)| assigned_verdict(n, value)).fold(INERT, Verdict::combine);
    if declares {
        let operands = args.iter().filter(|w| !w.starts_with(['-', '+']));
        let refused: Vec<bool> = operands
            .map(|w| w.split_once('=').map_or((w.as_str(), None), |(n, value)| (n, Some(value))))
            .map(|(n, value)| {
                // zsh evaluates the value a name already holds when it is declared without one, and
                // `+=` adds to it.
                let reads_old = value.is_none() || n.ends_with('+');
                let n = n.trim_end_matches('+');
                let refused = is_free(n) || !is_integer(value.unwrap_or_default()) || (reads_old && holds_an_expression(n));
                NUMERIC.with(|numeric| numeric.borrow_mut().push(n.to_string()));
                if !refused {
                    INTEGER.with(|i| i.borrow_mut().push(n.to_string()));
                }
                refused
            })
            .collect();
        return env.combine(refuse_if(refused.contains(&true)));
    }
    let set = match super::super::opaque::named(cmd) {
        Some(names) => names.iter().map(|n| declared_verdict(name, n, args)).fold(INERT, Verdict::combine),
        // Which names are set is not known: any numeric one could take an expression.
        None => {
            let names_numeric = args.iter().any(|w| is_numeric(w.split('=').next().unwrap_or_default()));
            value_verdict("*", "*").combine(refuse_if(names_numeric || NUMERIC.with(|n| !n.borrow().is_empty())))
        }
    };
    env.combine(set)
}

/// One name a setter sets: `local n=0` gives it a number; `read n`, `local n=$x` and `n+=…` may give
/// it anything.
fn declared_verdict(command: &str, name: &str, args: &[String]) -> Verdict {
    let assigns = matches!(command, "declare" | "typeset" | "local" | "export" | "readonly");
    let mut values = args
        .iter()
        .filter(|w| !w.starts_with(['-', '+']) && w.split('=').next().unwrap_or_default().trim_end_matches('+') == name)
        .map(|w| w.strip_prefix(name).and_then(|rest| rest.strip_prefix('=')))
        .peekable();
    let numbers = values.peek().is_some() && values.all(|v| v.is_some_and(|v| !v.is_empty() && is_integer(v)));
    if assigns && numbers {
        INTEGER.with(|i| i.borrow_mut().push(name.to_string()));
        return INERT;
    }
    value_verdict(name, "*")
}

/// The commands whose arguments are arithmetic, assignments through a subscript (`a[x]=1`), values
/// given to a numeric variable, and the names a command sets or tests.
pub(crate) fn command_verdict(cmd: &SimpleCmd) -> Verdict {
    let subscripts = cmd.env.iter().filter_map(|(name, _)| name.split_once('[')).map(|(_, s)| expression_verdict(s));
    let words: Vec<String> = cmd.words.iter().map(Word::eval).collect();
    let start = words.iter().take_while(|w| matches!(command_name(w).as_str(), "builtin" | "command")).count();
    let descriptor = descriptor_verdict(cmd);
    let Some(name) = words.get(start).map(|w| command_name(w)) else {
        return subscripts.fold(numeric_verdict(cmd, "", &[]).combine(descriptor), Verdict::combine);
    };
    let args = &words[start + 1..];
    let arg_words = &cmd.words[start + 1..];
    let numeric = numeric_verdict(cmd, &name, args);
    let arithmetic: Vec<usize> = match name.as_str() {
        "shift" | "return" | "break" | "continue" => (0..args.len()).collect(),
        "printf" => formatted(args, |w| w == "-v"),
        "print" if args.iter().any(|w| w.starts_with('-') && w.contains('f')) => formatted(args, |w| w.contains('f')),
        "read" => option_values(args, &['t', 'k', 'u']),
        _ => Vec::new(),
    };
    let named: Vec<usize> = match name.as_str() {
        "read" | "getopts" | "unset" | "mapfile" | "readarray" => (0..args.len()).collect(),
        "test" | "[" => tested_names(args),
        _ => Vec::new(),
    };
    let arithmetic = arithmetic.into_iter().map(|i| arithmetic_word_verdict(&arg_words[i]));
    let named = named.into_iter().map(|i| name_verdict(&arg_words[i]));
    arithmetic.chain(named).chain(subscripts).fold(numeric.combine(descriptor), Verdict::combine)
}

/// `echo hi {HOME}>/dev/null` opens a descriptor and assigns its number to HOME, in bash and zsh.
/// The parser keeps the `{HOME}` as a word, so any such word beside a redirect counts.
fn descriptor_verdict(cmd: &SimpleCmd) -> Verdict {
    let braced_name = |w: &Word| {
        let text = w.eval();
        text.strip_prefix('{')
            .and_then(|t| t.strip_suffix('}'))
            .is_some_and(|name| !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
    };
    refuse_if(!cmd.redirs.is_empty() && cmd.words.iter().any(braced_name))
}

/// A name a command sets or tests: `read 'a[HOME=1]'` and `test -v 'a[HOME=1]'` evaluate the
/// subscript. A name held in a variable or a command's output could carry any subscript.
fn name_verdict(word: &Word) -> Verdict {
    let text = word.eval();
    let (read, v) = read_text(word);
    v.combine(refuse_if(text.contains('$') || text.contains("__SAFE_CHAINS_")))
        .combine(subscript_verdict(&text))
        .combine(subscript_verdict(&read))
}

fn subscript_verdict(text: &str) -> Verdict {
    let name = text.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(text.len());
    text[name..]
        .strip_prefix('[')
        .filter(|_| name > 0)
        .map_or(INERT, |s| expression_verdict(bracketed(s, '[', ']')))
}

/// The operands of `-v`, which names a variable to test.
fn tested_names(args: &[String]) -> Vec<usize> {
    args.iter()
        .enumerate()
        .filter(|(_, w)| *w == "-v")
        .map(|(i, _)| i + 1)
        .filter(|&i| i < args.len())
        .collect()
}

/// The arguments a format string reads as numbers: every one after the format, unless the format
/// is known to hold no numeric conversion. `takes_value` names the option whose value comes next.
fn formatted(args: &[String], takes_value: fn(&str) -> bool) -> Vec<usize> {
    let mut i = 0;
    while let Some(arg) = args.get(i).filter(|w| w.starts_with('-') && w.len() > 1) {
        i += if takes_value(arg) { 2 } else { 1 };
        if arg == "--" {
            break;
        }
    }
    let Some(format) = args.get(i) else { return Vec::new() };
    let slots = numeric_slots(format);
    if !slots.contains(&true) {
        return Vec::new();
    }
    // The format is reused until the arguments run out, so argument k fills slot k mod its length.
    (i + 1..args.len()).filter(|k| slots[(k - i - 1) % slots.len()]).collect()
}

/// Whether each argument the format takes is read as a number, in order: a `*` width or precision
/// takes one (`printf '%*s' 5 x`), then the conversion. A `%1$d` picks its argument by number, so
/// every one counts.
fn numeric_slots(format: &str) -> Vec<bool> {
    if format.contains('$') {
        return vec![true];
    }
    let mut slots = Vec::new();
    for spec in format.replace("%%", "").split('%').skip(1) {
        let conversion = spec.trim_start_matches(|c: char| "-+ #0123456789.*".contains(c));
        slots.extend(spec[..spec.len() - conversion.len()].matches('*').map(|_| true));
        slots.push(conversion.starts_with(|c: char| "diouxXeEfFgGaAc".contains(c)));
    }
    slots
}

/// The values of the options in `letters`: `-t 5` and `-t5` alike.
fn option_values(args: &[String], letters: &[char]) -> Vec<usize> {
    let mut out = Vec::new();
    for (i, arg) in args.iter().enumerate() {
        let Some(cluster) = arg.strip_prefix('-') else { continue };
        if let Some(at) = cluster.find(letters) {
            if at + 1 == cluster.len() {
                out.extend(Some(i + 1).filter(|&next| next < args.len()));
            } else {
                out.push(i);
            }
        }
    }
    out
}

/// `[[ a -eq b ]]` evaluates both sides as arithmetic, in bash and zsh alike, and `-v` names a
/// variable whose subscript is arithmetic.
pub(crate) fn double_bracket_verdict(words: &[Word]) -> Verdict {
    let text: Vec<String> = words.iter().map(Word::eval).collect();
    let mut v = super::super::check::words_sub_verdict(words);
    for (i, w) in text.iter().enumerate() {
        if matches!(w.as_str(), "-eq" | "-ne" | "-lt" | "-le" | "-gt" | "-ge") {
            let sides = i.checked_sub(1).and_then(|l| words.get(l)).into_iter().chain(words.get(i + 1));
            v = sides.map(arithmetic_word_verdict).fold(v, Verdict::combine);
        }
    }
    tested_names(&text).into_iter().map(|i| name_verdict(&words[i])).fold(v, Verdict::combine)
}

fn command_name(word: &str) -> String {
    Token::from_raw(word.to_string()).command_name().to_string()
}
