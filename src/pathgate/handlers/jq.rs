//! What a jq-family filter reads besides the files it is given.
//!
//! Every operand of `jq`, `gojq`, `jaq`, `xq`, `tomlq` and `yq` is a content read, and the read
//! walk gates each one. The filter can also read files that no operand names, and those reads are
//! the ones this gate exists for:
//!
//! - **A program file** (`-f`/`--from-file`). The filter is read from a file this classifier
//!   cannot see, so nothing below can be checked. It is refused wherever it points.
//! - **Modules** (`import`, `include`, and `modulemeta`, which loads a module to describe it).
//!   `import "config" as $c;` loads `config.json` as data, from `~/.jq`, `$ORIGIN/../lib/jq`,
//!   `$ORIGIN/../lib`, any `-L`/`--library-path` directory, or a directory the directive's own
//!   metadata names (`{search: "/any/dir"}`). A module can import further modules, so loading
//!   code is as open as loading data. Any of the three words, and any module-path flag, refuses.
//! - **yq's load operators** (`load`, `load_str`/`strload`, `load_xml`/`xml_load`, `load_props`,
//!   `load_base64`, spelled with or without the underscore) read any file the expression names,
//!   and `eval` turns a string into an expression, which can be one of them. `yq` is checked as
//!   jq too: the name also belongs to the Python `yq`, which passes its arguments to jq.
//!
//! What a filter may still do, with the reasons:
//!
//! - `--rawfile NAME FILE` and `--slurpfile NAME FILE` read FILE, which the read walk gates as an
//!   operand, so a credential file there is refused like `jq . FILE`.
//! - `env`, `$ENV` and `getpath` over them read the process environment, which `printenv`
//!   already shows and safe-chains already approves.
//! - `input_filename`, `$__loc__` and `@sh` name the current file, the program's position, and
//!   quote a string for a shell. None of them reads or runs anything.
//!
//! The filter is the first operand once each option has taken its values (yq: or the value of
//! `--expression`, and the operand after an `eval` subcommand). Only that word is read as a
//! program; the option words are all checked for the flags above. The arities are each tool's own
//! and a test holds them to the grammars, since a wrong count would read a file name as the filter
//! and leave the real filter unread.
//!
//! The words are matched without a jq parser, conservatively: any spelling that could be the
//! keyword refuses, and only positions where the keyword cannot be one are let through. A
//! directive is only valid at the start of a program or after the `;` ending the one before it,
//! so it is never directly preceded by a letter, digit or underscore (then it is part of a longer
//! name), `.` (a field, `.include`), `$` (a variable), `/` or `-` (an operator, or a path such as
//! `./include/a.json`), or `"` (the end of a string). Anything else before it, including
//! whitespace, `;`, the start of the word, a newline ending a comment, or a byte jq might treat as
//! whitespace, counts. Text inside strings and comments is scanned as well, which can only add a
//! refusal.

use crate::parse::Token;
use crate::pathgate::{Role, RoleSpec, Shape, walk};

const DIRECTIVES: &[&str] = &["import", "include"];
const MODULE_FNS: &[&str] = &["modulemeta"];
const YQ_LOADERS: &[&str] = &["load", "eval"];

/// yq's subcommands are whole words; `eval` there is the subcommand, not the operator.
const YQ_SUBCOMMANDS: &[&str] = &["e", "eval", "ea", "eval-all"];

/// How many following words an option takes, per tool. Any option not listed takes none.
pub(super) struct Arities {
    pub(super) two: &'static [&'static str],
    pub(super) one: &'static [&'static str],
    /// Options whose value is itself a filter.
    pub(super) program: &'static [&'static str],
}

/// jq, gojq and jaq: `--arg NAME VALUE` and its kin take two words.
pub(super) const JQ: Arities = Arities {
    two: &["--arg", "--argjson", "--rawfile", "--slurpfile"],
    one: &["--indent", "--library-path", "--from-file", "-L", "-f"],
    program: &[],
};

/// The Python `yq` (`xq`, `tomlq`): jq's options, plus its own, which it removes before calling jq.
pub(super) const PY_YQ: Arities = Arities {
    two: JQ.two,
    one: &["--indent", "--library-path", "--from-file", "-L", "-f", "--width", "-w", "--xml-item-depth", "--xml-root", "--xml-force-list"],
    program: &[],
};

/// mikefarah's `yq`, and the Python one under the same name, which fails on what it does not take.
pub(super) const YQ: Arities = Arities {
    two: JQ.two,
    one: &[
        "--indent", "--front-matter", "--input-format", "--output-format", "--from-file", "-I", "-p", "-o", "-f", "-L", "--library-path",
    ],
    program: &["--expression"],
};

/// `[roles."jq"]`, `gojq`, `jaq`, `xq`, `tomlq`: the filter's own reads, then every operand.
pub(super) fn jq_program(tokens: &[Token]) -> bool {
    let name = tokens.first().map(|t| t.command_name().to_ascii_lowercase()).unwrap_or_default();
    let arities = if matches!(name.as_str(), "xq" | "tomlq") { &PY_YQ } else { &JQ };
    let args = words(tokens);
    jq_reads_beyond_operands(&args, &filter_words(&args, arities, false)) || read_operands(tokens)
}

/// `[roles."yq"]`: yq's load operators, jq's reads (the Python `yq` is jq), then every operand.
pub(super) fn yq_program(tokens: &[Token]) -> bool {
    let args = words(tokens);
    let filters = filter_words(&args, &YQ, true);
    yq_reads_beyond_operands(&args, &filters) || jq_reads_beyond_operands(&args, &filters) || read_operands(tokens)
}

fn words(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().skip(1).map(Token::as_str).collect()
}

fn read_operands(tokens: &[Token]) -> bool {
    walk(&RoleSpec::simple(Role::Read, Shape::Plain), tokens)
}

/// The words a tool reads as its filter: the first operand once every option has taken its
/// values, the value of an option that is a filter wherever it stands (options may follow
/// operands, and yq then reads every operand as a file), and with `subcommands` (yq) the operand
/// after `eval` as well.
pub(super) fn filter_words<'a>(args: &[&'a str], arities: &Arities, subcommands: bool) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut operands = Vec::new();
    let mut i = 0;
    let mut options_end = false;
    while i < args.len() {
        let w = args[i];
        i += 1;
        if !options_end && w == "--" {
            options_end = true;
        } else if !options_end && w.len() > 1 && w.starts_with('-') {
            out.extend(program_value(w, args.get(i).copied(), arities));
            i += option_values(w, arities);
        } else {
            operands.push(w);
        }
    }
    let leading = usize::from(subcommands && operands.first().is_some_and(|w| YQ_SUBCOMMANDS.contains(w)));
    out.extend(operands.into_iter().take(leading + 1));
    out
}

/// The filter an option carries: the next word, or what follows `=`.
fn program_value<'a>(word: &'a str, next: Option<&'a str>, arities: &Arities) -> Option<&'a str> {
    match word.split_once('=') {
        Some((name, value)) => arities.program.contains(&name).then_some(value),
        None => next.filter(|_| arities.program.contains(&word)),
    }
}

/// How many following words `word` takes. A long option with `=value` takes one fewer; in a short
/// cluster the first letter that takes a value takes the rest of the cluster, or the next word
/// when it is the last letter, as getopt reads it.
fn option_values(word: &str, arities: &Arities) -> usize {
    let listed = |w: &str| {
        if arities.two.contains(&w) { 2 } else { usize::from(arities.one.contains(&w) || arities.program.contains(&w)) }
    };
    if word.starts_with("--") {
        return match word.split_once('=') {
            Some((name, _)) => listed(name).saturating_sub(1),
            None => listed(word),
        };
    }
    let letters: Vec<char> = word[1..].chars().collect();
    match letters.iter().position(|c| listed(&format!("-{c}")) > 0) {
        Some(at) if at + 1 == letters.len() => listed(&format!("-{}", letters[at])),
        _ => 0,
    }
}

/// Whether a jq invocation lets the filter read a file no operand names: a program file or a
/// module path among `args`, or a module directive in any of `filters`.
pub(crate) fn jq_reads_beyond_operands(args: &[&str], filters: &[&str]) -> bool {
    args.iter().any(|w| names_program_file(w) || names_module_path(w)) || filters.iter().any(|w| loads_module(w))
}

/// Whether a yq invocation reads its expression from a file or uses an operator that reads a file
/// or evaluates a string.
pub(crate) fn yq_reads_beyond_operands(args: &[&str], filters: &[&str]) -> bool {
    args.iter().any(|w| w.starts_with("--from-file")) || filters.iter().any(|w| !YQ_SUBCOMMANDS.contains(w) && yq_loads(w))
}

/// `-f`, `--from-file`, `--from-file=…`, and `f` inside a cluster of short flags (`-nf`, and
/// `-fprog.jq`, gojq's and jaq's attached value).
fn names_program_file(word: &str) -> bool {
    word.starts_with("--from-file") || short_cluster_has(word, 'f')
}

/// `-L`, `-Ldir`, `--library-path`, `--library-path=…`, and `L` inside a short-flag cluster.
fn names_module_path(word: &str) -> bool {
    word.starts_with("--library-path") || short_cluster_has(word, 'L')
}

fn short_cluster_has(word: &str, flag: char) -> bool {
    word.strip_prefix('-').is_some_and(|rest| !rest.starts_with('-') && rest.contains(flag))
}

/// Whether `word` holds `import`, `include` or `modulemeta` anywhere jq could read it as one.
fn loads_module(word: &str) -> bool {
    if crate::parse::has_unknown_content(word) {
        return true;
    }
    let lower = word.to_ascii_lowercase();
    DIRECTIVES
        .iter()
        .any(|kw| occurrences(&lower, kw).any(|at| directive_could_start(&lower, at, kw.len())))
        || MODULE_FNS.iter().any(|f| occurrences(&lower, f).any(|at| name_could_start(&lower, at, f.len())))
}

/// Whether `word` holds a yq load operator or `eval` anywhere outside a field or variable name.
/// Any name CONTAINING one counts (`strload`, `xml_load`, `loadxml`), as yq's lexer matches those
/// spellings.
fn yq_loads(word: &str) -> bool {
    if crate::parse::has_unknown_content(word) {
        return true;
    }
    let lower = word.to_ascii_lowercase();
    YQ_LOADERS
        .iter()
        .any(|op| occurrences(&lower, op).any(|at| !is_field_or_variable(&lower, name_start(&lower, at))))
}

fn occurrences<'a>(hay: &'a str, needle: &'a str) -> impl Iterator<Item = usize> + 'a {
    hay.match_indices(needle).map(|(at, _)| at)
}

fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// A directive keyword at `at`: not continued into a longer name, and not preceded by a byte that
/// rules out the directive position (see the module comment).
fn directive_could_start(s: &str, at: usize, len: usize) -> bool {
    let bytes = s.as_bytes();
    if bytes.get(at + len).copied().is_some_and(is_name_byte) {
        return false;
    }
    match at.checked_sub(1).map(|i| bytes[i]) {
        None => true,
        Some(b) if b.is_ascii_alphabetic() || b == b'_' => false,
        Some(b'$' | b'/' | b'-' | b'"') => false,
        Some(b'.') => !is_single_dot_before(bytes, at),
        Some(_) => true,
    }
}

/// A function name at `at`: a whole name, and not a field (`.modulemeta`) or a variable. Unlike a
/// directive it can follow any operator, so only those two exemptions hold.
fn name_could_start(s: &str, at: usize, len: usize) -> bool {
    let bytes = s.as_bytes();
    if bytes.get(at + len).copied().is_some_and(is_name_byte) {
        return false;
    }
    match at.checked_sub(1).map(|i| bytes[i]) {
        Some(b) if b.is_ascii_alphabetic() || b == b'_' => false,
        _ => !is_field_or_variable(s, at),
    }
}

/// Where the name holding the byte at `at` begins.
fn name_start(s: &str, at: usize) -> usize {
    let bytes = s.as_bytes();
    let mut start = at;
    while start > 0 && is_name_byte(bytes[start - 1]) {
        start -= 1;
    }
    start
}

/// Whether the name starting at `start` follows exactly one `.` (a field) or a `$` (a variable).
/// `..name` is recursion followed by a bare name, so it is not a field.
fn is_field_or_variable(s: &str, start: usize) -> bool {
    let bytes = s.as_bytes();
    match start.checked_sub(1).map(|i| bytes[i]) {
        Some(b'$') => true,
        Some(b'.') => is_single_dot_before(bytes, start),
        _ => false,
    }
}

fn is_single_dot_before(bytes: &[u8], at: usize) -> bool {
    at >= 1 && bytes[at - 1] == b'.' && (at < 2 || bytes[at - 2] != b'.')
}

#[cfg(test)]
#[path = "jq_tests.rs"]
mod tests;
