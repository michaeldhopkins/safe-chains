//! The commands that read an argument as arithmetic, or that set or test a variable by a name that
//! may carry a subscript.

use super::{
    INERT, NUMERIC, arithmetic_word_verdict, assigned_verdict, bracketed, expression_verdict, is_free, is_integer, is_numeric, read_text,
    refuse_if, value_verdict,
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
            .map(|w| w.split_once('=').unwrap_or((w, "")))
            .map(|(n, value)| {
                let refused = is_free(n) || !is_integer(value);
                NUMERIC.with(|numeric| numeric.borrow_mut().push(n.to_string()));
                refused
            })
            .collect();
        return env.combine(refuse_if(refused.contains(&true)));
    }
    let set = match super::super::opaque::named(cmd) {
        Some(names) => names.iter().map(|n| value_verdict(n, "*")).fold(INERT, Verdict::combine),
        // Which names are set is not known: any numeric one could take an expression.
        None => {
            let names_numeric = args.iter().any(|w| is_numeric(w.split('=').next().unwrap_or_default()));
            value_verdict("*", "*").combine(refuse_if(names_numeric || NUMERIC.with(|n| !n.borrow().is_empty())))
        }
    };
    env.combine(set)
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
    match args.get(i) {
        Some(format) if numeric_conversion(format) => (i + 1..args.len()).collect(),
        _ => Vec::new(),
    }
}

/// A `*` width or precision reads its number from the arguments too: `printf '%*s' 5 x`.
fn numeric_conversion(format: &str) -> bool {
    format.contains('$')
        || format.split('%').skip(1).any(|spec| {
            let conversion = spec.trim_start_matches(|c: char| "-+ #0123456789.*".contains(c));
            spec[..spec.len() - conversion.len()].contains('*') || conversion.starts_with(|c: char| "diouxXeEfFgGaAc".contains(c))
        })
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
