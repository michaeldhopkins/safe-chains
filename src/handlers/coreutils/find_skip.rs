//! The entries a `find` expression keeps from its actions, read only from shapes whose meaning is
//! certain.
//!
//! `find . -path ./.git -prune -o -exec chmod +x {} +` never enters `.git`, and
//! `find . -type f -not -path './.git/*' -exec sed -i … {} +` never hands a file below `.git` to
//! `sed`. Two shapes are read:
//!
//! - a prune prefix: the expression opens with `TEST -prune -o`, `TEST` being `-path`, `-name` or
//!   their `-i`/`-wholename` forms, optionally after `-type d`, or several of them inside `( … -o … )`.
//!   Entries the test matches are neither acted on nor entered. `-depth`, which `-delete` implies,
//!   turns `-prune` off, and then only the matched entries themselves are kept away.
//! - filters ANDed before the first action, read only when the rest of the expression has no `-o`,
//!   `,` or parentheses: `-not TEST` and `! TEST` keep matching entries away, `-type f` keeps folders
//!   away.
//!
//! Anything else keeps nothing away, so the traversal is judged as reaching everything below it.
//! Name filters that only let some names through (`-name '*.rs'`) are not read: the files inside a
//! repository's metadata have names nobody chose (a branch named `x.rs` is `.git/refs/heads/x.rs`),
//! so no name pattern is known not to match one.

use crate::engine::resolve::frozen_walk::{Skip, Test};

/// The tests a prune prefix keeps away, each with whether it holds only for folders (`-type d`).
type Pruned = Vec<(Test, bool)>;

const ACTIONS: [&str; 14] = [
    "-exec", "-execdir", "-ok", "-okdir", "-delete", "-print", "-print0", "-printf", "-fprint", "-fprint0", "-fprintf", "-ls", "-fls",
    "-quit",
];

/// The entries the find expression in `args` (the words after `find`) keeps from its actions.
pub(crate) fn skip_of(args: &[&str]) -> Skip {
    let expr = expression(args);
    let depth = args.iter().any(|a| matches!(*a, "-depth" | "-d" | "-delete" | "-mindepth"));
    let mut skip = Skip::default();
    let expr = past_options(expr);
    let rest = match prune_prefix(expr) {
        Some((tests, rest)) => {
            if depth {
                skip.excluded.extend(tests.into_iter().map(|(t, _)| t));
            } else {
                skip.pruned = tests;
            }
            rest
        }
        None => expr,
    };
    if !alternates(rest) {
        filters(rest, &mut skip);
    }
    skip
}

/// The expression: what follows the leading options and the path operands.
fn expression<'a, 'b>(args: &'b [&'a str]) -> &'b [&'a str] {
    let mut i = 0;
    while let Some(a) = args.get(i) {
        match *a {
            "-H" | "-L" | "-P" => i += 1,
            "-D" => i += 2,
            a if a.starts_with("-O") => i += 1,
            _ => break,
        }
    }
    let rest = args.get(i..).unwrap_or(&[]);
    let start = rest
        .iter()
        .position(|a| a.starts_with('-') || matches!(*a, "(" | "!" | ")" | ","))
        .unwrap_or(rest.len());
    &rest[start..]
}

/// The expression past the options written at its start (`-depth`, `-xdev`, `-maxdepth 2`), which
/// apply to the whole walk and are not tests.
fn past_options<'a, 'b>(mut expr: &'b [&'a str]) -> &'b [&'a str] {
    loop {
        expr = match expr.first() {
            Some(&("-depth" | "-d" | "-xdev" | "-mount" | "-noleaf" | "-follow" | "-ignore_readdir_race")) => &expr[1..],
            Some(&("-maxdepth" | "-mindepth")) if expr.len() > 1 => &expr[2..],
            _ => return expr,
        };
    }
}

/// A single test and its pattern at the start of `words`: `(test, consumed)`.
fn test_at(words: &[&str]) -> Option<(Test, usize)> {
    let (primary, pattern) = (*words.first()?, (*words.get(1)?).to_string());
    let test = match primary {
        "-path" | "-wholename" => Test::Path(pattern, false),
        "-ipath" | "-iwholename" => Test::Path(pattern, true),
        "-name" => Test::Name(pattern, false),
        "-iname" => Test::Name(pattern, true),
        _ => return None,
    };
    Some((test, 2))
}

/// A test optionally preceded by `-type d`: `((test, dir_only), consumed)`.
fn alternative(words: &[&str]) -> Option<((Test, bool), usize)> {
    let dir_only = words.len() >= 2 && words[0] == "-type" && words[1] == "d";
    let at = if dir_only { 2 } else { 0 };
    let (test, used) = test_at(&words[at..])?;
    Some(((test, dir_only), at + used))
}

/// `TEST -prune -o REST` or `( TEST -o TEST … ) -prune -o REST`: the tests and `REST`.
fn prune_prefix<'a, 'b>(expr: &'b [&'a str]) -> Option<(Pruned, &'b [&'a str])> {
    let mut tests = Vec::new();
    let i = if expr.first() == Some(&"(") {
        let mut i = 1;
        loop {
            let (alt, used) = alternative(&expr[i..])?;
            tests.push(alt);
            i += used;
            match expr.get(i) {
                Some(&")") => break,
                Some(&("-o" | "-or")) => i += 1,
                _ => return None,
            }
        }
        i + 1
    } else {
        let (alt, used) = alternative(expr)?;
        tests.push(alt);
        used
    };
    match (expr.get(i), expr.get(i + 1)) {
        (Some(&"-prune"), Some(&("-o" | "-or"))) => Some((tests, &expr[i + 2..])),
        _ => None,
    }
}

/// Whether `expr` has an alternative, a list or a group outside an action's own command.
fn alternates(expr: &[&str]) -> bool {
    let mut words = expr.iter();
    while let Some(w) = words.next() {
        if matches!(*w, "-exec" | "-execdir" | "-ok" | "-okdir") {
            words.by_ref().find(|w| matches!(**w, ";" | "+"));
            continue;
        }
        if matches!(*w, "-o" | "-or" | "," | "(" | ")") {
            return true;
        }
    }
    false
}

/// The words one primary takes at `words[0]`: two for a primary with a value, else one.
fn width(words: &[&str]) -> usize {
    let valued = crate::registry::fallback_flag_sets("find").map_or(&[][..], |(_, valued)| valued);
    let w = words.first().copied().unwrap_or("");
    if w.starts_with("-newer") || valued.iter().any(|v| v == w) { 2 } else { 1 }
}

/// The filters ANDed before the first action. A negated primary that is not a test is stepped
/// over whole, so `! -type f` keeps nothing away.
fn filters(expr: &[&str], skip: &mut Skip) {
    let before = &expr[..expr.iter().position(|w| ACTIONS.contains(w)).unwrap_or(expr.len())];
    let mut i = 0;
    while i < before.len() {
        let here = &before[i..];
        i += match here[0] {
            "!" | "-not" => match test_at(&here[1..]) {
                Some((test, used)) => {
                    skip.excluded.push(test);
                    1 + used
                }
                None => 1 + width(&here[1..]),
            },
            "-type" if here.get(1) == Some(&"f") => {
                skip.files_only = true;
                2
            }
            _ => width(here),
        };
    }
}

#[cfg(test)]
#[path = "find_skip_tests.rs"]
mod tests;
